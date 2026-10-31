//! Wanneer moet Windows wakker blijven? Tijdens het afspelen van een video blijft ook het scherm aan,
//! bij muziek alleen de pc zelf. Bij pauze en stop mag alles weer in slaap.

use vu_core::player::session::is_audio_location;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerNeed {
    #[default]
    None,
    /// Alleen de pc wakker houden (muziek).
    System,
    /// Pc én scherm aan (video).
    Display,
}

/// `state` is de afspeelstatus zoals de speler die meldt ("playing", "paused", ...).
pub fn power_need(state: &str, location: Option<&str>) -> PowerNeed {
    match state {
        "playing" | "buffering" | "opening" => match location {
            Some(l) if is_audio_location(l) => PowerNeed::System,
            _ => PowerNeed::Display,
        },
        _ => PowerNeed::None,
    }
}

/// Onthoudt de laatst doorgegeven wens, zodat Windows alleen bij een verandering wordt aangesproken.
#[derive(Debug, Default)]
pub struct PowerTracker {
    last: PowerNeed,
}

impl PowerTracker {
    /// Geeft `Some(nieuwe wens)` terug als die verschilt van de vorige, anders `None`.
    pub fn update(&mut self, need: PowerNeed) -> Option<PowerNeed> {
        if need == self.last {
            return None;
        }
        self.last = need;
        Some(need)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_keeps_the_display_on_and_music_only_the_pc() {
        assert_eq!(power_need("playing", Some("C:\\Films\\a.mkv")), PowerNeed::Display);
        assert_eq!(power_need("buffering", Some("https://x.example/stream")), PowerNeed::Display);
        assert_eq!(power_need("playing", Some("C:\\Muziek\\a.FLAC")), PowerNeed::System);
        assert_eq!(power_need("opening", Some("/m/a.mp3")), PowerNeed::System);
    }

    #[test]
    fn pause_stop_and_errors_let_the_pc_sleep_again() {
        for s in ["paused", "stopped", "idle", "ended", "error"] {
            assert_eq!(power_need(s, Some("/m/a.mkv")), PowerNeed::None, "{s}");
        }
        assert_eq!(power_need("playing", None), PowerNeed::Display); // onbekend item: liever scherm aan dan midden in een film uit
    }

    #[test]
    fn tracker_only_reports_changes() {
        let mut t = PowerTracker::default();
        assert_eq!(t.update(PowerNeed::None), None);
        assert_eq!(t.update(PowerNeed::Display), Some(PowerNeed::Display));
        assert_eq!(t.update(PowerNeed::Display), None);
        assert_eq!(t.update(PowerNeed::System), Some(PowerNeed::System));
        assert_eq!(t.update(PowerNeed::None), Some(PowerNeed::None));
    }
}
