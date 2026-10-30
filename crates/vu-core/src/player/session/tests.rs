use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct EngineInner {
    status: EngineStatus,
    opened: Vec<(String, OpenOptions)>,
    fail: HashSet<String>,
    calls: Vec<String>,
}

#[derive(Clone, Default)]
struct FakeEngine(Arc<Mutex<EngineInner>>);

impl FakeEngine {
    fn with<R>(&self, f: impl FnOnce(&mut EngineInner) -> R) -> R {
        f(&mut self.0.lock().unwrap())
    }
    fn calls(&self) -> Vec<String> {
        self.with(|i| i.calls.clone())
    }
    fn set_state(&self, state: PlaybackState, time_ms: u64, length_ms: u64) {
        self.with(|i| {
            i.status.state = state;
            i.status.time_ms = time_ms;
            i.status.length_ms = length_ms;
        });
    }
}

impl Engine for FakeEngine {
    fn open(&mut self, location: &str, opts: &OpenOptions) -> Result<()> {
        self.with(|i| {
            i.calls.push(format!("open {location}"));
            if i.fail.contains(location) {
                return Err(CoreError::Invalid(format!("kan {location} niet openen")));
            }
            i.opened.push((location.to_string(), opts.clone()));
            i.status.state = PlaybackState::Opening;
            i.status.time_ms = opts.start_ms;
            i.status.length_ms = 100_000;
            Ok(())
        })
    }
    fn play(&mut self) -> Result<()> {
        self.with(|i| {
            i.calls.push("play".into());
            i.status.state = PlaybackState::Playing;
        });
        Ok(())
    }
    fn pause(&mut self) -> Result<()> {
        self.with(|i| {
            i.calls.push("pause".into());
            i.status.state = PlaybackState::Paused;
        });
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.with(|i| {
            i.calls.push("stop".into());
            i.status.state = PlaybackState::Stopped;
        });
        Ok(())
    }
    fn seek(&mut self, ms: u64) -> Result<()> {
        self.with(|i| {
            i.calls.push(format!("seek {ms}"));
            i.status.time_ms = ms;
        });
        Ok(())
    }
    fn set_volume(&mut self, p: u32) -> Result<()> {
        self.with(|i| {
            i.calls.push(format!("volume {p}"));
            i.status.volume = p;
        });
        Ok(())
    }
    fn set_mute(&mut self, m: bool) -> Result<()> {
        self.with(|i| i.status.muted = m);
        Ok(())
    }
    fn set_rate(&mut self, r: f32) -> Result<()> {
        self.with(|i| i.status.rate = r);
        Ok(())
    }
    fn set_audio_track(&mut self, id: i32) -> Result<()> {
        self.with(|i| {
            i.calls.push(format!("audio {id}"));
            i.status.current_audio = Some(id);
        });
        Ok(())
    }
    fn set_subtitle_track(&mut self, id: Option<i32>) -> Result<()> {
        self.with(|i| {
            i.calls.push(format!("sub {id:?}"));
            i.status.current_subtitle = id;
        });
        Ok(())
    }
    fn load_subtitle_file(&mut self, _p: &Path) -> Result<()> {
        Ok(())
    }
    fn set_audio_delay_ms(&mut self, _ms: i64) -> Result<()> {
        Ok(())
    }
    fn set_subtitle_delay_ms(&mut self, _ms: i64) -> Result<()> {
        Ok(())
    }
    fn next_frame(&mut self) -> Result<()> {
        self.with(|i| i.calls.push("next_frame".into()));
        Ok(())
    }
    fn take_snapshot(&mut self, _p: &Path) -> Result<()> {
        Ok(())
    }
    fn set_equalizer(&mut self, _eq: Option<&EqConfig>) -> Result<()> {
        Ok(())
    }
    fn status(&self) -> EngineStatus {
        self.with(|i| i.status.clone())
    }
}

#[derive(Default)]
struct StoreInner {
    resumes: HashMap<String, ResumePoint>,
    saves: Vec<(String, u64, u64)>,
    completed: Vec<String>,
    plays: Vec<String>,
    prefs: HashMap<String, TrackPref>,
    marks: HashMap<String, SkipMarks>,
}

#[derive(Clone, Default)]
struct FakeStore(Arc<Mutex<StoreInner>>);

impl Persistence for FakeStore {
    fn resume(&self, l: &str) -> Option<ResumePoint> {
        self.0.lock().unwrap().resumes.get(l).cloned()
    }
    fn save_resume(&self, l: &str, p: u64, d: u64) {
        self.0.lock().unwrap().saves.push((l.into(), p, d));
    }
    fn mark_completed(&self, l: &str) {
        self.0.lock().unwrap().completed.push(l.into());
    }
    fn record_play(&self, l: &str, _t: Option<&str>) {
        self.0.lock().unwrap().plays.push(l.into());
    }
    fn track_pref(&self, l: &str) -> Option<TrackPref> {
        self.0.lock().unwrap().prefs.get(l).cloned()
    }
    fn save_track_pref(&self, l: &str, p: &TrackPref) {
        self.0.lock().unwrap().prefs.insert(l.into(), p.clone());
    }
    fn skip_marks(&self, l: &str) -> Option<SkipMarks> {
        self.0.lock().unwrap().marks.get(l).cloned()
    }
    fn save_skip_marks(&self, l: &str, m: &SkipMarks) {
        self.0.lock().unwrap().marks.insert(l.into(), m.clone());
    }
}

fn items(names: &[&str]) -> Vec<QueueItem> {
    names.iter().map(|n| QueueItem { location: format!("/m/{n}.mp3"), title: Some(n.to_string()) }).collect()
}

fn setup(names: &[&str]) -> (Session<FakeEngine>, FakeEngine, FakeStore) {
    let engine = FakeEngine::default();
    let store = FakeStore::default();
    let mut s = Session::new(engine.clone(), Box::new(store.clone()), 1);
    if !names.is_empty() {
        s.play_items(items(names), 0).unwrap();
        s.take_events();
    }
    (s, engine, store)
}

fn resume_point(loc: &str, pos: u64, completed: bool) -> ResumePoint {
    ResumePoint { location: loc.into(), position_ms: pos, duration_ms: 100_000, completed, updated_at: 0 }
}

#[test]
fn play_items_opens_the_first_item_starts_playback_and_records_history() {
    let engine = FakeEngine::default();
    let store = FakeStore::default();
    let mut s = Session::new(engine.clone(), Box::new(store.clone()), 1);
    s.play_items(items(&["a", "b"]), 0).unwrap();
    assert_eq!(engine.calls(), ["open /m/a.mp3", "play"]);
    assert_eq!(store.0.lock().unwrap().plays, ["/m/a.mp3"]);
    assert_eq!(s.take_events(), vec![SessionEvent::TrackChanged { index: 0, location: "/m/a.mp3".into() }]);
    assert!(engine.with(|i| i.opened[0].1.audio_visual)); // mp3: spectrum aan
                                                          // een lege lijst afspelen is een duidelijke fout, geen stille no-op
    let err = s.play_items(vec![], 0).unwrap_err().to_string();
    assert!(err.contains("wachtrij is leeg"), "{err}");
    assert!(s.queue().is_empty());
    assert!(s.play_index(0).is_err());
}

#[test]
fn resume_applies_only_for_unfinished_meaningful_positions() {
    for (pos, completed, enabled, expected) in
        [(30_000, false, true, 30_000), (30_000, true, true, 0), (3_000, false, true, 0), (30_000, false, false, 0)]
    {
        let engine = FakeEngine::default();
        let store = FakeStore::default();
        store.0.lock().unwrap().resumes.insert("/m/a.mp3".into(), resume_point("/m/a.mp3", pos, completed));
        let mut s = Session::new(engine.clone(), Box::new(store), 1);
        s.resume_enabled = enabled;
        s.play_items(items(&["a"]), 0).unwrap();
        assert_eq!(engine.with(|i| i.opened[0].1.start_ms), expected, "pos={pos} completed={completed} enabled={enabled}");
        let has_event = s.take_events().iter().any(|e| matches!(e, SessionEvent::ResumeApplied { .. }));
        assert_eq!(has_event, expected > 0);
    }
}

#[test]
fn natural_end_marks_completed_and_advances_without_resuming() {
    let (mut s, engine, store) = setup(&["a", "b"]);
    store.0.lock().unwrap().resumes.insert("/m/b.mp3".into(), resume_point("/m/b.mp3", 40_000, false));
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(1000);
    assert_eq!(store.0.lock().unwrap().completed, ["/m/a.mp3"]);
    assert_eq!(engine.with(|i| i.opened.last().unwrap().0.clone()), "/m/b.mp3");
    assert_eq!(engine.with(|i| i.opened.last().unwrap().1.start_ms), 0); // automatisch doorgaan = vanaf het begin
    assert!(matches!(s.take_events().last(), Some(SessionEvent::TrackChanged { index: 1, .. })));
}

#[test]
fn end_of_the_list_stops_and_repeat_modes_continue() {
    let (mut s, engine, _) = setup(&["a"]);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(0);
    assert_eq!(s.take_events(), vec![SessionEvent::QueueEnded]);

    let (mut s, engine, _) = setup(&["a", "b"]);
    s.set_repeat(RepeatMode::All);
    s.queue.jump_to(1).unwrap();
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(0);
    assert_eq!(engine.with(|i| i.opened.last().unwrap().0.clone()), "/m/a.mp3"); // terug naar het begin

    let (mut s, engine, _) = setup(&["a", "b"]);
    s.set_repeat(RepeatMode::One);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(0);
    assert_eq!(engine.with(|i| i.opened.last().unwrap().0.clone()), "/m/a.mp3");
    assert_eq!(engine.with(|i| i.opened.len()), 2);
}

#[test]
fn a_failing_item_is_reported_and_skipped_during_auto_advance() {
    let (mut s, engine, _) = setup(&["a", "bad", "c"]);
    engine.with(|i| i.fail.insert("/m/bad.mp3".into()));
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(0);
    let ev = s.take_events();
    assert!(ev.iter().any(|e| matches!(e, SessionEvent::OpenFailed { location, .. } if location == "/m/bad.mp3")));
    assert_eq!(engine.with(|i| i.opened.last().unwrap().0.clone()), "/m/c.mp3");
    // doorgegaan naar c
}

#[test]
fn a_list_where_everything_fails_ends_instead_of_looping_forever() {
    let engine = FakeEngine::default();
    engine.with(|i| {
        i.fail.insert("/m/x.mp3".into());
        i.fail.insert("/m/y.mp3".into());
    });
    let mut s = Session::new(engine.clone(), Box::new(FakeStore::default()), 1);
    assert!(s.play_items(items(&["x", "y"]), 0).is_err());
    s.take_events();
    s.set_repeat(RepeatMode::All); // gevaarlijkste geval: herhalen
    engine.set_state(PlaybackState::Ended, 0, 0);
    s.tick(0);
    assert!(s.take_events().contains(&SessionEvent::QueueEnded));
    assert!(engine.calls().len() < 20, "te veel pogingen: {}", engine.calls().len());
}

#[test]
fn ab_loop_jumps_back_and_reports_it() {
    let (mut s, engine, _) = setup(&["a"]);
    engine.set_state(PlaybackState::Playing, 10_000, 100_000);
    s.ab_press().unwrap();
    engine.set_state(PlaybackState::Playing, 20_000, 100_000);
    s.ab_press().unwrap();
    engine.set_state(PlaybackState::Playing, 20_400, 100_000);
    s.tick(0);
    assert_eq!(engine.with(|i| i.status.time_ms), 10_000);
    assert_eq!(s.take_events(), vec![SessionEvent::AbLoopJump { to_ms: 10_000 }]);
    assert_eq!((s.view().ab_a_ms, s.view().ab_b_ms), (Some(10_000), Some(20_000)));
    s.ab_press().unwrap(); // derde druk wist de lus
    assert_eq!(s.view().ab_a_ms, None);
    // de lus verdwijnt ook bij een nieuw nummer
    s.ab_press().unwrap();
    s.play_items(items(&["b"]), 0).unwrap();
    assert_eq!(s.view().ab_a_ms, None);
}

#[test]
fn position_is_saved_every_five_seconds_while_playing_only() {
    let (mut s, engine, store) = setup(&["a"]);
    engine.set_state(PlaybackState::Playing, 12_000, 100_000);
    s.tick(5_000);
    s.tick(6_000);
    s.tick(9_000);
    s.tick(10_000);
    assert_eq!(store.0.lock().unwrap().saves.iter().filter(|x| x.1 == 12_000).count(), 2);
    engine.set_state(PlaybackState::Paused, 13_000, 100_000);
    s.tick(20_000);
    assert_eq!(store.0.lock().unwrap().saves.len(), 2); // pauze: niet opnieuw opslaan
    engine.set_state(PlaybackState::Playing, 14_000, 0); // onbekende lengte
    s.tick(30_000);
    assert_eq!(store.0.lock().unwrap().saves.len(), 2);
}

#[test]
fn stop_and_shutdown_save_the_position() {
    let (mut s, engine, store) = setup(&["a"]);
    engine.set_state(PlaybackState::Playing, 33_000, 100_000);
    s.stop().unwrap();
    assert!(store.0.lock().unwrap().saves.iter().any(|x| x.1 == 33_000));
    let (mut s2, engine2, store2) = setup(&["a"]);
    engine2.set_state(PlaybackState::Paused, 44_000, 100_000);
    s2.shutdown();
    assert!(store2.0.lock().unwrap().saves.iter().any(|x| x.1 == 44_000));
    assert!(engine2.calls().contains(&"stop".to_string()));
}

#[test]
fn seeking_is_clamped_to_the_media() {
    let (mut s, engine, _) = setup(&["a"]);
    engine.set_state(PlaybackState::Playing, 5_000, 100_000);
    s.seek_relative(-30_000).unwrap();
    assert_eq!(engine.with(|i| i.status.time_ms), 0);
    s.seek_to(999_999).unwrap();
    assert_eq!(engine.with(|i| i.status.time_ms), 100_000);
    s.seek_relative(10_000).unwrap();
    assert_eq!(engine.with(|i| i.status.time_ms), 100_000);
}

#[test]
fn previous_restarts_after_three_seconds_otherwise_goes_back() {
    let (mut s, engine, _) = setup(&["a", "b"]);
    s.skip_next().unwrap();
    engine.set_state(PlaybackState::Playing, 10_000, 100_000);
    s.skip_previous().unwrap();
    assert_eq!(engine.with(|i| i.status.time_ms), 0); // herstart b
    assert_eq!(s.queue().current().unwrap().location, "/m/b.mp3");
    engine.set_state(PlaybackState::Playing, 1_000, 100_000);
    s.skip_previous().unwrap();
    assert_eq!(s.queue().current().unwrap().location, "/m/a.mp3");
    engine.set_state(PlaybackState::Playing, 1_000, 100_000);
    s.skip_previous().unwrap(); // al op het eerste item: herstart
    assert_eq!(s.queue().current().unwrap().location, "/m/a.mp3");
    assert_eq!(engine.with(|i| i.status.time_ms), 0);
    s.skip_next().unwrap();
    s.skip_next().unwrap(); // voorbij het einde: geen fout, geen wijziging
    assert_eq!(s.queue().current().unwrap().location, "/m/b.mp3");
}

#[test]
fn toggle_pause_covers_all_states() {
    let (mut s, engine, _) = setup(&["a"]);
    engine.set_state(PlaybackState::Playing, 1000, 100_000);
    s.toggle_pause().unwrap();
    assert_eq!(engine.with(|i| i.status.state), PlaybackState::Paused);
    s.toggle_pause().unwrap();
    assert_eq!(engine.with(|i| i.status.state), PlaybackState::Playing);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.toggle_pause().unwrap(); // opnieuw afspelen vanaf het begin
    assert_eq!(engine.with(|i| i.opened.len()), 2);
    let (mut idle, _, _) = setup(&[]);
    assert!(idle.toggle_pause().is_ok()); // niets geladen: geen fout
}

#[test]
fn volume_mute_and_rate_are_validated() {
    let (mut s, engine, _) = setup(&["a"]);
    s.set_volume(500).unwrap();
    assert_eq!(engine.with(|i| i.status.volume), 200);
    s.change_volume(-300).unwrap();
    assert_eq!(engine.with(|i| i.status.volume), 0);
    s.toggle_mute().unwrap();
    assert!(engine.with(|i| i.status.muted));
    for bad in [0.0, 0.1, 10.0, f32::NAN, -1.0] {
        assert!(s.set_rate(bad).is_err(), "{bad}");
    }
    s.set_rate(1.5).unwrap();
    assert_eq!(engine.with(|i| i.status.rate), 1.5);
}

#[test]
fn rate_steps_walk_through_the_table_and_stop_at_the_ends() {
    let (mut s, engine, _) = setup(&["a"]);
    engine.with(|i| i.status.rate = 1.0);
    s.step_rate(true).unwrap();
    assert_eq!(engine.with(|i| i.status.rate), 1.25);
    for _ in 0..10 {
        s.step_rate(true).unwrap();
    }
    assert_eq!(engine.with(|i| i.status.rate), 4.0);
    for _ in 0..20 {
        s.step_rate(false).unwrap();
    }
    assert_eq!(engine.with(|i| i.status.rate), 0.25);
    engine.with(|i| i.status.rate = 1.1); // niet in de tabel
    s.step_rate(true).unwrap();
    assert_eq!(engine.with(|i| i.status.rate), 1.25);
}

fn track(id: i32, name: &str, lang: Option<&str>) -> Track {
    Track { id, name: name.into(), language: lang.map(String::from) }
}

#[test]
fn audio_and_subtitle_cycling_wraps_and_subtitles_include_off() {
    let (mut s, engine, _) = setup(&["a"]);
    engine.with(|i| {
        i.status.audio_tracks = vec![track(1, "A", None), track(2, "B", None)];
        i.status.current_audio = Some(2);
        i.status.subtitle_tracks = vec![track(10, "S1", None), track(11, "S2", None)];
        i.status.current_subtitle = None;
    });
    s.cycle_audio().unwrap();
    assert_eq!(engine.with(|i| i.status.current_audio), Some(1)); // wrap
    s.cycle_subtitles().unwrap();
    assert_eq!(engine.with(|i| i.status.current_subtitle), Some(10));
    s.cycle_subtitles().unwrap();
    s.cycle_subtitles().unwrap();
    assert_eq!(engine.with(|i| i.status.current_subtitle), None); // weer uit
    engine.with(|i| i.status.audio_tracks.clear());
    assert!(s.cycle_audio().is_ok()); // geen sporen: geen fout
}

#[test]
fn preferred_languages_are_applied_once_when_playback_starts() {
    let (mut s, engine, _) = setup(&["a"]);
    s.preferred_audio = vec!["nld".into(), "eng".into()];
    s.preferred_subtitles = vec!["nld".into()];
    s.play_items(items(&["a"]), 0).unwrap();
    engine.with(|i| {
        i.status.audio_tracks = vec![track(1, "Track 1 - [English]", None), track(2, "Track 2 - [Nederlands]", None)];
        i.status.current_audio = Some(1);
        i.status.subtitle_tracks = vec![track(5, "Subs", Some("eng")), track(6, "Ondertitels", Some("nld"))];
    });
    s.tick(0);
    assert_eq!(engine.with(|i| i.status.current_audio), Some(2));
    assert_eq!(engine.with(|i| i.status.current_subtitle), Some(6));
    // de gebruiker kiest zelf iets anders: dat blijft staan
    engine.with(|i| i.status.current_audio = Some(1));
    s.tick(1);
    assert_eq!(engine.with(|i| i.status.current_audio), Some(1));
}

#[test]
fn language_guessing_and_track_picking() {
    assert_eq!(guess_language("Track 2 - [Nederlands]"), Some("nld"));
    assert_eq!(guess_language("English 5.1"), Some("eng"));
    assert_eq!(guess_language("Deutsch"), Some("deu"));
    assert_eq!(guess_language("Commentary"), None);
    let tracks = vec![track(1, "x", Some("fra")), track(2, "y", Some("eng"))];
    assert_eq!(pick_track(&tracks, &["nld".into(), "eng".into()]), Some(2)); // tweede voorkeur
    assert_eq!(pick_track(&tracks, &["nld".into()]), None);
    assert_eq!(pick_track(&[], &["nld".into()]), None);
}

#[test]
fn frame_step_pauses_first() {
    let (mut s, engine, _) = setup(&["a"]);
    engine.set_state(PlaybackState::Playing, 1000, 100_000);
    s.frame_step().unwrap();
    let calls = engine.calls();
    let pause = calls.iter().rposition(|c| c == "pause").unwrap();
    assert!(calls.iter().rposition(|c| c == "next_frame").unwrap() > pause);
}

#[test]
fn sidecar_subtitles_are_found_next_to_the_video() {
    let dir = tempfile::tempdir().unwrap();
    for f in ["film.mkv", "film.srt", "film.nl.srt", "andere.srt", "film.txt", "film2.srt", "FILM.ASS"] {
        std::fs::write(dir.path().join(f), b"x").unwrap();
    }
    let found = Session::<FakeEngine>::find_sidecar_subtitles(&dir.path().join("film.mkv"));
    let names: Vec<String> = found.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["FILM.ASS", "film.nl.srt", "film.srt"]);
}

#[test]
fn network_caching_and_enqueue_do_not_interrupt_playback() {
    let engine = FakeEngine::default();
    let mut s = Session::new(engine.clone(), Box::new(FakeStore::default()), 1);
    s.network_caching_ms = Some(3000);
    s.play_items(items(&["a"]), 0).unwrap();
    assert_eq!(engine.with(|i| i.opened[0].1.network_caching_ms), Some(3000));
    let calls_before = engine.calls().len();
    assert_eq!(s.enqueue(items(&["b", "c"])), 1);
    assert_eq!(engine.calls().len(), calls_before);
    let v = s.view();
    assert_eq!((v.queue_index, v.queue_len), (Some(0), 3));
}

#[test]
fn next_track_helper_handles_edge_cases() {
    assert_eq!(next_track(&[], None, true), None);
    assert_eq!(next_track(&[], None, false), None);
    let t = vec![track(1, "a", None)];
    assert_eq!(next_track(&t, None, true), Some(1));
    assert_eq!(next_track(&t, Some(1), true), None);
    assert_eq!(next_track(&t, Some(99), false), Some(1)); // onbekend huidig spoor: begin opnieuw
}

#[test]
fn removing_the_playing_item_plays_the_next_one_and_removing_others_does_not_interrupt() {
    let (mut s, engine, _) = setup(&["a", "b", "c"]);
    let opens_before = engine.with(|i| i.opened.len());
    s.queue_remove(2).unwrap(); // niet het spelende item
    assert_eq!(engine.with(|i| i.opened.len()), opens_before);
    assert_eq!(s.queue().len(), 2);
    s.queue_remove(0).unwrap(); // het spelende item: b neemt het over
    assert_eq!(engine.with(|i| i.opened.last().unwrap().0.clone()), "/m/b.mp3");
    assert_eq!(s.queue().current().unwrap().location, "/m/b.mp3");
    s.queue_remove(0).unwrap(); // laatste item: speler stopt
    assert!(s.queue().is_empty());
    assert_eq!(engine.with(|i| i.status.state), PlaybackState::Stopped);
    assert!(s.queue_remove(5).is_err());
}

#[test]
fn removing_the_last_playing_item_of_a_longer_list_falls_back_to_the_new_last() {
    let (mut s, engine, _) = setup(&["a", "b", "c"]);
    s.play_index(2).unwrap();
    s.queue_remove(2).unwrap();
    assert_eq!(engine.with(|i| i.opened.last().unwrap().0.clone()), "/m/b.mp3");
}

#[test]
fn queue_move_and_clear_work_and_clear_stops_playback() {
    let (mut s, engine, store) = setup(&["a", "b", "c"]);
    s.queue_move(0, 2).unwrap();
    assert_eq!(s.queue().items()[2].location, "/m/a.mp3");
    assert_eq!(s.queue().current().unwrap().location, "/m/a.mp3"); // het spelende nummer blijft hetzelfde
    engine.set_state(PlaybackState::Playing, 12_000, 100_000);
    s.queue_clear().unwrap();
    assert!(s.queue().is_empty());
    assert_eq!(engine.with(|i| i.status.state), PlaybackState::Stopped);
    assert!(store.0.lock().unwrap().saves.iter().any(|x| x.1 == 12_000));
    assert!(s.queue_move(0, 1).is_err());
}

// ---------- automatisch volgende aflevering met aftelling ----------

fn video_setup(names: &[&str], delay_ms: u64) -> (Session<FakeEngine>, FakeEngine, FakeStore) {
    let engine = FakeEngine::default();
    let store = FakeStore::default();
    let mut s = Session::new(engine.clone(), Box::new(store.clone()), 1);
    s.next_delay_ms = delay_ms;
    let list = names.iter().map(|n| QueueItem { location: format!("/m/{n}.mkv"), title: None }).collect();
    s.play_items(list, 0).unwrap();
    s.take_events();
    (s, engine, store)
}

fn opened(engine: &FakeEngine) -> Vec<String> {
    engine.with(|i| i.opened.iter().map(|o| o.0.clone()).collect())
}

#[test]
fn a_finished_video_counts_down_before_the_next_item_starts() {
    let (mut s, engine, store) = video_setup(&["a", "b"], 10_000);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(1_000);
    assert_eq!(opened(&engine), ["/m/a.mkv"], "tijdens het aftellen start er niets");
    assert_eq!(store.0.lock().unwrap().completed, ["/m/a.mkv"]);
    let v = s.view();
    assert_eq!((v.next_in_ms, v.queue_index, v.location.as_deref()), (Some(10_000), Some(1), Some("/m/b.mkv")));
    s.tick(6_000);
    assert_eq!(s.view().next_in_ms, Some(5_000));
    assert_eq!(opened(&engine), ["/m/a.mkv"]);
    s.tick(11_000);
    assert_eq!(opened(&engine), ["/m/a.mkv", "/m/b.mkv"]);
    assert_eq!(s.view().next_in_ms, None);
    assert!(matches!(s.take_events().last(), Some(SessionEvent::TrackChanged { index: 1, .. })));
}

#[test]
fn music_and_a_zero_delay_advance_immediately() {
    let (mut s, engine, _) = setup(&["a", "b"]); // muziek
    s.next_delay_ms = 10_000;
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(0);
    assert_eq!(opened(&engine), ["/m/a.mp3", "/m/b.mp3"]);

    let (mut s, engine, _) = video_setup(&["a", "b"], 0);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(0);
    assert_eq!(opened(&engine), ["/m/a.mkv", "/m/b.mkv"]);
}

#[test]
fn next_during_the_countdown_plays_the_waiting_item_now_and_does_not_skip_one() {
    let (mut s, engine, _) = video_setup(&["a", "b", "c"], 10_000);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(1_000);
    s.skip_next().unwrap();
    assert_eq!(opened(&engine), ["/m/a.mkv", "/m/b.mkv"]);
    assert_eq!(s.view().next_in_ms, None);
    assert_eq!(s.view().queue_index, Some(1));
}

#[test]
fn cancelling_keeps_the_next_item_ready_but_starts_nothing() {
    let (mut s, engine, _) = video_setup(&["a", "b"], 10_000);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(1_000);
    s.cancel_autoplay().unwrap();
    assert_eq!(s.view().next_in_ms, None);
    s.tick(60_000);
    assert_eq!(opened(&engine), ["/m/a.mkv"], "na annuleren start er niets uit zichzelf");
    s.toggle_pause().unwrap(); // afspelen-knop start het klaargezette item
    assert_eq!(opened(&engine), ["/m/a.mkv", "/m/b.mkv"]);
}

#[test]
fn the_last_video_ends_the_queue_without_a_countdown() {
    let (mut s, engine, _) = video_setup(&["a"], 10_000);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(0);
    assert_eq!(s.take_events(), vec![SessionEvent::QueueEnded]);
    assert_eq!(s.view().next_in_ms, None);
}

#[test]
fn stopping_cancels_a_running_countdown() {
    let (mut s, engine, _) = video_setup(&["a", "b"], 10_000);
    engine.set_state(PlaybackState::Ended, 100_000, 100_000);
    s.tick(1_000);
    s.stop().unwrap();
    assert_eq!(s.view().next_in_ms, None);
    s.tick(60_000);
    assert_eq!(opened(&engine), ["/m/a.mkv"]);
}

// ---------- spoorkeuze per bestand onthouden ----------

fn with_tracks(engine: &FakeEngine) {
    engine.with(|i| {
        i.status.state = PlaybackState::Playing;
        i.status.audio_tracks = vec![track(1, "Track 1 - [Nederlands]", None), track(2, "Track 2 - [English]", None)];
        i.status.current_audio = Some(1);
        i.status.subtitle_tracks = vec![track(10, "Nederlands", None), track(11, "English", None)];
        i.status.current_subtitle = None;
    });
}

#[test]
fn a_manual_track_choice_is_remembered_and_applied_when_the_file_is_opened_again() {
    let (mut s, engine, store) = setup(&["a"]);
    s.preferred_audio = vec!["nld".into()];
    with_tracks(&engine);
    s.tick(1_000); // eerste keer: voorkeurstaal Nederlands
    assert_eq!(engine.with(|i| i.status.current_audio), Some(1));
    s.set_audio_track(2).unwrap(); // gebruiker kiest Engels
    s.set_subtitle_track(Some(10)).unwrap();
    assert_eq!(
        store.0.lock().unwrap().prefs["/m/a.mp3"],
        TrackPref { audio_key: Some("eng".into()), subtitle_key: Some("nld".into()), subtitle_off: false }
    );

    s.play_items(items(&["a"]), 0).unwrap(); // zelfde bestand opnieuw
    with_tracks(&engine);
    s.tick(2_000);
    assert_eq!(engine.with(|i| i.status.current_audio), Some(2), "Engels onthouden, ondanks voorkeur Nederlands");
    assert_eq!(engine.with(|i| i.status.current_subtitle), Some(10));
}

#[test]
fn turning_subtitles_off_is_remembered_too_and_beats_the_preferred_language() {
    let (mut s, engine, _) = setup(&["a"]);
    s.preferred_subtitles = vec!["nld".into()];
    with_tracks(&engine);
    s.tick(1_000);
    assert_eq!(engine.with(|i| i.status.current_subtitle), Some(10)); // voorkeur geldt eerst
    s.set_subtitle_track(None).unwrap();
    s.play_items(items(&["a"]), 0).unwrap();
    with_tracks(&engine);
    s.tick(2_000);
    assert_eq!(engine.with(|i| i.status.current_subtitle), None, "uit blijft uit");
}

#[test]
fn cycling_tracks_counts_as_a_choice_and_other_files_are_not_affected() {
    let (mut s, engine, store) = setup(&["a", "b"]);
    with_tracks(&engine);
    s.cycle_audio().unwrap(); // 1 -> 2
    assert_eq!(store.0.lock().unwrap().prefs["/m/a.mp3"].audio_key.as_deref(), Some("eng"));
    s.skip_next().unwrap(); // ander bestand: geen opgeslagen keuze, dus voorkeurstaal-logica
    with_tracks(&engine);
    s.preferred_audio = vec!["nld".into()];
    s.tick(5_000);
    assert_eq!(engine.with(|i| i.status.current_audio), Some(1));
    assert!(!store.0.lock().unwrap().prefs.contains_key("/m/b.mp3"));
}

#[test]
fn choices_are_matched_by_language_or_by_name_when_the_language_is_unknown() {
    let (mut s, engine, store) = setup(&["a"]);
    engine.with(|i| {
        i.status.state = PlaybackState::Playing;
        i.status.audio_tracks = vec![track(1, "Commentaar van de regisseur", None), track(2, "Hoofdgeluid", None)];
        i.status.current_audio = Some(2);
    });
    s.set_audio_track(1).unwrap();
    assert_eq!(store.0.lock().unwrap().prefs["/m/a.mp3"].audio_key.as_deref(), Some("commentaar van de regisseur"));
    s.play_items(items(&["a"]), 0).unwrap();
    engine.with(|i| {
        i.status.state = PlaybackState::Playing;
        i.status.audio_tracks = vec![track(7, "Hoofdgeluid", None), track(8, "Commentaar van de regisseur", None)];
        i.status.current_audio = Some(7);
    });
    s.tick(1_000);
    assert_eq!(engine.with(|i| i.status.current_audio), Some(8), "gevonden op naam, ook al zijn de nummers anders");
}

fn at(engine: &FakeEngine, ms: u64) {
    engine.with(|i| {
        i.status.state = PlaybackState::Playing;
        i.status.time_ms = ms;
        i.status.length_ms = 100_000;
    });
}

#[test]
fn intro_marks_are_saved_per_file_and_offer_a_skip_only_inside_the_intro() {
    let (mut s, engine, store) = setup(&["a"]);
    at(&engine, 5_000);
    s.mark_intro_start().unwrap();
    at(&engine, 35_000);
    s.mark_intro_end().unwrap();
    assert_eq!(
        store.0.lock().unwrap().marks["/m/a.mp3"],
        SkipMarks { intro_start_ms: Some(5_000), intro_end_ms: Some(35_000), credits_start_ms: None }
    );
    at(&engine, 10_000);
    assert_eq!(s.view().skip_offer, Some(SkipOffer::Intro));
    at(&engine, 4_000);
    assert_eq!(s.view().skip_offer, None);
    at(&engine, 35_000);
    assert_eq!(s.view().skip_offer, None);
}

#[test]
fn skipping_the_intro_seeks_to_its_end_and_remembered_marks_return_for_the_same_file() {
    let (mut s, engine, _) = setup(&["a"]);
    at(&engine, 0);
    s.mark_intro_start().unwrap();
    at(&engine, 40_000);
    s.mark_intro_end().unwrap();
    s.play_items(items(&["a"]), 0).unwrap();
    at(&engine, 12_000);
    s.skip_now().unwrap();
    assert!(
        engine.with(|i| i.calls.iter().any(|c| c.contains("40000"))),
        "gesprongen naar het einde van de intro: {:?}",
        engine.with(|i| i.calls.clone())
    );
}

#[test]
fn credits_offer_next_and_skipping_goes_to_the_next_item() {
    let (mut s, engine, _) = setup(&["a", "b"]);
    at(&engine, 90_000);
    s.mark_credits_start().unwrap();
    at(&engine, 80_000);
    assert_eq!(s.view().skip_offer, None);
    at(&engine, 91_000);
    assert_eq!(s.view().skip_offer, Some(SkipOffer::Credits));
    s.skip_now().unwrap();
    assert_eq!(s.view().location.as_deref(), Some("/m/b.mp3"));
}

#[test]
fn an_end_before_the_start_is_refused_and_marks_can_be_cleared() {
    let (mut s, engine, store) = setup(&["a"]);
    at(&engine, 30_000);
    s.mark_intro_start().unwrap();
    at(&engine, 10_000);
    assert!(s.mark_intro_end().is_err());
    s.clear_skip_marks();
    assert_eq!(store.0.lock().unwrap().marks["/m/a.mp3"], SkipMarks::default());
    assert_eq!(s.view().skip_offer, None);
}
