//! Speler: openen, wachtrij en alle bediening via één getypeerd commando.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vu_core::error::{CoreError, Result};
use vu_core::player::queue::{QueueItem, RepeatMode};
use vu_core::player::session::{Engine, SessionView};
use vu_core::validate::{sanitize_file_name, validate_media_path, validate_path_shape, validate_stream_url};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerCommand {
    TogglePause,
    Stop,
    Next,
    Previous,
    SeekTo { ms: u64 },
    SeekRelative { ms: i64 },
    Volume { percent: u32 },
    VolumeDelta { delta: i32 },
    ToggleMute,
    Rate { rate: f32 },
    RateStep { up: bool },
    CycleAudio,
    CycleSubtitles,
    AudioTrack { id: i32 },
    SubtitleTrack { id: Option<i32> },
    LoadSubtitle { path: String },
    AudioDelay { ms: i64 },
    SubtitleDelay { ms: i64 },
    AbPress,
    FrameStep,
    Repeat { mode: RepeatMode },
    Shuffle { on: bool },
    JumpTo { index: usize },
    RemoveFromQueue { index: usize },
    MoveInQueue { from: usize, to: usize },
    ClearQueue,
    CancelAutoplay,
}

pub fn command(st: &AppState, cmd: PlayerCommand) -> Result<SessionView> {
    let player = st.require_player()?;
    // een ondertitelbestand wordt eerst gevalideerd, buiten het slot van de speler
    let subtitle = match &cmd {
        PlayerCommand::LoadSubtitle { path } => Some(validate_subtitle_path(path)?),
        _ => None,
    };
    player.with(|s| -> Result<SessionView> {
        match cmd {
            PlayerCommand::TogglePause => s.toggle_pause()?,
            PlayerCommand::Stop => s.stop()?,
            PlayerCommand::Next => s.skip_next()?,
            PlayerCommand::Previous => s.skip_previous()?,
            PlayerCommand::SeekTo { ms } => s.seek_to(ms)?,
            PlayerCommand::SeekRelative { ms } => s.seek_relative(ms)?,
            PlayerCommand::Volume { percent } => s.set_volume(percent)?,
            PlayerCommand::VolumeDelta { delta } => s.change_volume(delta)?,
            PlayerCommand::ToggleMute => s.toggle_mute()?,
            PlayerCommand::Rate { rate } => s.set_rate(rate)?,
            PlayerCommand::RateStep { up } => s.step_rate(up)?,
            PlayerCommand::CycleAudio => s.cycle_audio()?,
            PlayerCommand::CycleSubtitles => s.cycle_subtitles()?,
            PlayerCommand::AudioTrack { id } => s.engine_mut().set_audio_track(id)?,
            PlayerCommand::SubtitleTrack { id } => s.engine_mut().set_subtitle_track(id)?,
            PlayerCommand::LoadSubtitle { .. } => s.engine_mut().load_subtitle_file(subtitle.as_deref().unwrap())?,
            PlayerCommand::AudioDelay { ms } => s.engine_mut().set_audio_delay_ms(ms.clamp(-60_000, 60_000))?,
            PlayerCommand::SubtitleDelay { ms } => s.engine_mut().set_subtitle_delay_ms(ms.clamp(-60_000, 60_000))?,
            PlayerCommand::AbPress => s.ab_press()?,
            PlayerCommand::FrameStep => s.frame_step()?,
            PlayerCommand::CancelAutoplay => s.cancel_autoplay()?,
            PlayerCommand::Repeat { mode } => s.set_repeat(mode),
            PlayerCommand::Shuffle { on } => s.set_shuffle(on),
            PlayerCommand::JumpTo { index } => s.play_index(index)?,
            PlayerCommand::RemoveFromQueue { index } => s.queue_remove(index)?,
            PlayerCommand::MoveInQueue { from, to } => s.queue_move(from, to)?,
            PlayerCommand::ClearQueue => s.queue_clear()?,
        }
        Ok(s.view())
    })?
}

fn validate_subtitle_path(path: &str) -> Result<PathBuf> {
    let p = validate_media_path(path)?;
    let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if !["srt", "ass", "ssa", "vtt", "sub", "idx"].contains(&ext.as_str()) {
        return Err(CoreError::Invalid("Kies een ondertitelbestand (.srt, .ass, .ssa, .vtt, .sub of .idx)".into()));
    }
    Ok(p)
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlayResult {
    pub queued: usize,
    pub rejected: Vec<(String, String)>,
}

type Rejected = Vec<(String, String)>;

fn to_queue_items(st: &AppState, locations: &[String]) -> Result<(Vec<QueueItem>, Rejected)> {
    let allow_local = st.settings()?.allow_local_streams;
    let (mut items, mut rejected) = (vec![], vec![]);
    for loc in locations {
        let checked = if loc.contains("://") {
            validate_stream_url(loc, allow_local).map(|u| (u.to_string(), None))
        } else {
            validate_media_path(loc).map(|p| {
                let title = p.file_stem().map(|s| s.to_string_lossy().into_owned());
                (p.to_string_lossy().into_owned(), title)
            })
        };
        match checked {
            Ok((location, title)) => items.push(QueueItem { location, title }),
            Err(e) => rejected.push((loc.clone(), e.to_string())),
        }
    }
    Ok((items, rejected))
}

/// Vervangt de wachtrij en speelt `start` af (index binnen de geldige items).
pub fn play_locations(st: &AppState, locations: Vec<String>, start: usize) -> Result<PlayResult> {
    let player = st.require_player()?;
    let (items, rejected) = to_queue_items(st, &locations)?;
    if items.is_empty() {
        return Err(CoreError::Invalid(match rejected.first() {
            Some((p, why)) => format!("Kan {p} niet openen: {why}"),
            None => "Er is niets om af te spelen".into(),
        }));
    }
    let queued = items.len();
    player.with(|s| s.play_items(items, start.min(queued - 1)))??;
    Ok(PlayResult { queued, rejected })
}

pub fn enqueue_locations(st: &AppState, locations: Vec<String>) -> Result<PlayResult> {
    let player = st.require_player()?;
    let (items, rejected) = to_queue_items(st, &locations)?;
    let queued = items.len();
    if queued > 0 {
        player.with(|s| s.enqueue(items))?;
    }
    Ok(PlayResult { queued, rejected })
}

pub fn play_library(st: &AppState, ids: Vec<i64>, start: usize) -> Result<PlayResult> {
    let paths: Vec<String> = {
        let db = st.lock_db()?;
        ids.iter().filter_map(|id| db.get_media(*id).ok().flatten()).filter(|m| !m.missing).map(|m| m.path).collect()
    };
    play_locations(st, paths, start)
}

pub fn play_playlist(st: &AppState, playlist_id: i64, start: usize) -> Result<PlayResult> {
    let items = st.lock_db()?.playlist_items(playlist_id)?;
    let (existing, missing): (Vec<_>, Vec<_>) =
        items.into_iter().partition(|i| i.location.contains("://") || Path::new(&i.location).exists());
    let mut r = play_locations(st, existing.into_iter().map(|i| i.location).collect(), start)?;
    r.rejected.extend(missing.into_iter().map(|i| (i.location, "bestand ontbreekt".to_string())));
    Ok(r)
}

pub fn view(st: &AppState) -> Result<SessionView> {
    st.require_player()?.view()
}

pub fn queue_items(st: &AppState) -> Result<Vec<QueueItem>> {
    st.require_player()?.with(|s| s.queue().items().to_vec())
}

/// Maakt een screenshot in `dir`. Geeft het pad terug.
pub fn snapshot(st: &AppState, dir: &str) -> Result<PathBuf> {
    let player = st.require_player()?;
    let dir = validate_path_shape(dir)?;
    std::fs::create_dir_all(&dir)?;
    player.with(|s| {
        let v = s.view();
        let loc = v.location.ok_or_else(|| CoreError::Invalid("Er speelt niets".into()))?;
        let stem = Path::new(&loc).file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_else(|| "screenshot".into());
        let secs = v.status.time_ms / 1000;
        let name = sanitize_file_name(&format!("{stem} {:02}-{:02}-{:02}.png", secs / 3600, (secs / 60) % 60, secs % 60));
        let path = dir.join(name);
        s.engine_mut().take_snapshot(&path)?;
        Ok(path)
    })?
}

/// Ondertitelbestanden naast de video, voor de keuzelijst.
pub fn sidecar_subtitles(path: &str) -> Result<Vec<String>> {
    let p = validate_media_path(path)?;
    Ok(vu_core::player::session::Session::<vu_vlc::VlcEngine>::find_sidecar_subtitles(&p)
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;
    use vu_core::player::session::PlaybackState;

    fn has_player(app: &TestApp) -> bool {
        app.state.player.is_ok()
    }

    #[test]
    fn plays_files_validates_inputs_and_reports_rejections() {
        let app = app();
        if !has_player(&app) {
            return;
        }
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        let r = play_locations(&app.state, vec!["/bestaat/niet.mkv".into(), film.clone(), "http://127.0.0.1/x".into()], 0).unwrap();
        assert_eq!(r.queued, 1);
        assert_eq!(r.rejected.len(), 2);
        assert!(wait_until(10, || view(&app.state).unwrap().status.state == PlaybackState::Playing));
        assert_eq!(view(&app.state).unwrap().location.as_deref(), Some(film.as_str()));
        // alleen ongeldige invoer: duidelijke fout, de huidige afspeling blijft
        let err = play_locations(&app.state, vec!["/tmp/../etc/passwd".into()], 0).unwrap_err().to_string();
        assert!(err.contains("niet openen"), "{err}");
        assert_eq!(view(&app.state).unwrap().location.as_deref(), Some(film.as_str()));
    }

    #[test]
    fn typed_commands_drive_the_real_player() {
        let app = app();
        if !has_player(&app) {
            return;
        }
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        let lied = dir.join("sub/lied.mp3").to_string_lossy().into_owned();
        play_locations(&app.state, vec![film, lied.clone()], 0).unwrap();
        assert!(wait_until(10, || view(&app.state).unwrap().status.state == PlaybackState::Playing));
        let v = command(&app.state, PlayerCommand::Volume { percent: 70 }).unwrap();
        assert_eq!(v.status.volume, 70);
        assert_eq!(command(&app.state, PlayerCommand::VolumeDelta { delta: 500 }).unwrap().status.volume, 200);
        assert!(command(&app.state, PlayerCommand::ToggleMute).unwrap().status.muted);
        assert!(command(&app.state, PlayerCommand::Rate { rate: 0.0 }).is_err());
        assert!(command(&app.state, PlayerCommand::Shuffle { on: true }).unwrap().shuffle);
        let v = command(&app.state, PlayerCommand::Repeat { mode: RepeatMode::All }).unwrap();
        assert_eq!(v.repeat, RepeatMode::All);
        command(&app.state, PlayerCommand::TogglePause).unwrap();
        assert!(wait_until(5, || view(&app.state).unwrap().status.state == PlaybackState::Paused));
        // wachtrij: verplaatsen en verwijderen
        command(&app.state, PlayerCommand::Shuffle { on: false }).unwrap();
        command(&app.state, PlayerCommand::MoveInQueue { from: 1, to: 0 }).unwrap();
        assert_eq!(queue_items(&app.state).unwrap()[0].location, lied);
        assert!(command(&app.state, PlayerCommand::JumpTo { index: 99 }).is_err());
        let v = command(&app.state, PlayerCommand::ClearQueue).unwrap();
        assert_eq!(v.queue_len, 0);
    }

    #[test]
    fn subtitle_files_are_validated_by_extension_and_path() {
        let app = app();
        if !has_player(&app) {
            return;
        }
        let dir = media_dir(&app);
        let txt = dir.join("x.txt");
        std::fs::write(&txt, "x").unwrap();
        assert!(command(&app.state, PlayerCommand::LoadSubtitle { path: txt.to_string_lossy().into() }).is_err());
        assert!(command(&app.state, PlayerCommand::LoadSubtitle { path: "/tmp/../etc/x.srt".into() }).is_err());
        assert!(command(&app.state, PlayerCommand::LoadSubtitle { path: "relatief.srt".into() }).is_err());
    }

    #[test]
    fn playlist_playback_skips_missing_files_and_reports_them() {
        let app = app();
        if !has_player(&app) {
            return;
        }
        let dir = media_dir(&app);
        let pl = crate::services::playlists::create(&app.state, "mix").unwrap();
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        crate::services::playlists::add_items(&app.state, pl, std::slice::from_ref(&film)).unwrap();
        app.state.lock_db().unwrap().add_playlist_items(pl, &[("/weg/verdwenen.mkv".into(), None)]).unwrap();
        let r = play_playlist(&app.state, pl, 0).unwrap();
        assert_eq!(r.queued, 1);
        assert_eq!(r.rejected, vec![("/weg/verdwenen.mkv".to_string(), "bestand ontbreekt".to_string())]);
    }

    #[test]
    fn history_and_resume_are_recorded_through_the_real_stack() {
        let app = app();
        if !has_player(&app) {
            return;
        }
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        play_locations(&app.state, vec![film.clone()], 0).unwrap();
        assert!(wait_until(10, || view(&app.state).unwrap().status.state == PlaybackState::Playing));
        command(&app.state, PlayerCommand::Stop).unwrap();
        let h = crate::services::history::list(&app.state, 10).unwrap();
        assert_eq!(h[0].location, film);
        assert_eq!(h[0].title.as_deref(), Some("film"));
    }

    #[test]
    fn sidecar_subtitle_listing_validates_the_video_path() {
        let app = app();
        let dir = media_dir(&app);
        std::fs::write(dir.join("film.srt"), "x").unwrap();
        let found = sidecar_subtitles(dir.join("film.mkv").to_str().unwrap()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(sidecar_subtitles("/bestaat/niet.mkv").is_err());
    }

    #[test]
    fn commands_deserialize_from_the_json_the_ui_sends() {
        let c: PlayerCommand = serde_json::from_str(r#"{"type":"seek_relative","ms":-5000}"#).unwrap();
        assert!(matches!(c, PlayerCommand::SeekRelative { ms: -5000 }));
        let c: PlayerCommand = serde_json::from_str(r#"{"type":"subtitle_track","id":null}"#).unwrap();
        assert!(matches!(c, PlayerCommand::SubtitleTrack { id: None }));
        let c: PlayerCommand = serde_json::from_str(r#"{"type":"repeat","mode":"all"}"#).unwrap();
        assert!(matches!(c, PlayerCommand::Repeat { mode: RepeatMode::All }));
        assert!(serde_json::from_str::<PlayerCommand>(r#"{"type":"rm_rf"}"#).is_err());
        assert!(serde_json::from_str::<PlayerCommand>(r#"{"type":"volume","percent":-5}"#).is_err());
    }
}
