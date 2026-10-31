//! Integratietests tegen de echte LibVLC. Zonder geïnstalleerde LibVLC worden ze overgeslagen.
//! Geluid en beeld gaan naar "dummy"-uitvoer, dus er is geen scherm of geluidskaart nodig.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use vu_core::audio::EqConfig;
use vu_core::db::Database;
use vu_core::error::CoreError;
use vu_core::player::queue::{QueueItem, RepeatMode};
use vu_core::player::session::{Engine, EngineStatus, OpenOptions, PlaybackState, Session, SessionEvent};
use vu_vlc::{VlcConfig, VlcEngine};

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../vu-core/tests/fixtures").join(name).to_string_lossy().into_owned()
}

fn engine() -> Option<VlcEngine> {
    let mut args = vec!["--aout=dummy".to_string(), "--vout=dummy".to_string()];
    #[cfg(not(windows))]
    args.push("--no-xlib".into());
    // In de Windows-CI wijst VU_VLC_DIR naar de map met libvlc.dll en plugins
    let cfg = VlcConfig { library_dir: std::env::var_os("VU_VLC_DIR").map(PathBuf::from), extra_args: args };
    match VlcEngine::new(&cfg) {
        Ok(e) => Some(e),
        Err(e) => {
            eprintln!("LibVLC niet beschikbaar, test overgeslagen: {e}");
            None
        }
    }
}

fn wait_for(e: &VlcEngine, what: &str, secs: u64, pred: impl Fn(&EngineStatus) -> bool) -> EngineStatus {
    let t = Instant::now();
    loop {
        let st = e.status();
        if pred(&st) {
            return st;
        }
        assert!(t.elapsed() < Duration::from_secs(secs), "time-out bij '{what}'; laatste status: {st:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn open_play(e: &mut VlcEngine, file: &str) {
    e.open(file, &OpenOptions::default()).unwrap();
    e.play().unwrap();
}

#[test]
fn loads_libvlc_3_and_reports_its_version() {
    let Some(e) = engine() else { return };
    assert!(e.version().starts_with("3."), "{}", e.version());
}

#[test]
fn plays_a_video_with_real_length_tracks_and_advancing_time() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    let st = wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing && s.length_ms > 0);
    assert!((3_800..4_300).contains(&st.length_ms), "lengte {}", st.length_ms);
    let later = wait_for(&e, "tijd loopt", 5, |s| s.time_ms > 300);
    assert!(later.time_ms <= later.length_ms);
    assert!(!later.audio_tracks.is_empty(), "audiosporen: {:?}", later.audio_tracks);
    assert!(!later.subtitle_tracks.is_empty(), "ondertitelsporen: {:?}", later.subtitle_tracks);
    assert!(later.audio_tracks.iter().all(|t| t.id >= 0), "'Uitschakelen'-regel moet gefilterd zijn");
}

#[test]
fn pause_holds_the_position_and_resume_continues() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing && s.time_ms > 200);
    e.pause().unwrap();
    let paused = wait_for(&e, "pauze", 5, |s| s.state == PlaybackState::Paused);
    std::thread::sleep(Duration::from_millis(600));
    let still = e.status();
    assert!(still.time_ms.abs_diff(paused.time_ms) < 150, "{} vs {}", paused.time_ms, still.time_ms);
    e.play().unwrap();
    wait_for(&e, "hervatten", 5, |s| s.state == PlaybackState::Playing);
}

#[test]
fn seeking_moves_the_position() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing && s.length_ms > 0);
    e.pause().unwrap();
    wait_for(&e, "pauze", 5, |s| s.state == PlaybackState::Paused);
    e.seek(2_500).unwrap();
    let st = wait_for(&e, "zoeken", 5, |s| s.time_ms >= 2_300);
    assert!(st.time_ms <= st.length_ms);
}

#[test]
fn start_time_option_begins_playback_at_the_resume_position() {
    let Some(mut e) = engine() else { return };
    e.open(&fixture("sample.mkv"), &OpenOptions { start_ms: 2_000, ..Default::default() }).unwrap();
    e.play().unwrap();
    let st = wait_for(&e, "hervatpositie", 8, |s| s.state == PlaybackState::Playing && s.time_ms >= 1_900);
    assert!(st.time_ms < 3_500, "{}", st.time_ms);
}

#[test]
fn volume_mute_and_rate_roundtrip() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing);
    for (set, expected) in [(50, 50), (150, 150), (900, 200), (0, 0)] {
        e.set_volume(set).unwrap();
        assert_eq!(e.status().volume, expected, "volume {set}");
    }
    e.set_mute(true).unwrap();
    assert!(e.status().muted);
    e.set_mute(false).unwrap();
    assert!(!e.status().muted);
    e.set_rate(2.0).unwrap();
    assert!((e.status().rate - 2.0).abs() < 0.01);
}

#[test]
fn audio_and_subtitle_track_selection_is_reflected_in_the_status() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    let st = wait_for(&e, "sporen", 8, |s| s.state == PlaybackState::Playing && !s.subtitle_tracks.is_empty());
    let sub = st.subtitle_tracks[0].id;
    e.set_subtitle_track(Some(sub)).unwrap();
    assert_eq!(e.status().current_subtitle, Some(sub));
    e.set_subtitle_track(None).unwrap();
    assert_eq!(e.status().current_subtitle, None);
    let audio = st.audio_tracks[0].id;
    e.set_audio_track(audio).unwrap();
    assert_eq!(e.status().current_audio, Some(audio));
}

#[test]
fn delays_are_stored_in_milliseconds() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing);
    e.set_audio_delay_ms(250).unwrap();
    e.set_subtitle_delay_ms(-400).unwrap();
    let st = e.status();
    assert_eq!((st.audio_delay_ms, st.subtitle_delay_ms), (250, -400));
}

#[test]
fn stop_and_end_of_media_are_reported() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing);
    e.stop().unwrap();
    wait_for(&e, "stoppen", 5, |s| s.state == PlaybackState::Stopped);
    e.open(&fixture("sample_small_change.mkv"), &OpenOptions::default()).unwrap();
    e.play().unwrap();
    e.set_rate(4.0).unwrap();
    wait_for(&e, "einde", 10, |s| s.state == PlaybackState::Ended);
}

#[test]
fn audio_file_plays_and_reports_its_length() {
    let Some(mut e) = engine() else { return };
    e.open(&fixture("sample.mp3"), &OpenOptions { audio_visual: true, ..Default::default() }).unwrap();
    e.play().unwrap();
    let st = wait_for(&e, "audio", 8, |s| s.state == PlaybackState::Playing && s.length_ms > 0);
    assert!((1_800..2_400).contains(&st.length_ms), "{}", st.length_ms);
}

#[test]
fn missing_file_is_refused_and_corrupt_file_ends_in_an_error_state() {
    let Some(mut e) = engine() else { return };
    assert!(matches!(e.open("/bestaat/echt/niet.mkv", &OpenOptions::default()), Err(CoreError::NotFound(_))));
    let dir = tempfile::tempdir().unwrap();
    let bad: PathBuf = dir.path().join("kapot.mkv");
    std::fs::write(&bad, b"dit is helemaal geen video").unwrap();
    e.open(bad.to_str().unwrap(), &OpenOptions::default()).unwrap();
    e.play().unwrap();
    let st = wait_for(&e, "fout", 10, |s| matches!(s.state, PlaybackState::Error | PlaybackState::Ended | PlaybackState::Stopped));
    assert_ne!(st.state, PlaybackState::Playing);
}

#[test]
fn equalizer_can_be_applied_changed_and_removed() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mp3"));
    wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing);
    let mut cfg = EqConfig { preamp_db: -3.0, bands_db: [0.0; 10] };
    cfg.bands_db[4] = 6.0;
    e.set_equalizer(Some(&cfg)).unwrap();
    cfg.bands_db[4] = -4.0;
    e.set_equalizer(Some(&cfg)).unwrap();
    e.set_equalizer(None).unwrap();
    assert!(e.set_equalizer(Some(&EqConfig { preamp_db: 50.0, bands_db: [0.0; 10] })).is_err());
}

#[test]
fn picture_settings_are_validated() {
    let Some(mut e) = engine() else { return };
    use vu_vlc::{Adjust, Rotation};
    assert!(e.set_aspect_ratio(Some("16:9")).is_ok());
    assert!(e.set_aspect_ratio(None).is_ok());
    for bad in ["16/9", "0:9", "a:b", ""] {
        assert!(e.set_aspect_ratio(Some(bad)).is_err(), "{bad:?}");
        assert!(e.set_crop(Some(bad)).is_err(), "{bad:?}");
    }
    assert!(e.set_adjust(Adjust::Brightness, 1.5).is_ok());
    assert!(e.set_adjust(Adjust::Brightness, 5.0).is_err());
    assert!(e.set_adjust(Adjust::Hue, -90.0).is_ok());
    assert!(e.set_adjust(Adjust::Gamma, f32::NAN).is_err());
    assert!(e.set_deinterlace("yadif").is_ok());
    assert!(e.set_deinterlace("; rm -rf /").is_err());
    assert!(e.set_hw_decoding("d3d11va").is_ok() && e.set_hw_decoding("magie").is_err());
    assert!(e.set_rotation(Rotation::Rotate90).is_ok()); // zonder geladen media: geen fout
    assert!(e.set_audio_device("bestaat-niet").is_err());
}

#[test]
fn rotation_reopens_at_the_current_position_and_keeps_playing() {
    let Some(mut e) = engine() else { return };
    open_play(&mut e, &fixture("sample.mkv"));
    wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing && s.time_ms > 500);
    let before = e.status().time_ms;
    e.set_rotation(vu_vlc::Rotation::Rotate90).unwrap();
    let st = wait_for(&e, "opnieuw afspelen", 10, |s| s.state == PlaybackState::Playing);
    assert!(st.time_ms + 700 >= before, "positie ging terug: {} -> {}", before, st.time_ms);
}

#[test]
fn audio_device_list_does_not_crash() {
    let Some(e) = engine() else { return };
    let _ = e.list_audio_devices();
}

#[test]
fn engines_can_be_created_and_dropped_repeatedly() {
    for _ in 0..3 {
        let Some(mut e) = engine() else { return };
        open_play(&mut e, &fixture("sample.mp3"));
        wait_for(&e, "afspelen", 8, |s| s.state == PlaybackState::Playing);
    }
}

/// De hele keten met de echte engine: wachtrij, hervatten, geschiedenis en einde van de lijst.
#[test]
fn full_session_plays_a_queue_to_the_end_and_saves_history() {
    let Some(engine) = engine() else { return };
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    struct Store(Arc<Mutex<Database>>);
    impl vu_core::player::session::Persistence for Store {
        fn resume(&self, l: &str) -> Option<vu_core::db::ResumePoint> {
            self.0.resume(l)
        }
        fn save_resume(&self, l: &str, p: u64, d: u64) {
            self.0.save_resume(l, p, d)
        }
        fn mark_completed(&self, l: &str) {
            self.0.mark_completed(l)
        }
        fn record_play(&self, l: &str, t: Option<&str>) {
            self.0.record_play(l, t)
        }
    }
    use vu_core::player::session::Persistence;
    let mut s = Session::new(engine, Box::new(Store(db.clone())), 1);
    let a = fixture("sample_small_change.mkv");
    let b = fixture("sample.mp3");
    s.play_items(
        vec![QueueItem { location: a.clone(), title: Some("A".into()) }, QueueItem { location: b.clone(), title: Some("B".into()) }],
        0,
    )
    .unwrap();
    s.engine_mut().set_rate(4.0).ok();
    let t = Instant::now();
    let mut events = vec![];
    let mut now = 0u64;
    loop {
        now += 250;
        s.tick(now);
        events.extend(s.take_events());
        if events.contains(&SessionEvent::QueueEnded) {
            break;
        }
        // na het wisselen van nummer opnieuw versnellen
        if s.engine().status().rate < 3.0 {
            s.engine_mut().set_rate(4.0).ok();
        }
        assert!(t.elapsed() < Duration::from_secs(40), "wachtrij eindigde niet; events: {events:?}");
        std::thread::sleep(Duration::from_millis(250));
    }
    assert!(events.iter().any(|e| matches!(e, SessionEvent::TrackChanged { index: 1, .. })), "{events:?}");
    let h = db.lock().unwrap().list_history(10).unwrap();
    assert_eq!(h.len(), 2);
    let r = db.resume(&a);
    assert!(r.is_some_and(|r| r.completed));
    s.set_repeat(RepeatMode::Off);
}

#[test]
fn libvlc_accepts_the_subtitle_options_we_generate() {
    // Een onbekende of ongeldige optie laat libvlc_new mislukken; dit bewaakt de vertaling van de instellingen.
    if engine().is_none() {
        return; // zonder LibVLC niets te testen
    }
    let settings = vu_core::settings::AppSettings { subtitle_scale: 1.5, subtitle_color: "#ffcc00".into(), ..Default::default() };
    let mut args = vec!["--aout=dummy".to_string(), "--vout=dummy".to_string()];
    #[cfg(not(windows))]
    args.push("--no-xlib".into());
    args.extend(settings.subtitle_vlc_args());
    let cfg = VlcConfig { library_dir: std::env::var_os("VU_VLC_DIR").map(PathBuf::from), extra_args: args };
    if let Err(e) = VlcEngine::new(&cfg) {
        panic!("libvlc weigert de ondertitelopties: {e}");
    }
}

#[test]
fn hardware_decoding_can_be_switched_off_exactly_once() {
    let Some(mut e) = engine() else { return };
    assert!(e.disable_hw_decoding(), "standaard staat hardwaredecodering aan (any)");
    assert!(!e.disable_hw_decoding(), "daarna is er niets meer om uit te zetten");
}
