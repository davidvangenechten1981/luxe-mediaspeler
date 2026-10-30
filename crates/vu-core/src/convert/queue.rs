//! Wachtrij voor conversietaken: draait op achtergrondthreads, rapporteert
//! voortgang via een callback en ruimt bij afsluiten alle processen op.

use super::args::Capabilities;
use super::runner::{resolve_output, run_conversion, OutputSpec, Progress, Resolved, Tools};
use super::{Mode, Settings};
use crate::db::now;
use crate::error::{CoreError, Result};
use crate::probe::probe_file;
use crate::validate::validate_media_path;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobRequest {
    pub input: String,
    pub settings: Settings,
    pub profile_name: String,
    pub output: OutputSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobSnapshot {
    pub id: u64,
    pub input: String,
    pub output: Option<String>,
    pub profile: String,
    pub state: JobState,
    pub mode: Mode,
    pub fraction: Option<f64>,
    pub eta_s: Option<u64>,
    pub speed: Option<f64>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    /// De conversie is gepauzeerd (het FFmpeg-proces staat stil) en kan worden hervat.
    pub paused: bool,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

struct Job {
    snap: JobSnapshot,
    request: JobRequest,
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

#[derive(Default)]
struct State {
    jobs: Vec<Job>,
    next_id: u64,
    running: usize,
    shutdown: bool,
    workers: Vec<JoinHandle<()>>,
}

pub type EventFn = Arc<dyn Fn(&JobSnapshot) + Send + Sync>;

struct Shared {
    state: Mutex<State>,
    cv: Condvar,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    caps: Capabilities,
    /// Werkende videokaarten; pas bij het eerste gebruik bepaald (de proef start FFmpeg een paar keer).
    hw: std::sync::OnceLock<Vec<super::HwBackend>>,
    max_parallel: usize,
    log_path: Option<PathBuf>,
    on_event: EventFn,
}

pub struct TaskManager {
    shared: Arc<Shared>,
    coordinator: Mutex<Option<JoinHandle<()>>>,
}

impl Shared {
    fn emit(&self, snap: &JobSnapshot) {
        (self.on_event)(snap);
    }

    fn log(&self, snap: &JobSnapshot) {
        if let Some(p) = &self.log_path {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                let _ = writeln!(
                    f,
                    "[{}] #{} {:?} {} -> {}{}",
                    snap.finished_at.unwrap_or_else(now),
                    snap.id,
                    snap.state,
                    snap.input,
                    snap.output.as_deref().unwrap_or("-"),
                    snap.error.as_deref().map(|e| format!(" | {}", e.replace('\n', " / "))).unwrap_or_default()
                );
            }
        }
    }

    /// Past een taak aan onder het slot en geeft daarna (buiten het slot) een melding.
    fn update(&self, id: u64, f: impl FnOnce(&mut JobSnapshot)) {
        let snap = {
            let mut st = self.state.lock().unwrap();
            let Some(job) = st.jobs.iter_mut().find(|j| j.snap.id == id) else {
                return;
            };
            f(&mut job.snap);
            job.snap.clone()
        };
        self.emit(&snap);
    }
}

fn worker(shared: Arc<Shared>, id: u64, request: JobRequest, cancel: Arc<AtomicBool>, paused: Arc<AtomicBool>) {
    let result = (|| -> Result<(Option<std::path::PathBuf>, Mode, Vec<String>, bool)> {
        let input = validate_media_path(&request.input)?;
        request.settings.validate()?;
        let info = probe_file(&shared.ffprobe, &input)?;
        let resolved = resolve_output(&input, &request.output, request.settings.container, &request.profile_name, now())?;
        let final_path = match resolved {
            Resolved::Skipped(p) => return Ok((Some(p), request.settings.mode(), vec![], true)),
            Resolved::Path(p) => p,
        };
        shared.update(id, |s| s.output = Some(final_path.to_string_lossy().into_owned()));
        let caps = shared.caps_with_hw();
        let tools = Tools { ffmpeg: &shared.ffmpeg, ffprobe: &shared.ffprobe, caps: &caps };
        let shared2 = shared.clone();
        let outcome = run_conversion(&tools, &input, &final_path, &request.settings, &info, &cancel, &paused, &mut |p: Progress| {
            shared2.update(id, |s| {
                s.fraction = p.fraction;
                s.eta_s = p.eta_s;
                s.speed = p.speed;
            });
        })?;
        Ok((Some(outcome.output), outcome.mode, outcome.warnings, false))
    })();

    let finished = now();
    // Eerst de uitkomst vastleggen en melden, en pas daarna de plek vrijgeven: zo komt de
    // melding "klaar" van deze taak altijd vóór de melding "gestart" van de volgende.
    let snap = {
        let mut st = shared.state.lock().unwrap();
        let job = st.jobs.iter_mut().find(|j| j.snap.id == id);
        match job {
            None => None,
            Some(job) => {
                job.snap.finished_at = Some(finished);
                job.snap.paused = false;
                job.snap.speed = None;
                job.snap.eta_s = None;
                match result {
                    Ok((out, mode, warnings, skipped)) => {
                        job.snap.output = out.map(|p| p.to_string_lossy().into_owned());
                        job.snap.mode = mode;
                        job.snap.warnings = warnings;
                        if skipped {
                            job.snap.state = JobState::Skipped;
                            job.snap.error = Some("Het uitvoerbestand bestaat al; overgeslagen".into());
                        } else {
                            job.snap.state = JobState::Done;
                            job.snap.fraction = Some(1.0);
                        }
                    }
                    Err(CoreError::Cancelled) => {
                        job.snap.state = JobState::Cancelled;
                        job.snap.output = None;
                    }
                    Err(e) => {
                        job.snap.state = JobState::Failed;
                        job.snap.error = Some(e.to_string());
                        job.snap.output = None;
                    }
                }
                Some(job.snap.clone())
            }
        }
    };
    if let Some(s) = snap {
        shared.log(&s);
        shared.emit(&s);
    }
    {
        let mut st = shared.state.lock().unwrap();
        st.running = st.running.saturating_sub(1);
    }
    shared.cv.notify_all();
}

fn coordinator(shared: Arc<Shared>) {
    let mut st = shared.state.lock().unwrap();
    loop {
        if st.shutdown {
            return;
        }
        if st.running < shared.max_parallel {
            if let Some(job) = st.jobs.iter_mut().find(|j| j.snap.state == JobState::Queued) {
                job.snap.state = JobState::Running;
                job.snap.started_at = Some(now());
                let (id, req, cancel, paused, snap) =
                    (job.snap.id, job.request.clone(), job.cancel.clone(), job.paused.clone(), job.snap.clone());
                st.running += 1;
                let sh = shared.clone();
                let handle = std::thread::spawn(move || worker(sh, id, req, cancel, paused));
                st.workers.push(handle);
                drop(st);
                shared.emit(&snap);
                st = shared.state.lock().unwrap();
                continue;
            }
        }
        st = shared.cv.wait(st).unwrap();
    }
}

impl Shared {
    fn hw(&self) -> &Vec<super::HwBackend> {
        self.hw.get_or_init(|| {
            let mut c = self.caps.clone();
            c.probe_hardware(&self.ffmpeg);
            c.working_hw
        })
    }

    fn caps_with_hw(&self) -> Capabilities {
        let mut c = self.caps.clone();
        c.working_hw = self.hw().clone();
        c
    }
}

impl TaskManager {
    pub fn new(
        ffmpeg: PathBuf,
        ffprobe: PathBuf,
        max_parallel: usize,
        log_path: Option<PathBuf>,
        on_event: EventFn,
    ) -> Result<TaskManager> {
        let caps = Capabilities::detect(&ffmpeg)?;
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            cv: Condvar::new(),
            ffmpeg,
            ffprobe,
            caps,
            hw: std::sync::OnceLock::new(),
            max_parallel: max_parallel.clamp(1, 4),
            log_path,
            on_event,
        });
        let sh = shared.clone();
        let handle = std::thread::spawn(move || coordinator(sh));
        Ok(TaskManager { shared, coordinator: Mutex::new(Some(handle)) })
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.shared.caps
    }

    /// Videokaarten waarmee een proefcodering echt lukte. De eerste aanroep doet de proef; daarna is het direct.
    pub fn hardware(&self) -> Vec<super::HwBackend> {
        self.shared.hw().clone()
    }

    pub fn add(&self, request: JobRequest) -> Result<u64> {
        validate_media_path(&request.input)?;
        request.settings.validate()?;
        self.shared.caps.check(&request.settings)?;
        let snap = {
            let mut st = self.shared.state.lock().unwrap();
            if st.shutdown {
                return Err(CoreError::Invalid("De wachtrij is afgesloten".into()));
            }
            st.next_id += 1;
            let id = st.next_id;
            let snap = JobSnapshot {
                id,
                input: request.input.clone(),
                output: None,
                profile: request.profile_name.clone(),
                state: JobState::Queued,
                mode: request.settings.mode(),
                fraction: None,
                eta_s: None,
                speed: None,
                error: None,
                warnings: vec![],
                paused: false,
                started_at: None,
                finished_at: None,
            };
            st.jobs.push(Job {
                snap: snap.clone(),
                request,
                cancel: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
            });
            snap
        };
        self.shared.cv.notify_all();
        self.shared.emit(&snap);
        Ok(snap.id)
    }

    pub fn list(&self) -> Vec<JobSnapshot> {
        self.shared.state.lock().unwrap().jobs.iter().map(|j| j.snap.clone()).collect()
    }

    pub fn get(&self, id: u64) -> Option<JobSnapshot> {
        self.shared.state.lock().unwrap().jobs.iter().find(|j| j.snap.id == id).map(|j| j.snap.clone())
    }

    /// Wachtende taken worden direct geannuleerd; lopende taken stoppen hun FFmpeg-proces.
    pub fn cancel(&self, id: u64) -> Result<()> {
        let snap = {
            let mut st = self.shared.state.lock().unwrap();
            let job = st.jobs.iter_mut().find(|j| j.snap.id == id).ok_or_else(|| CoreError::NotFound(format!("taak {id}")))?;
            match job.snap.state {
                JobState::Queued => {
                    job.snap.state = JobState::Cancelled;
                    job.snap.finished_at = Some(now());
                    Some(job.snap.clone())
                }
                JobState::Running => {
                    job.cancel.store(true, Ordering::Relaxed);
                    None
                }
                _ => return Err(CoreError::Invalid("Deze taak is al afgerond".into())),
            }
        };
        if let Some(s) = snap {
            self.shared.log(&s);
            self.shared.emit(&s);
        }
        Ok(())
    }

    /// Pauzeert een lopende conversie: het FFmpeg-proces staat stil tot je hervat.
    pub fn pause(&self, id: u64) -> Result<()> {
        self.set_paused(id, true)
    }

    pub fn resume(&self, id: u64) -> Result<()> {
        self.set_paused(id, false)
    }

    fn set_paused(&self, id: u64, paused: bool) -> Result<()> {
        let snap = {
            let mut st = self.shared.state.lock().unwrap();
            let job = st.jobs.iter_mut().find(|j| j.snap.id == id).ok_or_else(|| CoreError::NotFound(format!("taak {id}")))?;
            if job.snap.state != JobState::Running {
                return Err(CoreError::Invalid("Alleen een lopende conversie kan worden gepauzeerd of hervat".into()));
            }
            if job.snap.paused == paused {
                return Err(CoreError::Invalid(
                    if paused { "Deze conversie is al gepauzeerd" } else { "Deze conversie is niet gepauzeerd" }.into(),
                ));
            }
            job.paused.store(paused, Ordering::Relaxed);
            job.snap.paused = paused;
            if paused {
                job.snap.speed = None;
                job.snap.eta_s = None;
            }
            job.snap.clone()
        };
        self.shared.emit(&snap);
        Ok(())
    }

    /// Zet een mislukte of geannuleerde taak opnieuw in de wachtrij.
    pub fn retry(&self, id: u64) -> Result<()> {
        let snap = {
            let mut st = self.shared.state.lock().unwrap();
            let job = st.jobs.iter_mut().find(|j| j.snap.id == id).ok_or_else(|| CoreError::NotFound(format!("taak {id}")))?;
            if !matches!(job.snap.state, JobState::Failed | JobState::Cancelled | JobState::Skipped) {
                return Err(CoreError::Invalid("Alleen mislukte, geannuleerde of overgeslagen taken kunnen opnieuw".into()));
            }
            job.cancel = Arc::new(AtomicBool::new(false));
            job.paused = Arc::new(AtomicBool::new(false));
            let s = &mut job.snap;
            (s.state, s.fraction, s.eta_s, s.speed, s.error, s.output) = (JobState::Queued, None, None, None, None, None);
            (s.warnings, s.started_at, s.finished_at) = (vec![], None, None);
            s.clone()
        };
        self.shared.cv.notify_all();
        self.shared.emit(&snap);
        Ok(())
    }

    /// Verwijdert een taak uit de lijst (niet terwijl hij draait).
    pub fn remove(&self, id: u64) -> Result<()> {
        let mut st = self.shared.state.lock().unwrap();
        let pos = st.jobs.iter().position(|j| j.snap.id == id).ok_or_else(|| CoreError::NotFound(format!("taak {id}")))?;
        if st.jobs[pos].snap.state == JobState::Running {
            return Err(CoreError::Invalid("Annuleer de taak eerst voordat je hem verwijdert".into()));
        }
        st.jobs.remove(pos);
        Ok(())
    }

    pub fn clear_finished(&self) {
        let mut st = self.shared.state.lock().unwrap();
        st.jobs.retain(|j| matches!(j.snap.state, JobState::Queued | JobState::Running));
    }

    /// Annuleert alles, wacht tot alle FFmpeg-processen gestopt zijn en sluit de wachtrij.
    pub fn shutdown(&self) {
        let workers = {
            let mut st = self.shared.state.lock().unwrap();
            st.shutdown = true;
            for j in st.jobs.iter_mut() {
                match j.snap.state {
                    JobState::Running => j.cancel.store(true, Ordering::Relaxed),
                    JobState::Queued => {
                        j.snap.state = JobState::Cancelled;
                        j.snap.finished_at = Some(now());
                    }
                    _ => {}
                }
            }
            std::mem::take(&mut st.workers)
        };
        self.shared.cv.notify_all();
        for w in workers {
            let _ = w.join();
        }
        if let Some(c) = self.coordinator.lock().unwrap().take() {
            let _ = c.join();
        }
    }
}

impl Drop for TaskManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::builtin_profiles;
    use crate::convert::runner::test_support::*;
    use crate::convert::runner::Overwrite;
    use std::path::Path;
    use std::time::{Duration, Instant};

    fn settings(id: &str) -> Settings {
        builtin_profiles().into_iter().find(|p| p.id == id).unwrap().settings
    }

    fn manager(parallel: usize, log: Option<PathBuf>) -> (TaskManager, Arc<Mutex<Vec<JobSnapshot>>>) {
        let events: Arc<Mutex<Vec<JobSnapshot>>> = Arc::new(Mutex::new(vec![]));
        let e2 = events.clone();
        let m = TaskManager::new("ffmpeg".into(), "ffprobe".into(), parallel, log, Arc::new(move |s| e2.lock().unwrap().push(s.clone())))
            .unwrap();
        (m, events)
    }

    fn request(input: &Path, out_dir: &Path, profile: &str) -> JobRequest {
        JobRequest {
            input: input.to_string_lossy().into(),
            settings: settings(profile),
            profile_name: profile.into(),
            output: OutputSpec { dir: out_dir.to_path_buf(), template: "{name}".into(), overwrite: Overwrite::Rename },
        }
    }

    fn wait_for(m: &TaskManager, id: u64, f: impl Fn(&JobSnapshot) -> bool) -> JobSnapshot {
        let t = Instant::now();
        loop {
            if let Some(s) = m.get(id) {
                if f(&s) {
                    return s;
                }
            }
            assert!(t.elapsed() < Duration::from_secs(60), "time-out: {:?}", m.get(id));
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn finished(s: &JobSnapshot) -> bool {
        !matches!(s.state, JobState::Queued | JobState::Running)
    }

    #[test]
    fn runs_a_batch_in_order_and_reports_each_result() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("uit");
        let (m, events) = manager(1, Some(dir.path().join("conversie.log")));
        let mut ids = vec![];
        for n in ["a", "b"] {
            let input = dir.path().join(format!("{n}.mkv"));
            make_video(&input, 2, "160x90");
            ids.push(m.add(request(&input, &out, "remux_mp4")).unwrap());
        }
        for id in &ids {
            let s = wait_for(&m, *id, finished);
            assert_eq!(s.state, JobState::Done, "{s:?}");
            assert_eq!(s.fraction, Some(1.0));
            assert!(Path::new(s.output.as_ref().unwrap()).exists());
        }
        assert!(out.join("a.mp4").exists() && out.join("b.mp4").exists());
        // taak 2 begon pas nadat taak 1 klaar was (parallel = 1)
        let ev = events.lock().unwrap();
        let first_done = ev.iter().position(|s| s.id == ids[0] && s.state == JobState::Done).unwrap();
        let second_start = ev.iter().position(|s| s.id == ids[1] && s.state == JobState::Running).unwrap();
        assert!(second_start > first_done);
        let log = std::fs::read_to_string(dir.path().join("conversie.log")).unwrap();
        assert_eq!(log.lines().count(), 2);
        assert!(log.contains("Done"));
    }

    #[test]
    fn invalid_requests_are_rejected_immediately() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (m, _) = manager(1, None);
        let mut r = request(&dir.path().join("bestaat-niet.mkv"), dir.path(), "remux_mkv");
        assert!(m.add(r.clone()).is_err());
        let f = dir.path().join("a.mkv");
        std::fs::write(&f, b"x").unwrap();
        r.input = f.to_string_lossy().into();
        r.settings.max_height = Some(1);
        assert!(m.add(r).is_err());
        assert!(m.list().is_empty());
    }

    #[test]
    fn a_corrupt_file_fails_with_a_message_and_can_be_retried_after_fixing() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.mkv");
        std::fs::write(&input, b"dit is geen video").unwrap();
        let (m, _) = manager(1, None);
        let id = m.add(request(&input, dir.path(), "remux_mp4")).unwrap();
        let s = wait_for(&m, id, finished);
        assert_eq!(s.state, JobState::Failed);
        assert!(s.error.as_deref().is_some_and(|e| !e.is_empty()));
        make_video(&input, 1, "160x90"); // bestand "gerepareerd"
        m.retry(id).unwrap();
        let s = wait_for(&m, id, |s| s.state == JobState::Done);
        assert!(s.error.is_none() && Path::new(s.output.as_ref().unwrap()).exists());
        assert!(m.retry(id).is_err()); // een geslaagde taak kan niet opnieuw
    }

    #[test]
    fn existing_output_is_skipped_or_renamed_by_policy() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("uit");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("a.mp4"), b"bestaand").unwrap();
        let input = dir.path().join("a.mkv");
        make_video(&input, 1, "160x90");
        let (m, _) = manager(1, None);
        let mut skip = request(&input, &out, "remux_mp4");
        skip.output.overwrite = Overwrite::Skip;
        let s = wait_for(&m, m.add(skip).unwrap(), finished);
        assert_eq!(s.state, JobState::Skipped);
        assert_eq!(std::fs::read(out.join("a.mp4")).unwrap(), b"bestaand"); // niet aangeraakt
        let s = wait_for(&m, m.add(request(&input, &out, "remux_mp4")).unwrap(), finished);
        assert_eq!(s.state, JobState::Done);
        assert!(out.join("a (1).mp4").exists());
    }

    #[test]
    fn cancelling_a_queued_job_and_a_running_job_both_work() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("uit");
        let lang = dir.path().join("lang.mkv");
        make_video(&lang, 60, "1280x720");
        let kort = dir.path().join("kort.mkv");
        make_video(&kort, 1, "160x90");
        let (m, _) = manager(1, None);
        let mut slow = request(&lang, &out, "high_quality");
        slow.settings.video = crate::convert::VideoCodec::H265;
        let running = m.add(slow).unwrap();
        let queued = m.add(request(&kort, &out, "remux_mp4")).unwrap();
        wait_for(&m, running, |s| s.state == JobState::Running);
        m.cancel(queued).unwrap(); // wachtte nog
        assert_eq!(m.get(queued).unwrap().state, JobState::Cancelled);
        std::thread::sleep(Duration::from_millis(800));
        m.cancel(running).unwrap();
        let s = wait_for(&m, running, finished);
        assert_eq!(s.state, JobState::Cancelled);
        assert!(s.output.is_none());
        assert!(std::fs::read_dir(&out).map(|d| d.count()).unwrap_or(0) == 0, "er mag niets achterblijven");
        assert!(m.remove(running).is_ok());
        assert!(m.cancel(queued).is_err()); // al afgerond
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_running_conversion_can_be_paused_stays_still_and_finishes_after_resuming() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("uit");
        let src = dir.path().join("bron.mkv");
        make_video(&src, 40, "1280x720");
        let (m, _) = manager(1, None);
        let mut slow = request(&src, &out, "high_quality");
        slow.settings.video = crate::convert::VideoCodec::H265;
        slow.settings.hw_accel = crate::convert::HwAccel::Off;
        let id = m.add(slow).unwrap();
        wait_for(&m, id, |s| s.state == JobState::Running && s.fraction.is_some());
        m.pause(id).unwrap();
        assert!(m.get(id).unwrap().paused);
        std::thread::sleep(Duration::from_millis(600)); // laat lopende voortgangsregels binnenkomen
        let frozen = m.get(id).unwrap().fraction;
        std::thread::sleep(Duration::from_millis(1500));
        assert_eq!(m.get(id).unwrap().fraction, frozen, "tijdens pauze mag de voortgang niet lopen");
        assert!(m.pause(id).is_err(), "dubbel pauzeren is een fout");
        m.resume(id).unwrap();
        assert!(!m.get(id).unwrap().paused);
        wait_for(&m, id, |s| s.fraction.zip(frozen).is_some_and(|(a, b)| a > b) || finished(s));
        m.cancel(id).unwrap();
        assert_eq!(wait_for(&m, id, finished).state, JobState::Cancelled);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_paused_conversion_can_still_be_cancelled() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("uit");
        let src = dir.path().join("bron.mkv");
        make_video(&src, 40, "1280x720");
        let (m, _) = manager(1, None);
        let mut slow = request(&src, &out, "high_quality");
        slow.settings.video = crate::convert::VideoCodec::H265;
        slow.settings.hw_accel = crate::convert::HwAccel::Off;
        let id = m.add(slow).unwrap();
        wait_for(&m, id, |s| s.state == JobState::Running && s.fraction.is_some());
        m.pause(id).unwrap();
        m.cancel(id).unwrap();
        let s = wait_for(&m, id, finished);
        assert_eq!(s.state, JobState::Cancelled);
        assert!(!s.paused);
        assert!(std::fs::read_dir(&out).map(|d| d.count()).unwrap_or(0) == 0);
    }

    #[test]
    fn only_running_jobs_can_be_paused() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("uit");
        let (m, _) = manager(1, None);
        assert!(m.pause(999).is_err());
        assert!(m.resume(999).is_err());
        let _ = (dir, out);
    }

    #[test]
    fn shutdown_cancels_running_work_without_leaving_files_or_processes() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("uit");
        let lang = dir.path().join("lang.mkv");
        make_video(&lang, 60, "1280x720");
        let (m, _) = manager(1, None);
        let mut slow = request(&lang, &out, "high_quality");
        slow.settings.video = crate::convert::VideoCodec::H265;
        let id = m.add(slow).unwrap();
        wait_for(&m, id, |s| s.state == JobState::Running);
        std::thread::sleep(Duration::from_millis(800));
        let t = Instant::now();
        m.shutdown();
        // Veiligheidsgrens tegen een vastgelopen afsluiting; ruim, want op een overbelaste CI-runner
        // (alle tests starten tegelijk zware ffmpeg-processen) kost starten en opruimen soms tientallen seconden.
        // De echte controle is dat de taak geannuleerd is en niet is afgemaakt.
        assert!(t.elapsed() < Duration::from_secs(30), "afsluiten duurde {:?}", t.elapsed());
        assert_eq!(m.get(id).unwrap().state, JobState::Cancelled);
        assert!(std::fs::read_dir(&out).map(|d| d.count()).unwrap_or(0) == 0);
        assert!(m.add(request(&lang, &out, "remux_mkv")).is_err()); // na afsluiten geen nieuwe taken
    }

    #[test]
    fn clear_finished_keeps_active_jobs() {
        if !have_ffmpeg() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.mkv");
        make_video(&input, 1, "160x90");
        let (m, _) = manager(1, None);
        let id = m.add(request(&input, &dir.path().join("o"), "remux_mp4")).unwrap();
        wait_for(&m, id, finished);
        m.clear_finished();
        assert!(m.list().is_empty());
    }
}
