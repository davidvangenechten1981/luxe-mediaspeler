//! Afspeelsessie: verbindt wachtrij, A-B-lus, hervatten en geschiedenis met een
//! afspeelengine. De engine (LibVLC) zit achter de `Engine`-trait, zodat alle
//! logica hier zonder echte speler te testen is.

use super::ab::AbLoop;
use super::queue::{Queue, QueueItem, RepeatMode};
use crate::audio::EqConfig;
use crate::db::{Database, ResumePoint, SkipMarks, TrackPref};
use crate::error::{CoreError, Result};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    #[default]
    Idle,
    Opening,
    Buffering,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Track {
    pub id: i32,
    pub name: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct EngineStatus {
    pub state: PlaybackState,
    pub time_ms: u64,
    pub length_ms: u64,
    pub volume: u32,
    pub muted: bool,
    pub rate: f32,
    pub audio_tracks: Vec<Track>,
    pub current_audio: Option<i32>,
    pub subtitle_tracks: Vec<Track>,
    /// `None` = ondertitels uit
    pub current_subtitle: Option<i32>,
    pub audio_delay_ms: i64,
    pub subtitle_delay_ms: i64,
    pub buffering_percent: Option<f32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OpenOptions {
    pub start_ms: u64,
    pub network_caching_ms: Option<u32>,
    /// Voor audiobestanden: laat de engine een spectrum tekenen in het videovenster.
    pub audio_visual: bool,
}

pub trait Engine: Send {
    fn open(&mut self, location: &str, opts: &OpenOptions) -> Result<()>;
    fn play(&mut self) -> Result<()>;
    fn pause(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn seek(&mut self, ms: u64) -> Result<()>;
    fn set_volume(&mut self, percent: u32) -> Result<()>;
    fn set_mute(&mut self, muted: bool) -> Result<()>;
    fn set_rate(&mut self, rate: f32) -> Result<()>;
    fn set_audio_track(&mut self, id: i32) -> Result<()>;
    /// `None` zet ondertitels uit.
    fn set_subtitle_track(&mut self, id: Option<i32>) -> Result<()>;
    fn load_subtitle_file(&mut self, path: &Path) -> Result<()>;
    fn set_audio_delay_ms(&mut self, ms: i64) -> Result<()>;
    fn set_subtitle_delay_ms(&mut self, ms: i64) -> Result<()>;
    fn next_frame(&mut self) -> Result<()>;
    fn take_snapshot(&mut self, path: &Path) -> Result<()>;
    fn set_equalizer(&mut self, eq: Option<&EqConfig>) -> Result<()>;
    /// Zet hardwaredecodering uit voor volgende pogingen. Geeft `true` als er iets is veranderd,
    /// `false` als dit niet kan of al uit staat.
    fn disable_hw_decoding(&mut self) -> bool {
        false
    }
    fn status(&self) -> EngineStatus;
}

/// Opslag van hervatposities en geschiedenis.
pub trait Persistence: Send + Sync {
    fn resume(&self, location: &str) -> Option<ResumePoint>;
    fn save_resume(&self, location: &str, position_ms: u64, duration_ms: u64);
    fn mark_completed(&self, location: &str);
    fn record_play(&self, location: &str, title: Option<&str>);
    /// Eerder gekozen audio- en ondertitelspoor voor dit bestand.
    fn track_pref(&self, _location: &str) -> Option<TrackPref> {
        None
    }
    fn save_track_pref(&self, _location: &str, _pref: &TrackPref) {}
    fn skip_marks(&self, _location: &str) -> Option<SkipMarks> {
        None
    }
    fn save_skip_marks(&self, _location: &str, _marks: &SkipMarks) {}
}

impl Persistence for Mutex<Database> {
    fn resume(&self, location: &str) -> Option<ResumePoint> {
        self.lock().ok()?.get_resume(location).ok().flatten()
    }
    fn save_resume(&self, location: &str, position_ms: u64, duration_ms: u64) {
        if let Ok(db) = self.lock() {
            let _ = db.save_resume(location, position_ms, duration_ms);
        }
    }
    fn mark_completed(&self, location: &str) {
        if let Ok(db) = self.lock() {
            let _ = db.mark_completed(location, true);
        }
    }
    fn record_play(&self, location: &str, title: Option<&str>) {
        if let Ok(mut db) = self.lock() {
            let _ = db.record_play(location, title);
        }
    }
    fn track_pref(&self, location: &str) -> Option<TrackPref> {
        self.lock().ok()?.get_track_pref(location).ok().flatten()
    }
    fn save_track_pref(&self, location: &str, pref: &TrackPref) {
        if let Ok(db) = self.lock() {
            let _ = db.save_track_pref(location, pref);
        }
    }
    fn skip_marks(&self, location: &str) -> Option<SkipMarks> {
        self.lock().ok()?.get_skip_marks(location).ok().flatten()
    }
    fn save_skip_marks(&self, location: &str, marks: &SkipMarks) {
        if let Ok(db) = self.lock() {
            let _ = db.save_skip_marks(location, marks);
        }
    }
}

impl<T: Persistence + ?Sized> Persistence for std::sync::Arc<T> {
    fn resume(&self, location: &str) -> Option<ResumePoint> {
        (**self).resume(location)
    }
    fn save_resume(&self, location: &str, position_ms: u64, duration_ms: u64) {
        (**self).save_resume(location, position_ms, duration_ms)
    }
    fn mark_completed(&self, location: &str) {
        (**self).mark_completed(location)
    }
    fn record_play(&self, location: &str, title: Option<&str>) {
        (**self).record_play(location, title)
    }
    fn track_pref(&self, location: &str) -> Option<TrackPref> {
        (**self).track_pref(location)
    }
    fn save_track_pref(&self, location: &str, pref: &TrackPref) {
        (**self).save_track_pref(location, pref)
    }
    fn skip_marks(&self, location: &str) -> Option<SkipMarks> {
        (**self).skip_marks(location)
    }
    fn save_skip_marks(&self, location: &str, marks: &SkipMarks) {
        (**self).save_skip_marks(location, marks)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    TrackChanged {
        index: usize,
        location: String,
    },
    ResumeApplied {
        position_ms: u64,
    },
    AbLoopJump {
        to_ms: u64,
    },
    OpenFailed {
        location: String,
        message: String,
    },
    /// Het afspelen faalde met hardwaredecodering; opnieuw gestart zonder.
    HwFallback {
        location: String,
    },
    QueueEnded,
}

/// Wat de knop "overslaan" nu kan doen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkipOffer {
    Intro,
    Credits,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionView {
    pub status: EngineStatus,
    pub queue_index: Option<usize>,
    pub queue_len: usize,
    pub repeat: RepeatMode,
    pub shuffle: bool,
    pub ab_a_ms: Option<u64>,
    pub ab_b_ms: Option<u64>,
    pub location: Option<String>,
    /// Resterende milliseconden van de aftelling voor het volgende item (alleen na een video).
    pub next_in_ms: Option<u64>,
    /// Staat de speler in een gemarkeerde intro of aftiteling? Dan kan die worden overgeslagen.
    pub skip_offer: Option<SkipOffer>,
}

pub const RATE_STEPS: [f32; 9] = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0];
const RESUME_SAVE_EVERY_MS: u64 = 5_000;
const MIN_RESUME_MS: u64 = 5_000;
pub const MAX_VOLUME: u32 = 200;

/// Volgende spoor in de lijst (met wrap-around). Met `include_off` hoort "uit" bij de cyclus.
pub fn next_track(tracks: &[Track], current: Option<i32>, include_off: bool) -> Option<i32> {
    let mut cycle: Vec<Option<i32>> = vec![];
    if include_off {
        cycle.push(None);
    }
    cycle.extend(tracks.iter().map(|t| Some(t.id)));
    if cycle.is_empty() {
        return None;
    }
    let pos = cycle.iter().position(|c| *c == current).map(|p| (p + 1) % cycle.len()).unwrap_or(0);
    cycle[pos]
}

/// Raadt een taalcode (ISO 639-2) uit een spoornaam zoals "Track 2 - [Nederlands]".
pub fn guess_language(name: &str) -> Option<&'static str> {
    const TABLE: [(&str, &[&str]); 8] = [
        ("nld", &["nld", "dut", "dutch", "nederlands", "vlaams"]),
        ("eng", &["eng", "english", "engels"]),
        ("fra", &["fra", "fre", "french", "frans", "français"]),
        ("deu", &["deu", "ger", "german", "duits", "deutsch"]),
        ("spa", &["spa", "spanish", "spaans", "español"]),
        ("ita", &["ita", "italian", "italiaans", "italiano"]),
        ("jpn", &["jpn", "japanese", "japans"]),
        ("por", &["por", "portuguese", "portugees"]),
    ];
    let lower = name.to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric() && c != 'ç' && c != 'é').filter(|w| !w.is_empty()).collect();
    TABLE.iter().find(|(_, names)| words.iter().any(|w| names.contains(w))).map(|(code, _)| *code)
}

/// Sleutel waarmee we een spoor in een ander afspeelmoment terugvinden: taalcode, anders de naam in kleine letters.
pub fn track_key(t: &Track) -> String {
    t.language.clone().or_else(|| guess_language(&t.name).map(str::to_string)).unwrap_or_else(|| t.name.trim().to_lowercase())
}

/// Kiest het eerste spoor dat bij een voorkeurstaal past (in volgorde van voorkeur).
pub fn pick_track(tracks: &[Track], preferred: &[String]) -> Option<i32> {
    preferred.iter().find_map(|lang| {
        tracks.iter().find(|t| t.language.as_deref().or_else(|| guess_language(&t.name)).is_some_and(|l| l == lang)).map(|t| t.id)
    })
}

pub struct Session<E: Engine> {
    engine: E,
    store: Box<dyn Persistence>,
    queue: Queue,
    ab: AbLoop,
    events: Vec<SessionEvent>,
    current_location: Option<String>,
    last_resume_save_ms: u64,
    prefs_pending: bool,
    fail_streak: usize,
    /// Tijdstip (klok van `tick`) waarop het wachtende volgende item start.
    pending_next: Option<u64>,
    next_in_ms: Option<u64>,
    skip_marks: SkipMarks,
    /// Het einde van het afspelen is al verwerkt; wordt weer onwaar zodra er iets nieuws start.
    end_reported: bool,
    /// Bestand waarvoor de terugval zonder hardwaredecodering al is geprobeerd.
    hw_fallback_done: Option<String>,
    /// Wachttijd na een afgelopen video voordat het volgende item start; 0 = direct.
    pub next_delay_ms: u64,
    pub resume_enabled: bool,
    pub network_caching_ms: Option<u32>,
    pub preferred_audio: Vec<String>,
    pub preferred_subtitles: Vec<String>,
}

/// Is dit item muziek (op bestandsextensie)?
pub fn is_audio_location(loc: &str) -> bool {
    let ext = Path::new(loc).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    ["mp3", "aac", "flac", "wav", "alac", "ogg", "opus", "wma", "aiff", "aif", "m4a"].contains(&ext.as_str())
}

impl<E: Engine> Session<E> {
    pub fn new(engine: E, store: Box<dyn Persistence>, seed: u64) -> Session<E> {
        Session {
            engine,
            store,
            queue: Queue::new(seed),
            ab: AbLoop::default(),
            events: vec![],
            current_location: None,
            last_resume_save_ms: 0,
            prefs_pending: false,
            fail_streak: 0,
            pending_next: None,
            next_in_ms: None,
            skip_marks: SkipMarks::default(),
            end_reported: false,
            hw_fallback_done: None,
            next_delay_ms: 0,
            resume_enabled: true,
            network_caching_ms: None,
            preferred_audio: vec![],
            preferred_subtitles: vec![],
        }
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }
    pub fn engine_mut(&mut self) -> &mut E {
        &mut self.engine
    }
    pub fn queue(&self) -> &Queue {
        &self.queue
    }
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn view(&self) -> SessionView {
        SessionView {
            status: self.engine.status(),
            queue_index: self.queue.current_index(),
            queue_len: self.queue.len(),
            repeat: self.queue.repeat(),
            shuffle: self.queue.shuffle(),
            ab_a_ms: self.ab.a(),
            ab_b_ms: self.ab.b(),
            location: self.current_location.clone(),
            next_in_ms: self.next_in_ms,
            skip_offer: self.skip_offer(),
        }
    }

    fn skip_offer(&self) -> Option<SkipOffer> {
        self.current_location.as_ref()?;
        let t = self.engine.status().time_ms;
        let m = &self.skip_marks;
        if let (Some(a), Some(b)) = (m.intro_start_ms, m.intro_end_ms) {
            if (a..b).contains(&t) {
                return Some(SkipOffer::Intro);
            }
        }
        m.credits_start_ms.filter(|c| t >= *c).map(|_| SkipOffer::Credits)
    }

    fn save_marks(&mut self) {
        if let Some(loc) = &self.current_location {
            self.store.save_skip_marks(loc, &self.skip_marks);
        }
    }

    /// Markeert de huidige positie als begin van de intro.
    pub fn mark_intro_start(&mut self) -> Result<()> {
        self.skip_marks.intro_start_ms = Some(self.engine.status().time_ms);
        self.save_marks();
        Ok(())
    }

    /// Markeert de huidige positie als einde van de intro; moet na het begin liggen.
    pub fn mark_intro_end(&mut self) -> Result<()> {
        let t = self.engine.status().time_ms;
        if self.skip_marks.intro_start_ms.is_some_and(|a| t <= a) {
            return Err(CoreError::Invalid("Het einde van de intro moet na het begin liggen".into()));
        }
        self.skip_marks.intro_end_ms = Some(t);
        self.save_marks();
        Ok(())
    }

    pub fn mark_credits_start(&mut self) -> Result<()> {
        self.skip_marks.credits_start_ms = Some(self.engine.status().time_ms);
        self.save_marks();
        Ok(())
    }

    pub fn clear_skip_marks(&mut self) {
        self.skip_marks = SkipMarks::default();
        self.save_marks();
    }

    /// Slaat de intro over (spring naar het einde) of gaat bij de aftiteling naar het volgende item.
    pub fn skip_now(&mut self) -> Result<()> {
        match self.skip_offer() {
            Some(SkipOffer::Intro) => self.seek_to(self.skip_marks.intro_end_ms.unwrap_or(0)),
            Some(SkipOffer::Credits) => self.skip_next(),
            None => Ok(()),
        }
    }

    // ---------- openen ----------

    /// Vervangt de wachtrij en speelt `start` af.
    pub fn play_items(&mut self, items: Vec<QueueItem>, start: usize) -> Result<()> {
        self.queue.replace(items, start)?;
        self.fail_streak = 0;
        self.open_current(true)
    }

    pub fn enqueue(&mut self, items: Vec<QueueItem>) -> usize {
        self.queue.append(items)
    }

    /// Haalt een item uit de wachtrij. Verwijder je het spelende item, dan gaat het volgende
    /// meteen spelen (of de speler stopt als de lijst leeg is).
    pub fn queue_remove(&mut self, index: usize) -> Result<()> {
        let was_current = self.queue.current_index() == Some(index);
        if was_current {
            self.save_position();
        }
        self.queue.remove(index)?;
        if !was_current {
            return Ok(());
        }
        self.current_location = None;
        if self.queue.is_empty() {
            self.ab.clear();
            return self.engine.stop();
        }
        self.queue.jump_to(index.min(self.queue.len() - 1))?;
        self.open_current(false)
    }

    pub fn queue_move(&mut self, from: usize, to: usize) -> Result<()> {
        self.queue.move_item(from, to)
    }

    /// Leegt de wachtrij en stopt het afspelen.
    pub fn queue_clear(&mut self) -> Result<()> {
        self.save_position();
        self.queue.clear();
        self.current_location = None;
        self.ab.clear();
        self.engine.stop()
    }

    pub fn play_index(&mut self, index: usize) -> Result<()> {
        self.queue.jump_to(index)?;
        self.open_current(true)
    }

    fn open_current(&mut self, allow_resume: bool) -> Result<()> {
        self.clear_countdown();
        self.save_position();
        let item = self.queue.current().cloned().ok_or_else(|| CoreError::Invalid("De wachtrij is leeg".into()))?;
        let resume_ms = if allow_resume && self.resume_enabled {
            self.store.resume(&item.location).filter(|r| !r.completed && r.position_ms > MIN_RESUME_MS).map(|r| r.position_ms)
        } else {
            None
        };
        let opts = OpenOptions {
            start_ms: resume_ms.unwrap_or(0),
            network_caching_ms: self.network_caching_ms,
            audio_visual: is_audio_location(&item.location),
        };
        if let Err(e) = self.engine.open(&item.location, &opts) {
            self.current_location = None;
            self.events.push(SessionEvent::OpenFailed { location: item.location.clone(), message: e.to_string() });
            return Err(e);
        }
        self.engine.play()?;
        self.store.record_play(&item.location, item.title.as_deref());
        self.ab.clear();
        self.prefs_pending = true;
        self.last_resume_save_ms = 0;
        self.current_location = Some(item.location.clone());
        self.end_reported = false;
        self.skip_marks = self.store.skip_marks(&item.location).unwrap_or_default();
        self.events.push(SessionEvent::TrackChanged { index: self.queue.current_index().unwrap_or(0), location: item.location });
        if let Some(p) = resume_ms {
            self.events.push(SessionEvent::ResumeApplied { position_ms: p });
        }
        Ok(())
    }

    /// Legt de huidige positie vast (of markeert voltooid vlak voor het einde).
    fn save_position(&mut self) {
        let Some(loc) = self.current_location.clone() else {
            return;
        };
        let st = self.engine.status();
        if st.length_ms > 0 && matches!(st.state, PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Buffering) {
            self.store.save_resume(&loc, st.time_ms, st.length_ms);
        }
    }

    // ---------- bediening ----------

    pub fn toggle_pause(&mut self) -> Result<()> {
        match self.engine.status().state {
            PlaybackState::Playing | PlaybackState::Buffering => self.engine.pause(),
            PlaybackState::Paused => self.engine.play(),
            PlaybackState::Stopped | PlaybackState::Ended if self.current_location.is_some() => self.open_current(false),
            _ => Ok(()),
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        self.clear_countdown();
        self.save_position();
        self.ab.clear();
        self.engine.stop()
    }

    pub fn seek_to(&mut self, ms: u64) -> Result<()> {
        let len = self.engine.status().length_ms;
        let target = if len > 0 { ms.min(len) } else { ms };
        self.engine.seek(target)
    }

    pub fn seek_relative(&mut self, delta_ms: i64) -> Result<()> {
        let st = self.engine.status();
        let target = (st.time_ms as i64 + delta_ms).max(0) as u64;
        self.seek_to(target)
    }

    pub fn skip_next(&mut self) -> Result<()> {
        if self.pending_next.is_some() {
            // Tijdens de aftelling is het wachtende item al "het volgende": nu afspelen, niet een item overslaan.
            return self.open_current(false);
        }
        if self.queue.go_next().is_none() {
            return Ok(()); // einde van de lijst: niets te doen
        }
        self.open_current(false)
    }

    pub fn skip_previous(&mut self) -> Result<()> {
        // Zoals in VLC: na 3 seconden herstart "vorige" eerst het huidige nummer.
        if self.engine.status().time_ms > 3_000 {
            return self.engine.seek(0);
        }
        if self.queue.go_previous().is_none() {
            return self.engine.seek(0);
        }
        self.open_current(false)
    }

    pub fn set_repeat(&mut self, r: RepeatMode) {
        self.queue.set_repeat(r);
    }
    pub fn set_shuffle(&mut self, on: bool) {
        self.queue.set_shuffle(on);
    }

    pub fn set_volume(&mut self, percent: u32) -> Result<()> {
        self.engine.set_volume(percent.min(MAX_VOLUME))
    }
    pub fn change_volume(&mut self, delta: i32) -> Result<()> {
        let v = (self.engine.status().volume as i32 + delta).clamp(0, MAX_VOLUME as i32) as u32;
        self.engine.set_volume(v)
    }
    pub fn toggle_mute(&mut self) -> Result<()> {
        let m = self.engine.status().muted;
        self.engine.set_mute(!m)
    }

    pub fn set_rate(&mut self, rate: f32) -> Result<()> {
        if !rate.is_finite() || !(0.25..=4.0).contains(&rate) {
            return Err(CoreError::Invalid("De afspeelsnelheid moet tussen 0,25× en 4× liggen".into()));
        }
        self.engine.set_rate(rate)
    }

    /// Stap omhoog/omlaag in de vaste snelheidstabel.
    pub fn step_rate(&mut self, up: bool) -> Result<()> {
        let cur = self.engine.status().rate;
        let cur = if cur <= 0.0 { 1.0 } else { cur };
        let idx = RATE_STEPS.iter().position(|r| (r - cur).abs() < 0.01);
        let next = match (idx, up) {
            (Some(i), true) => RATE_STEPS[(i + 1).min(RATE_STEPS.len() - 1)],
            (Some(i), false) => RATE_STEPS[i.saturating_sub(1)],
            (None, true) => RATE_STEPS.iter().copied().find(|r| *r > cur).unwrap_or(4.0),
            (None, false) => RATE_STEPS.iter().rev().copied().find(|r| *r < cur).unwrap_or(0.25),
        };
        self.engine.set_rate(next)
    }

    pub fn cycle_audio(&mut self) -> Result<()> {
        let st = self.engine.status();
        match next_track(&st.audio_tracks, st.current_audio, false) {
            Some(id) => self.set_audio_track(id),
            None => Ok(()),
        }
    }

    pub fn cycle_subtitles(&mut self) -> Result<()> {
        let st = self.engine.status();
        let next = next_track(&st.subtitle_tracks, st.current_subtitle, true);
        self.set_subtitle_track(next)
    }

    /// Kiest een audiospoor en onthoudt die keuze voor dit bestand.
    pub fn set_audio_track(&mut self, id: i32) -> Result<()> {
        self.engine.set_audio_track(id)?;
        let key = self.engine.status().audio_tracks.iter().find(|t| t.id == id).map(track_key);
        if let (Some(loc), Some(key)) = (self.current_location.clone(), key) {
            let mut pref = self.store.track_pref(&loc).unwrap_or_default();
            pref.audio_key = Some(key);
            self.store.save_track_pref(&loc, &pref);
        }
        Ok(())
    }

    /// Kiest een ondertitelspoor (`None` = uit) en onthoudt die keuze voor dit bestand.
    pub fn set_subtitle_track(&mut self, id: Option<i32>) -> Result<()> {
        self.engine.set_subtitle_track(id)?;
        if let Some(loc) = self.current_location.clone() {
            let mut pref = self.store.track_pref(&loc).unwrap_or_default();
            match id {
                None => (pref.subtitle_off, pref.subtitle_key) = (true, None),
                Some(id) => {
                    pref.subtitle_off = false;
                    pref.subtitle_key = self.engine.status().subtitle_tracks.iter().find(|t| t.id == id).map(track_key);
                }
            }
            self.store.save_track_pref(&loc, &pref);
        }
        Ok(())
    }

    pub fn ab_press(&mut self) -> Result<()> {
        let t = self.engine.status().time_ms;
        self.ab.press(t)
    }

    pub fn frame_step(&mut self) -> Result<()> {
        if self.engine.status().state == PlaybackState::Playing {
            self.engine.pause()?;
        }
        self.engine.next_frame()
    }

    /// Zoekt een ondertitelbestand naast de video (zelfde naam, andere extensie).
    pub fn find_sidecar_subtitles(video: &Path) -> Vec<std::path::PathBuf> {
        let Some(stem) = video.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else {
            return vec![];
        };
        let Some(dir) = video.parent() else {
            return vec![];
        };
        let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                let name = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
                ["srt", "ass", "ssa", "vtt", "sub", "idx"].contains(&ext.as_str())
                    && (name == stem || name.starts_with(&format!("{stem}.")))
            })
            .collect();
        found.sort();
        found
    }

    // ---------- tikken ----------

    /// Wordt elke ~250 ms aangeroepen. `now_ms` is een stijgende klok in milliseconden.
    pub fn tick(&mut self, now_ms: u64) {
        let st = self.engine.status();
        match st.state {
            PlaybackState::Playing => {
                self.fail_streak = 0;
                if self.prefs_pending && (!st.audio_tracks.is_empty() || !st.subtitle_tracks.is_empty()) {
                    self.prefs_pending = false;
                    // Een eerdere keuze van de gebruiker voor dit bestand gaat vóór de algemene voorkeurstalen.
                    let saved = self.current_location.as_deref().and_then(|l| self.store.track_pref(l)).unwrap_or_default();
                    let audio = saved
                        .audio_key
                        .as_ref()
                        .and_then(|k| st.audio_tracks.iter().find(|t| track_key(t) == *k).map(|t| t.id))
                        .or_else(|| pick_track(&st.audio_tracks, &self.preferred_audio));
                    if let Some(id) = audio {
                        if st.current_audio != Some(id) {
                            let _ = self.engine.set_audio_track(id);
                        }
                    }
                    if saved.subtitle_off {
                        let _ = self.engine.set_subtitle_track(None);
                    } else {
                        let subs = saved
                            .subtitle_key
                            .as_ref()
                            .and_then(|k| st.subtitle_tracks.iter().find(|t| track_key(t) == *k).map(|t| t.id))
                            .or_else(|| pick_track(&st.subtitle_tracks, &self.preferred_subtitles));
                        if let Some(id) = subs {
                            let _ = self.engine.set_subtitle_track(Some(id));
                        }
                    }
                }
                if let Some(a) = self.ab.jump_target(st.time_ms) {
                    if self.engine.seek(a).is_ok() {
                        self.events.push(SessionEvent::AbLoopJump { to_ms: a });
                    }
                }
                if now_ms.saturating_sub(self.last_resume_save_ms) >= RESUME_SAVE_EVERY_MS {
                    self.last_resume_save_ms = now_ms;
                    self.save_position();
                }
            }
            PlaybackState::Ended => self.handle_end(now_ms),
            PlaybackState::Error => self.try_hw_fallback(st.time_ms),
            _ => {}
        }
    }

    /// Faalt het afspelen met hardwaredecodering (zwart beeld, driverfout), dan één keer opnieuw op dezelfde
    /// plek zonder. Per bestand maar één poging, zodat een echt kapot bestand niet blijft herstarten.
    fn try_hw_fallback(&mut self, at_ms: u64) {
        let Some(loc) = self.current_location.clone() else { return };
        if self.hw_fallback_done.as_deref() == Some(loc.as_str()) {
            return;
        }
        self.hw_fallback_done = Some(loc.clone());
        if !self.engine.disable_hw_decoding() {
            return;
        }
        let opts = OpenOptions { start_ms: at_ms, network_caching_ms: self.network_caching_ms, audio_visual: is_audio_location(&loc) };
        if self.engine.open(&loc, &opts).is_ok() && self.engine.play().is_ok() {
            self.prefs_pending = true;
            self.events.push(SessionEvent::HwFallback { location: loc });
        }
    }

    fn handle_end(&mut self, now_ms: u64) {
        // Het einde is al afgehandeld (de engine blijft in de stand "afgelopen" staan): niets meer te doen,
        // anders wordt "wachtrij afgelopen" elke tik opnieuw gemeld.
        if self.end_reported && self.pending_next.is_none() {
            return;
        }
        if let Some(deadline) = self.pending_next {
            if now_ms < deadline {
                self.next_in_ms = Some(deadline - now_ms);
                return;
            }
            self.clear_countdown();
            self.open_after_end(false);
            return;
        }
        self.end_reported = true;
        let finished = self.current_location.take();
        let was_video = finished.as_deref().is_some_and(|l| !is_audio_location(l));
        if let Some(loc) = finished {
            self.store.mark_completed(&loc);
        }
        // Na een video even wachten (zoals bij series), zodat je kunt afhaken of direct door kunt gaan.
        if was_video && self.next_delay_ms > 0 {
            if let Some(next) = self.queue.next_after_end().cloned() {
                self.current_location = Some(next.location);
                self.pending_next = Some(now_ms + self.next_delay_ms);
                self.next_in_ms = Some(self.next_delay_ms);
            } else {
                self.events.push(SessionEvent::QueueEnded);
            }
            return;
        }
        self.open_after_end(true);
    }

    /// Opent het volgende item na een einde. Met `advance = false` is de wachtrij al doorgeschoven.
    fn open_after_end(&mut self, mut advance: bool) {
        loop {
            if advance && self.queue.next_after_end().is_none() {
                self.events.push(SessionEvent::QueueEnded);
                return;
            }
            advance = true;
            match self.open_current(false) {
                Ok(()) => return,
                Err(_) => {
                    self.fail_streak += 1;
                    if self.fail_streak >= self.queue.len().max(1) {
                        self.events.push(SessionEvent::QueueEnded);
                        return; // alles in de lijst faalt: stop in plaats van eindeloos te proberen
                    }
                }
            }
        }
    }

    fn clear_countdown(&mut self) {
        self.pending_next = None;
        self.next_in_ms = None;
    }

    /// Stopt de aftelling; het wachtende item blijft klaarstaan en start met de afspeelknop.
    pub fn cancel_autoplay(&mut self) -> Result<()> {
        if self.pending_next.is_none() {
            return Ok(());
        }
        self.clear_countdown();
        // Stoppen zet de engine uit de stand "afgelopen"; anders telt de volgende tik het wachtende item als afgespeeld.
        self.engine.stop()
    }

    /// Bij afsluiten van de app: positie vastleggen en afspelen stoppen.
    pub fn shutdown(&mut self) {
        self.save_position();
        let _ = self.engine.stop();
    }
}

#[cfg(test)]
mod tests;
