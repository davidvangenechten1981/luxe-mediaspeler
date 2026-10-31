//! Windows-mediabediening (SMTC): wat tonen we in het volumepaneel en wat doen de knoppen?
//! Dit bestand bevat alleen de platformonafhankelijke logica; het koppelen aan Windows zit in `src-tauri`.

use crate::services::player::PlayerCommand;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDisplay {
    pub title: Option<String>,
    pub status: MediaStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKey {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
}

fn title_of(location: &str) -> String {
    if location.contains("://") {
        return location.split("://").nth(1).and_then(|r| r.split(['/', '?']).next()).unwrap_or(location).to_string();
    }
    let name = location.rsplit(['/', '\\']).next().unwrap_or(location);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem).to_string()
}

/// Leidt uit de statusmelding van de speler (`player://state`) af wat Windows moet tonen.
pub fn display_from(payload: &Value) -> MediaDisplay {
    let status = match payload.pointer("/status/state").and_then(Value::as_str) {
        Some("playing" | "buffering" | "opening") => MediaStatus::Playing,
        Some("paused") => MediaStatus::Paused,
        _ => MediaStatus::Stopped,
    };
    let title = payload.get("location").and_then(Value::as_str).filter(|l| !l.is_empty()).map(title_of);
    MediaDisplay {
        title: if status == MediaStatus::Stopped && payload.get("location").is_none_or(Value::is_null) { None } else { title },
        status,
    }
}

/// Welk spelerscommando hoort bij een knop, gegeven de huidige status? `None` = niets te doen.
pub fn command_for(key: MediaKey, status: MediaStatus) -> Option<PlayerCommand> {
    match (key, status) {
        (MediaKey::Play, MediaStatus::Paused) | (MediaKey::Pause, MediaStatus::Playing) => Some(PlayerCommand::TogglePause),
        (MediaKey::Play, _) | (MediaKey::Pause, _) => None,
        (MediaKey::Stop, MediaStatus::Stopped) => None,
        (MediaKey::Stop, _) => Some(PlayerCommand::Stop),
        (MediaKey::Next, _) => Some(PlayerCommand::Next),
        (MediaKey::Previous, _) => Some(PlayerCommand::Previous),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_player_state_maps_to_what_windows_shows() {
        let d = display_from(&json!({"status": {"state": "playing"}, "location": "D:\\Muziek\\Mijn lied.mp3"}));
        assert_eq!(d, MediaDisplay { title: Some("Mijn lied".into()), status: MediaStatus::Playing });
        assert_eq!(
            display_from(&json!({"status": {"state": "paused"}, "location": "/m/film.2020.mkv"})).title.as_deref(),
            Some("film.2020")
        );
        assert_eq!(display_from(&json!({"status": {"state": "buffering"}, "location": "/m/a.mp3"})).status, MediaStatus::Playing);
    }

    #[test]
    fn a_stopped_player_shows_nothing_and_streams_show_their_host() {
        let d = display_from(&json!({"status": {"state": "idle"}, "location": null}));
        assert_eq!(d, MediaDisplay { title: None, status: MediaStatus::Stopped });
        assert_eq!(
            display_from(&json!({"status": {"state": "playing"}, "location": "http://radio.example.nl/stream?x=1"})).title.as_deref(),
            Some("radio.example.nl")
        );
        assert_eq!(display_from(&json!({})).status, MediaStatus::Stopped);
    }

    #[test]
    fn keys_only_act_when_they_make_sense() {
        assert!(matches!(command_for(MediaKey::Play, MediaStatus::Paused), Some(PlayerCommand::TogglePause)));
        assert!(matches!(command_for(MediaKey::Pause, MediaStatus::Playing), Some(PlayerCommand::TogglePause)));
        assert!(command_for(MediaKey::Play, MediaStatus::Playing).is_none());
        assert!(command_for(MediaKey::Pause, MediaStatus::Paused).is_none());
        assert!(command_for(MediaKey::Stop, MediaStatus::Stopped).is_none());
        assert!(matches!(command_for(MediaKey::Stop, MediaStatus::Playing), Some(PlayerCommand::Stop)));
        assert!(matches!(command_for(MediaKey::Next, MediaStatus::Stopped), Some(PlayerCommand::Next)));
        assert!(matches!(command_for(MediaKey::Previous, MediaStatus::Paused), Some(PlayerCommand::Previous)));
    }
}
