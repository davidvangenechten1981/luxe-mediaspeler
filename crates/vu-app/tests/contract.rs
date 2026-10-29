//! Contracttest: de JSON-vorm die de interface ontvangt, ligt vast in `tests/golden/*.json`.
//! De TypeScript-typen worden daartegen gecontroleerd (src/api/contract.test.ts).
//! Wijzigt een Rust-type, dan faalt deze test; met UPDATE_GOLDEN=1 vernieuw je de bestanden en
//! faalt de TypeScript-typecontrole als de typen niet zijn bijgewerkt.

use serde_json::{json, Value};
use std::path::PathBuf;
use vu_core::audio::{builtin_presets, EqConfig};
use vu_core::convert::queue::{JobSnapshot, JobState};
use vu_core::convert::{builtin_profiles, Mode};
use vu_core::db::{Bookmark, HistoryEntry, Media, MediaKind, MediaQuery, Playlist, PlaylistItem};
use vu_core::player::queue::{QueueItem, RepeatMode};
use vu_core::player::session::{EngineStatus, PlaybackState, SessionEvent, SessionView, Track};
use vu_core::probe::{parse_ffprobe_json, MediaInfo};
use vu_core::settings::AppSettings;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn check(name: &str, value: Value) {
    let path = golden_dir().join(format!("{name}.json"));
    let text = serde_json::to_string_pretty(&value).unwrap() + "\n";
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let stored = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} ontbreekt; draai met UPDATE_GOLDEN=1", path.display()));
    assert_eq!(stored, text, "De JSON-vorm van '{name}' is veranderd. Werk src/api/types.ts bij en draai met UPDATE_GOLDEN=1.");
}

fn media() -> Media {
    Media {
        id: 1,
        path: "/m/film.mkv".into(),
        kind: MediaKind::Video,
        title: Some("Film".into()),
        artist: None,
        album: None,
        genre: Some("Drama".into()),
        year: Some(2024),
        track: None,
        duration_ms: 7_200_000,
        width: Some(1920),
        height: Some(1080),
        codec: Some("h264".into()),
        container: Some("matroska".into()),
        bitrate: Some(8_000_000),
        size: 5_000_000_000,
        mtime: 1_700_000_000,
        added_at: 1_700_000_100,
        missing: false,
        favorite: true,
        play_count: 3,
        last_played: Some(1_700_000_200),
    }
}

fn status() -> EngineStatus {
    EngineStatus {
        state: PlaybackState::Playing,
        time_ms: 12_000,
        length_ms: 100_000,
        volume: 80,
        muted: false,
        rate: 1.0,
        audio_tracks: vec![Track { id: 1, name: "Nederlands".into(), language: Some("nld".into()) }],
        current_audio: Some(1),
        subtitle_tracks: vec![Track { id: 5, name: "Engels".into(), language: None }],
        current_subtitle: None,
        audio_delay_ms: 0,
        subtitle_delay_ms: -250,
        buffering_percent: None,
        error: None,
    }
}

#[test]
fn golden_json_shapes_are_stable() {
    check("media", serde_json::to_value(media()).unwrap());
    check("media_query", serde_json::to_value(MediaQuery::default()).unwrap());
    check("playlist", serde_json::to_value(Playlist { id: 1, name: "Mix".into(), item_count: 4, updated_at: 5 }).unwrap());
    check(
        "playlist_item",
        serde_json::to_value(PlaylistItem { id: 1, position: 0, location: "/m/a.mp3".into(), title: Some("A".into()) }).unwrap(),
    );
    check("history_entry", serde_json::to_value(HistoryEntry { id: 1, location: "/m/a.mp3".into(), title: None, played_at: 9 }).unwrap());
    check(
        "bookmark",
        serde_json::to_value(Bookmark { id: 1, location: "/m/a.mp3".into(), position_ms: 5_000, label: Some("Begin".into()), created_at: 9 }).unwrap(),
    );
    check("engine_status", serde_json::to_value(status()).unwrap());
    check(
        "session_view",
        serde_json::to_value(SessionView {
            status: status(),
            queue_index: Some(0),
            queue_len: 3,
            repeat: RepeatMode::All,
            shuffle: true,
            ab_a_ms: Some(1_000),
            ab_b_ms: None,
            location: Some("/m/a.mp3".into()),
        })
        .unwrap(),
    );
    check("queue_item", serde_json::to_value(QueueItem { location: "/m/a.mp3".into(), title: Some("A".into()) }).unwrap());
    check(
        "session_events",
        json!([
            serde_json::to_value(SessionEvent::TrackChanged { index: 1, location: "/m/b.mp3".into() }).unwrap(),
            serde_json::to_value(SessionEvent::ResumeApplied { position_ms: 30_000 }).unwrap(),
            serde_json::to_value(SessionEvent::AbLoopJump { to_ms: 5_000 }).unwrap(),
            serde_json::to_value(SessionEvent::OpenFailed { location: "/m/x.mp3".into(), message: "kapot".into() }).unwrap(),
            serde_json::to_value(SessionEvent::QueueEnded).unwrap(),
        ]),
    );
    check("eq_preset", serde_json::to_value(&builtin_presets()[1]).unwrap());
    check("eq_config", serde_json::to_value(EqConfig::default()).unwrap());
    check("profile", serde_json::to_value(&builtin_profiles()[2]).unwrap());
    check(
        "job_snapshot",
        serde_json::to_value(JobSnapshot {
            id: 1,
            input: "/m/a.mkv".into(),
            output: Some("/o/a.mp4".into()),
            profile: "Compacte video".into(),
            state: JobState::Running,
            mode: Mode::Transcode,
            fraction: Some(0.4),
            eta_s: Some(30),
            speed: Some(2.5),
            error: None,
            warnings: vec!["let op".into()],
            started_at: Some(1),
            finished_at: None,
        })
        .unwrap(),
    );
    let info: MediaInfo = parse_ffprobe_json(include_str!("../../vu-core/tests/fixtures/sample.ffprobe.json")).unwrap();
    check("media_info", serde_json::to_value(info).unwrap());
    check("app_settings", serde_json::to_value(AppSettings::default()).unwrap());
}
