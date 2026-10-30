//! Voert één conversie uit: schrijft eerst naar een tijdelijk `.part`-bestand,
//! meet de echte voortgang, ondersteunt annuleren en controleert het resultaat
//! voordat het als geslaagd geldt.

use super::args::{build_args, Capabilities};
use super::{Container, Mode, Settings};
use crate::error::{CoreError, Result};
use crate::probe::{probe_file, MediaInfo};
use crate::validate::{ensure_not_same_file, sanitize_file_name};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overwrite {
    Skip,
    Rename,
    /// Alleen gebruiken nadat de gebruiker dit expliciet heeft bevestigd.
    Replace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputSpec {
    pub dir: PathBuf,
    /// Tokens: {name} {ext} {profile} {date}
    pub template: String,
    pub overwrite: Overwrite,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Resolved {
    Path(PathBuf),
    Skipped(PathBuf),
}

fn civil_date(unix_secs: i64) -> String {
    // Algoritme van Howard Hinnant (civil_from_days).
    let z = unix_secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn expand_template(template: &str, input: &Path, profile: &str, now_unix: i64) -> String {
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = input.extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let raw =
        template.replace("{name}", &stem).replace("{ext}", &ext).replace("{profile}", profile).replace("{date}", &civil_date(now_unix));
    sanitize_file_name(&raw)
}

pub fn resolve_output(input: &Path, spec: &OutputSpec, container: Container, profile: &str, now_unix: i64) -> Result<Resolved> {
    if !spec.dir.is_absolute() {
        return Err(CoreError::Invalid("De uitvoermap moet een absoluut pad zijn".into()));
    }
    std::fs::create_dir_all(&spec.dir)?;
    let name = expand_template(if spec.template.trim().is_empty() { "{name}" } else { &spec.template }, input, profile, now_unix);
    let candidate = spec.dir.join(format!("{name}.{}", container.ext()));
    ensure_not_same_file(input, &candidate)?;
    if !candidate.exists() {
        return Ok(Resolved::Path(candidate));
    }
    match spec.overwrite {
        Overwrite::Skip => Ok(Resolved::Skipped(candidate)),
        Overwrite::Replace => Ok(Resolved::Path(candidate)),
        Overwrite::Rename => {
            for n in 1..1000 {
                let alt = spec.dir.join(format!("{name} ({n}).{}", container.ext()));
                if !alt.exists() {
                    return Ok(Resolved::Path(alt));
                }
            }
            Err(CoreError::Invalid("Geen vrije bestandsnaam gevonden".into()))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Progress {
    /// `None` als de duur onbekend is: dan tonen we geen percentage.
    pub fraction: Option<f64>,
    pub eta_s: Option<u64>,
    pub speed: Option<f64>,
    pub out_time_ms: u64,
}

pub struct Tools<'a> {
    pub ffmpeg: &'a Path,
    pub ffprobe: &'a Path,
    pub caps: &'a Capabilities,
}

#[derive(Debug)]
pub struct RunOutcome {
    pub output: PathBuf,
    pub mode: Mode,
    pub warnings: Vec<String>,
}

fn temp_path_for(final_path: &Path) -> PathBuf {
    let mut s = final_path.as_os_str().to_os_string();
    s.push(".part");
    PathBuf::from(s)
}

/// Geschatte benodigde ruimte in bytes.
fn estimate_bytes(input_size: u64, s: &Settings) -> u64 {
    let factor = if s.container.is_audio_only() { 0.5 } else { 1.1 };
    (input_size as f64 * factor) as u64
}

pub fn check_disk_space(dir: &Path, needed: u64) -> Result<()> {
    if let Ok(free) = fs4::available_space(dir) {
        let required = needed + needed / 10 + 10 * 1024 * 1024;
        if free < required {
            return Err(CoreError::Invalid(format!(
                "Onvoldoende schijfruimte: {} MB vrij, ongeveer {} MB nodig",
                free / 1_048_576,
                required / 1_048_576
            )));
        }
    }
    Ok(())
}

fn parse_speed(v: &str) -> Option<f64> {
    v.trim().trim_end_matches('x').trim().parse::<f64>().ok().filter(|s| *s > 0.0)
}

#[allow(clippy::too_many_arguments)]
pub fn run_conversion(
    tools: &Tools,
    input: &Path,
    final_path: &Path,
    settings: &Settings,
    info: &MediaInfo,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<RunOutcome> {
    let dir = final_path.parent().ok_or_else(|| CoreError::Invalid("Ongeldig uitvoerpad".into()))?;
    check_disk_space(dir, estimate_bytes(info.size, settings))?;

    let temp = temp_path_for(final_path);
    let plan = build_args(input, &temp, settings, info, tools.caps)?;
    let result = execute(tools, plan, &temp, final_path, cancel, on_progress);
    // Een videokaart die bij het starten of tijdens het coderen faalt (driver, geheugen, niet-ondersteund formaat)
    // mag de conversie niet laten mislukken: één nieuwe poging met de processor.
    if let (Err(CoreError::Ffmpeg(msg)), Some(backend)) = (&result, hw_used(settings, tools.caps, input, &temp, info)) {
        let mut cpu = settings.clone();
        cpu.hw_accel = super::HwAccel::Off;
        let mut plan = build_args(input, &temp, &cpu, info, tools.caps)?;
        let first_line = msg.lines().next().unwrap_or("onbekende fout").trim().to_string();
        plan.warnings.push(format!("Coderen met {} mislukte ({first_line}); de processor is gebruikt", backend.label()));
        return execute(tools, plan, &temp, final_path, cancel, on_progress);
    }
    result
}

/// Welke videokaart het plan voor deze instellingen zou gebruiken (voor de terugval).
fn hw_used(settings: &Settings, caps: &Capabilities, input: &Path, temp: &Path, info: &MediaInfo) -> Option<super::HwBackend> {
    build_args(input, temp, settings, info, caps).ok().and_then(|p| p.hardware)
}

#[allow(clippy::too_many_arguments)]
fn execute(
    tools: &Tools,
    plan: super::args::Plan,
    temp: &Path,
    final_path: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<RunOutcome> {
    let temp = temp.to_path_buf();
    let _ = std::fs::remove_file(&temp); // restant van een eerdere, afgebroken poging

    let mut child = crate::proc::background(tools.ffmpeg)
        .args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::Ffmpeg(format!("FFmpeg kon niet worden gestart ({}): {e}", tools.ffmpeg.display())))?;

    let stdout = child.stdout.take().expect("stdout is gepiped");
    let stderr = child.stderr.take().expect("stderr is gepiped");

    let (tx, rx) = mpsc::channel::<String>();
    let out_thread = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(|l| l.ok()) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let err_thread = std::thread::spawn(move || {
        let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        for line in BufReader::new(stderr).lines().map_while(|l| l.ok()) {
            if tail.len() == 30 {
                tail.pop_front();
            }
            tail.push_back(line);
        }
        tail.into_iter().collect::<Vec<_>>().join("\n")
    });

    let cleanup = |temp: &Path| {
        let _ = std::fs::remove_file(temp);
    };

    let total_ms = plan.duration_ms;
    let mut out_time_us: u64 = 0;
    let mut speed: Option<f64> = None;
    let mut cancelled = false;
    loop {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            let _ = child.kill();
            break;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if let Some((k, v)) = line.split_once('=') {
                    match k {
                        // Beide sleutels bevatten microseconden (ffmpeg-eigenaardigheid).
                        "out_time_us" | "out_time_ms" => out_time_us = v.trim().parse().unwrap_or(out_time_us),
                        "speed" => speed = parse_speed(v).or(speed),
                        "progress" => {
                            let out_ms = out_time_us / 1000;
                            let end = v.trim() == "end";
                            let fraction =
                                (total_ms > 0).then(|| if end { 1.0 } else { (out_ms as f64 / total_ms as f64).clamp(0.0, 0.999) });
                            let eta = match (fraction, speed) {
                                (Some(f), Some(sp)) if !end => {
                                    let remaining_ms = (total_ms as f64 * (1.0 - f)).max(0.0);
                                    Some((remaining_ms / 1000.0 / sp).round() as u64)
                                }
                                _ => None,
                            };
                            on_progress(Progress { fraction, eta_s: eta, speed, out_time_ms: out_ms });
                        }
                        _ => {}
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let status = child.wait()?;
    let _ = out_thread.join();
    let stderr_tail = err_thread.join().unwrap_or_default();

    if cancelled {
        cleanup(&temp);
        return Err(CoreError::Cancelled);
    }
    if !status.success() {
        cleanup(&temp);
        let msg = if stderr_tail.trim().is_empty() { format!("FFmpeg stopte met {status}") } else { stderr_tail };
        return Err(CoreError::Ffmpeg(msg));
    }
    // Pas geslaagd als het bestand er echt is en door ffprobe te lezen is.
    let size = std::fs::metadata(&temp).map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        cleanup(&temp);
        return Err(CoreError::Ffmpeg("FFmpeg meldde succes, maar het uitvoerbestand ontbreekt of is leeg".into()));
    }
    if let Err(e) = probe_file(tools.ffprobe, &temp) {
        cleanup(&temp);
        return Err(CoreError::Ffmpeg(format!("Het uitvoerbestand is niet leesbaar: {e}")));
    }
    if let Err(e) = std::fs::rename(&temp, final_path) {
        cleanup(&temp);
        return Err(e.into());
    }
    on_progress(Progress { fraction: Some(1.0), eta_s: Some(0), speed, out_time_ms: total_ms });
    Ok(RunOutcome { output: final_path.to_path_buf(), mode: plan.mode, warnings: plan.warnings })
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;
    use std::process::Command;

    pub fn have_ffmpeg() -> bool {
        Command::new("ffmpeg").arg("-version").output().is_ok() && Command::new("ffprobe").arg("-version").output().is_ok()
    }

    /// Maakt een echt testbestand van `secs` seconden (video + audio).
    pub fn make_video(path: &Path, secs: u32, size: &str) {
        let st = Command::new("ffmpeg")
            .args(["-loglevel", "error", "-y", "-f", "lavfi", "-i"])
            .arg(format!("testsrc2=size={size}:rate=25"))
            .args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000", "-t"])
            .arg(secs.to_string())
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
            .arg(path)
            .status()
            .unwrap();
        assert!(st.success());
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::convert::{builtin_profiles, AudioCodec, VideoCodec};
    use crate::probe::{probe_file, StreamKind};
    use std::process::Command;

    fn settings(id: &str) -> Settings {
        builtin_profiles().into_iter().find(|p| p.id == id).unwrap().settings
    }

    struct Env {
        caps: Capabilities,
    }
    impl Env {
        fn new() -> Env {
            Env { caps: Capabilities::detect(Path::new("ffmpeg")).unwrap() }
        }
        fn tools(&self) -> Tools<'_> {
            Tools { ffmpeg: Path::new("ffmpeg"), ffprobe: Path::new("ffprobe"), caps: &self.caps }
        }
    }

    fn convert(env: &Env, input: &Path, out: &Path, s: &Settings, cancel: &AtomicBool) -> (Result<RunOutcome>, Vec<Progress>) {
        let info = probe_file(Path::new("ffprobe"), input).unwrap();
        let mut ticks = vec![];
        let r = run_conversion(&env.tools(), input, out, s, &info, cancel, &mut |p| ticks.push(p));
        (r, ticks)
    }

    #[test]
    fn a_gpu_encoder_that_fails_falls_back_to_the_cpu_and_says_so() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 3, "320x180");
        let out = dir.path().join("uit.mp4");
        // NVIDIA afdwingen: op een pc zonder NVIDIA-kaart (zoals deze testmachine) faalt de encoder bij het starten
        // of kent FFmpeg hem niet; in beide gevallen moet de conversie met de processor slagen.
        let mut s = settings("compact");
        s.hw_accel = crate::convert::HwAccel::Nvenc;
        let (r, _) = convert(&env, &input, &out, &s, &AtomicBool::new(false));
        let outcome = r.expect("terugval op de processor moet slagen");
        let info = probe_file(Path::new("ffprobe"), &out).unwrap();
        assert_eq!(info.first_video().unwrap().codec, "h264");
        let gpu_present = std::process::Command::new("nvidia-smi").output().map(|o| o.status.success()).unwrap_or(false);
        if !gpu_present {
            assert!(outcome.warnings.iter().any(|w| w.contains("NVIDIA")), "{:?}", outcome.warnings);
        }
    }

    #[test]
    fn an_explicit_thread_count_is_accepted_by_the_real_encoders() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 2, "320x180");
        for (video, codec, enc) in [(VideoCodec::H264, "h264", "libx264"), (VideoCodec::H265, "hevc", "libx265")] {
            if !env.caps.has_encoder(enc) {
                continue;
            }
            let mut s = settings("compact");
            s.video = video;
            s.hw_accel = crate::convert::HwAccel::Off;
            s.threads = Some(2);
            let out = dir.path().join(format!("uit-{codec}.mp4"));
            let (r, _) = convert(&env, &input, &out, &s, &AtomicBool::new(false));
            r.unwrap_or_else(|e| panic!("{enc} met 2 threads: {e}"));
            assert_eq!(probe_file(Path::new("ffprobe"), &out).unwrap().first_video().unwrap().codec, codec);
        }
    }

    #[test]
    fn the_new_phone_and_music_profiles_really_convert() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 3, "960x540");
        for (id, ext, check_height) in [("phone", "mp4", Some(480)), ("music_opus", "opus", None), ("music_m4a", "m4a", None)] {
            let s = settings(id);
            if !env.caps.container_available(s.container)
                || (!s.container.is_audio_only() && !env.caps.video_available(s.video))
                || !env.caps.audio_available(s.audio)
            {
                continue;
            }
            let out = dir.path().join(format!("uit-{id}.{ext}"));
            let (r, _) = convert(&env, &input, &out, &s, &AtomicBool::new(false));
            r.unwrap_or_else(|e| panic!("profiel {id}: {e}"));
            let info = probe_file(Path::new("ffprobe"), &out).unwrap();
            if let Some(h) = check_height {
                assert_eq!(info.first_video().unwrap().height, Some(h), "profiel {id}");
            } else {
                assert!(info.first_video().is_none(), "profiel {id} hoort alleen audio te zijn");
            }
        }
    }

    #[test]
    fn trimming_really_cuts_the_output_and_progress_reaches_one() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 8, "320x180");
        let out = dir.path().join("uit.mp4");
        let mut s = settings("compact");
        s.hw_accel = crate::convert::HwAccel::Off;
        s.trim_start_ms = Some(2_000);
        s.trim_end_ms = Some(5_000);
        let (r, ticks) = convert(&env, &input, &out, &s, &AtomicBool::new(false));
        r.unwrap();
        let info = probe_file(Path::new("ffprobe"), &out).unwrap();
        assert!((2_500..3_700).contains(&info.duration_ms), "duur {}", info.duration_ms);
        assert_eq!(ticks.last().and_then(|t| t.fraction), Some(1.0));
    }

    #[test]
    fn civil_date_is_correct() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(951_782_400), "2000-02-29"); // schrikkeldag
        assert_eq!(civil_date(1_761_782_400), "2025-10-30");
    }

    #[test]
    fn template_expands_tokens_and_sanitizes() {
        let n = expand_template("{name} - {profile} ({date})", Path::new("/m/Mijn film.mkv"), "Compact/720p", 0);
        assert_eq!(n, "Mijn film - Compact_720p (1970-01-01)");
        assert_eq!(expand_template("../../etc/{name}", Path::new("/m/a.mkv"), "p", 0), ".._.._etc_a");
    }

    #[test]
    fn output_resolution_handles_skip_rename_replace_and_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("film.mkv");
        std::fs::write(&input, b"x").unwrap();
        let spec = |o| OutputSpec { dir: dir.path().to_path_buf(), template: "{name}".into(), overwrite: o };
        // zelfde bestand: geweigerd
        assert!(resolve_output(&input, &spec(Overwrite::Replace), Container::Mkv, "p", 0).is_err());
        // nieuw bestand
        let out = dir.path().join("film.mp4");
        assert_eq!(resolve_output(&input, &spec(Overwrite::Skip), Container::Mp4, "p", 0).unwrap(), Resolved::Path(out.clone()));
        std::fs::write(&out, b"oud").unwrap();
        assert_eq!(resolve_output(&input, &spec(Overwrite::Skip), Container::Mp4, "p", 0).unwrap(), Resolved::Skipped(out.clone()));
        assert_eq!(resolve_output(&input, &spec(Overwrite::Replace), Container::Mp4, "p", 0).unwrap(), Resolved::Path(out.clone()));
        assert_eq!(
            resolve_output(&input, &spec(Overwrite::Rename), Container::Mp4, "p", 0).unwrap(),
            Resolved::Path(dir.path().join("film (1).mp4"))
        );
        // relatieve map wordt geweigerd
        let bad = OutputSpec { dir: "relatief".into(), template: "{name}".into(), overwrite: Overwrite::Skip };
        assert!(resolve_output(&input, &bad, Container::Mp4, "p", 0).is_err());
    }

    #[test]
    fn remux_produces_a_readable_mp4_with_real_progress() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 6, "320x180");
        let out = dir.path().join("uit.mp4");
        let (r, ticks) = convert(&env, &input, &out, &settings("remux_mp4"), &AtomicBool::new(false));
        let outcome = r.unwrap();
        assert_eq!(outcome.mode, Mode::Remux);
        let info = probe_file(Path::new("ffprobe"), &out).unwrap();
        assert!(info.format.contains("mp4"));
        assert_eq!(info.first_video().unwrap().codec, "h264"); // niet opnieuw gecodeerd
        assert!((5_500..6_600).contains(&info.duration_ms), "duur {}", info.duration_ms);
        assert!(!dir.path().join("uit.mp4.part").exists());
        assert!(!ticks.is_empty());
        assert_eq!(ticks.last().unwrap().fraction, Some(1.0));
        // voortgang loopt nooit achteruit
        let fr: Vec<f64> = ticks.iter().filter_map(|t| t.fraction).collect();
        assert!(fr.windows(2).all(|w| w[0] <= w[1]), "{fr:?}");
    }

    #[test]
    fn transcode_changes_codec_and_resolution_without_upscaling() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 2, "640x360");
        let mut s = settings("compact");
        s.video = VideoCodec::H265;
        s.max_height = Some(240);
        let out = dir.path().join("klein.mp4");
        let (r, _) = convert(&env, &input, &out, &s, &AtomicBool::new(false));
        assert_eq!(r.unwrap().mode, Mode::Transcode);
        let v = probe_file(Path::new("ffprobe"), &out).unwrap().first_video().unwrap().clone();
        assert_eq!((v.codec.as_str(), v.height), ("hevc", Some(240)));
        // hoogte 1080 op een 360p-bron: blijft 360
        let mut s2 = settings("hd_1080");
        s2.video = VideoCodec::H264;
        let out2 = dir.path().join("niet-groter.mp4");
        convert(&env, &input, &out2, &s2, &AtomicBool::new(false)).0.unwrap();
        assert_eq!(probe_file(Path::new("ffprobe"), &out2).unwrap().first_video().unwrap().height, Some(360));
    }

    #[test]
    fn external_subtitle_is_really_muxed_into_the_output() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 2, "160x90");
        let srt = dir.path().join("film.nl.srt");
        std::fs::write(&srt, "1\n00:00:00,000 --> 00:00:01,500\nHallo\n").unwrap();
        let mut s = settings("remux_mp4");
        s.external_subtitle = Some(srt.to_string_lossy().into_owned());
        s.external_subtitle_language = Some("nld".into());
        let out = dir.path().join("met-ondertitel.mp4");
        convert(&env, &input, &out, &s, &AtomicBool::new(false)).0.unwrap();
        let info = probe_file(Path::new("ffprobe"), &out).unwrap();
        let sub = info.streams.iter().find(|s| s.kind == StreamKind::Subtitle).expect("ondertitelspoor ontbreekt");
        assert_eq!((sub.codec.as_str(), sub.language.as_deref()), ("mov_text", Some("nld")));
        assert_eq!(info.first_video().unwrap().codec, "h264"); // video niet opnieuw gecodeerd
                                                               // ondertitel in mkv blijft srt
        let mut m = settings("remux_mkv");
        m.external_subtitle = s.external_subtitle.clone();
        let out2 = dir.path().join("met-ondertitel.mkv");
        convert(&env, &input, &out2, &m, &AtomicBool::new(false)).0.unwrap();
        assert!(probe_file(Path::new("ffprobe"), &out2).unwrap().streams.iter().any(|s| s.kind == StreamKind::Subtitle));
    }

    #[test]
    fn audio_extraction_to_mp3_and_flac_works() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 2, "160x90");
        for (id, ext, codec) in [("audio_only", "mp3", "mp3"), ("music_hq", "flac", "flac")] {
            let out = dir.path().join(format!("a.{ext}"));
            convert(&env, &input, &out, &settings(id), &AtomicBool::new(false)).0.unwrap();
            let info = probe_file(Path::new("ffprobe"), &out).unwrap();
            assert!(!info.has_video(), "{ext}");
            assert_eq!(info.streams.iter().find(|s| s.kind == StreamKind::Audio).unwrap().codec, codec);
        }
    }

    fn mean_volume_db(file: &Path) -> f64 {
        let out = Command::new("ffmpeg")
            .args(["-hide_banner", "-nostats", "-i"])
            .arg(file)
            .args(["-af", "volumedetect", "-f", "null", "-"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stderr).into_owned();
        text.lines()
            .find_map(|l| l.split("mean_volume:").nth(1))
            .and_then(|v| v.trim().trim_end_matches("dB").trim().parse().ok())
            .unwrap_or_else(|| panic!("geen mean_volume in: {text}"))
    }

    #[test]
    fn equalizer_really_changes_the_audio_signal() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        // 1 kHz-toon: valt precies op de band van 1000 Hz
        let input = dir.path().join("toon.wav");
        let st = Command::new("ffmpeg")
            .args(["-loglevel", "error", "-y", "-f", "lavfi", "-i", "sine=frequency=1000:sample_rate=48000", "-t", "2"])
            .arg(&input)
            .status()
            .unwrap();
        assert!(st.success());
        let render = |name: &str, eq: Option<crate::audio::EqConfig>, normalize: bool| {
            let mut s = settings("music_hq");
            s.equalizer = eq;
            s.normalize_audio = normalize;
            let out = dir.path().join(name);
            convert(&env, &input, &out, &s, &AtomicBool::new(false)).0.unwrap();
            mean_volume_db(&out)
        };
        let flat = render("vlak.flac", None, false);
        let mut boost = crate::audio::EqConfig::default();
        boost.bands_db[4] = 6.0; // 1000 Hz +6 dB
        let boosted = render("boost.flac", Some(boost.clone()), false);
        let mut cut = crate::audio::EqConfig::default();
        cut.bands_db[4] = -6.0;
        let lowered = render("cut.flac", Some(cut), false);
        assert!(boosted - flat > 3.0, "boost: {flat} -> {boosted}");
        assert!(flat - lowered > 3.0, "cut: {flat} -> {lowered}");
        // een band ver van 1 kHz (60 Hz) verandert de 1 kHz-toon nauwelijks
        let mut far = crate::audio::EqConfig::default();
        far.bands_db[0] = 6.0;
        let unaffected = render("ver.flac", Some(far), false);
        assert!((unaffected - flat).abs() < 1.0, "{flat} vs {unaffected}");
        // normalisatie trekt een zachte toon omhoog richting -16 LUFS
        let normalized = render("norm.flac", None, true);
        assert!(normalized > flat + 3.0, "{flat} -> {normalized}");
    }

    #[test]
    fn webm_vp9_opus_transcode_works_when_available() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        if !env.caps.video_available(VideoCodec::Vp9) || !env.caps.audio_available(AudioCodec::Opus) {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.mkv");
        make_video(&input, 1, "160x90");
        let mut s = settings("remux_mkv");
        s.container = Container::Webm;
        s.video = VideoCodec::Vp9;
        s.audio = AudioCodec::Opus;
        let out = dir.path().join("x.webm");
        convert(&env, &input, &out, &s, &AtomicBool::new(false)).0.unwrap();
        let info = probe_file(Path::new("ffprobe"), &out).unwrap();
        assert_eq!(info.first_video().unwrap().codec, "vp9");
    }

    #[test]
    fn corrupt_input_reports_the_real_ffmpeg_error_and_leaves_nothing_behind() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("goed.mkv");
        make_video(&good, 1, "160x90");
        let info = probe_file(Path::new("ffprobe"), &good).unwrap();
        let bad = dir.path().join("kapot.mkv");
        std::fs::write(&bad, b"dit is geen video").unwrap();
        let out = dir.path().join("uit.mp4");
        let r = run_conversion(&env.tools(), &bad, &out, &settings("remux_mp4"), &info, &AtomicBool::new(false), &mut |_| {});
        match r {
            Err(CoreError::Ffmpeg(m)) => assert!(!m.trim().is_empty()),
            other => panic!("verwachtte een FFmpeg-fout: {other:?}"),
        }
        assert!(!out.exists() && !dir.path().join("uit.mp4.part").exists());
    }

    #[test]
    fn cancellation_stops_ffmpeg_and_removes_the_partial_file() {
        if !have_ffmpeg() {
            return;
        }
        let env = Env::new();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("lang.mkv");
        make_video(&input, 60, "1280x720");
        let mut s = settings("high_quality");
        s.video = VideoCodec::H265; // traag genoeg om te kunnen annuleren
        let out = dir.path().join("uit.mp4");
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let c2 = cancel.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1200));
            c2.store(true, Ordering::Relaxed);
        });
        let started = std::time::Instant::now();
        let (r, _) = convert(&env, &input, &out, &s, &cancel);
        t.join().unwrap();
        assert!(matches!(r, Err(CoreError::Cancelled)), "{r:?}");
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(!out.exists() && !dir.path().join("uit.mp4.part").exists());
    }

    #[test]
    fn missing_ffmpeg_binary_is_a_clear_error() {
        let caps = Capabilities::from_text(
            include_str!("../../tests/fixtures/ffmpeg_encoders.txt"),
            include_str!("../../tests/fixtures/ffmpeg_muxers.txt"),
        );
        let tools = Tools { ffmpeg: Path::new("/bestaat/niet/ffmpeg"), ffprobe: Path::new("ffprobe"), caps: &caps };
        let info = crate::probe::parse_ffprobe_json(include_str!("../../tests/fixtures/sample.ffprobe.json")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let r = run_conversion(
            &tools,
            Path::new("/in.mkv"),
            &dir.path().join("o.mkv"),
            &settings("remux_mkv"),
            &info,
            &AtomicBool::new(false),
            &mut |_| {},
        );
        match r {
            Err(CoreError::Ffmpeg(m)) => assert!(m.contains("kon niet worden gestart"), "{m}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn insufficient_disk_space_is_reported_before_starting() {
        let dir = tempfile::tempdir().unwrap();
        let r = check_disk_space(dir.path(), u64::MAX / 4);
        assert!(matches!(r, Err(CoreError::Invalid(m)) if m.contains("schijfruimte")));
        assert!(check_disk_space(dir.path(), 1024).is_ok());
    }
}
