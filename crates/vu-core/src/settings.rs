//! Gebruikersinstellingen: getypeerd, gevalideerd en opgeslagen als JSON in de database.
//! Beschadigde of onbekende opslag leidt nooit tot een crash of stil gegevensverlies.

use crate::db::Database;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SETTINGS_KEY: &str = "app_settings";
const CORRUPT_BACKUP_KEY: &str = "app_settings_corrupt_backup";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HwDecoding {
    Auto,
    None,
    D3d11va,
    Dxva2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverwriteChoice {
    Skip,
    Rename,
    Ask,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    // uiterlijk
    pub theme: Theme,
    pub accent: String,
    pub ui_scale: f32,
    pub compact: bool,
    /// Sterker contrast en dikkere randen (toegankelijkheid).
    pub high_contrast: bool,
    pub always_on_top: bool,
    pub language: String,
    // afspelen
    pub seek_short_s: u32,
    pub seek_long_s: u32,
    pub default_volume: u32,
    pub resume_playback: bool,
    pub hw_decoding: HwDecoding,
    pub preferred_audio_languages: Vec<String>,
    pub preferred_subtitle_languages: Vec<String>,
    pub subtitle_scale: f32,
    pub subtitle_color: String,
    /// Tekenset van externe ondertitelbestanden; leeg = automatisch herkennen.
    pub subtitle_encoding: String,
    /// Wachttijd na een afgelopen video voordat het volgende item start; 0 = direct.
    pub autoplay_next_delay_s: u32,
    // audio
    pub crossfade_ms: u32,
    pub normalize_realtime: bool,
    /// ReplayGain: "none", "track" of "album". Gebruikt de volumegegevens in de tags van je muziek.
    pub replay_gain: String,
    /// Stuurt AC3/DTS ongewijzigd naar een versterker (S/PDIF/HDMI). Equalizer, ReplayGain en volume werken dan niet.
    pub audio_passthrough: bool,
    /// Gedeelde map waarin pc's hun kijkstatus uitwisselen; `None` = uit.
    pub sync_folder: Option<String>,
    /// Bewaakte map: nieuwe bestanden worden automatisch geconverteerd met dit profiel.
    pub watch_dir: Option<String>,
    pub watch_profile: Option<String>,
    // netwerk en privacy
    pub network_caching_ms: u32,
    pub allow_local_streams: bool,
    pub online_metadata: bool,
    pub online_subtitle_search: bool,
    // bibliotheek
    pub scan_on_startup: bool,
    pub periodic_scan_minutes: Option<u32>,
    pub extensions: Vec<String>,
    // converter
    pub convert_output_dir: Option<String>,
    pub convert_template: String,
    pub convert_overwrite: OverwriteChoice,
    pub convert_parallel: u32,
    // sneltoetsen: actie → toets
    pub shortcuts: BTreeMap<String, String>,
}

pub fn default_shortcuts() -> BTreeMap<String, String> {
    [
        ("play_pause", "Space"),
        ("stop", "S"),
        ("volume_up", "ArrowUp"),
        ("volume_down", "ArrowDown"),
        ("mute", "M"),
        ("seek_forward", "ArrowRight"),
        ("seek_back", "ArrowLeft"),
        ("seek_forward_long", "Shift+ArrowRight"),
        ("seek_back_long", "Shift+ArrowLeft"),
        ("fullscreen", "F"),
        ("next", "N"),
        ("previous", "P"),
        ("cycle_subtitles", "V"),
        ("cycle_audio", "B"),
        ("speed_up", "]"),
        ("speed_down", "["),
        ("speed_reset", "="),
        ("frame_step", "E"),
        ("screenshot", "Shift+S"),
        ("ab_loop", "Shift+L"),
        ("open_file", "Ctrl+O"),
        ("open_playlist", "Ctrl+L"),
        ("open_settings", "Ctrl+,"),
        ("mini_player", "Ctrl+M"),
        ("toggle_playlist", "Ctrl+P"),
        ("go_to_time", "Ctrl+T"),
        ("subtitle_delay_down", "G"),
        ("subtitle_delay_up", "H"),
        ("audio_delay_down", "J"),
        ("audio_delay_up", "K"),
        ("chapter_next", "Ctrl+ArrowRight"),
        ("chapter_previous", "Ctrl+ArrowLeft"),
    ]
    .iter()
    .map(|(a, k)| (a.to_string(), k.to_string()))
    .collect()
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: Theme::Dark,
            accent: "#6d5dfc".into(),
            ui_scale: 1.0,
            compact: false,
            high_contrast: false,
            always_on_top: false,
            language: "nl".into(),
            seek_short_s: 5,
            seek_long_s: 30,
            default_volume: 80,
            resume_playback: true,
            hw_decoding: HwDecoding::Auto,
            preferred_audio_languages: vec!["nld".into(), "eng".into()],
            preferred_subtitle_languages: vec!["nld".into()],
            subtitle_scale: 1.0,
            subtitle_color: "#ffffff".into(),
            subtitle_encoding: String::new(),
            autoplay_next_delay_s: 10,
            crossfade_ms: 0,
            normalize_realtime: false,
            replay_gain: "none".into(),
            audio_passthrough: false,
            sync_folder: None,
            watch_dir: None,
            watch_profile: None,
            network_caching_ms: 1500,
            allow_local_streams: false,
            online_metadata: false,
            online_subtitle_search: false,
            scan_on_startup: false,
            periodic_scan_minutes: None,
            extensions: crate::scanner::DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            convert_output_dir: None,
            convert_template: "{name}".into(),
            convert_overwrite: OverwriteChoice::Rename,
            convert_parallel: 1,
            shortcuts: default_shortcuts(),
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_lang_code(s: &str) -> bool {
    (2..=3).contains(&s.len()) && s.chars().all(|c| c.is_ascii_lowercase())
}

/// Zet een sneltoets om naar de vaste schrijfwijze `Ctrl+Alt+Shift+Meta+Toets`.
pub fn normalize_accelerator(input: &str) -> Result<String> {
    let bad = || CoreError::Invalid(format!("Ongeldige sneltoets: '{input}'"));
    let mut mods = [false; 4]; // ctrl, alt, shift, meta
    let mut key: Option<String> = None;
    let parts: Vec<&str> = if input.trim() == "+" { vec!["+"] } else { input.split('+').collect() };
    for raw in parts {
        let p = raw.trim();
        if p.is_empty() {
            return Err(bad());
        }
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods[0] = true,
            "alt" | "option" => mods[1] = true,
            "shift" => mods[2] = true,
            "meta" | "cmd" | "win" | "super" => mods[3] = true,
            lower => {
                if key.is_some() {
                    return Err(bad());
                }
                const NAMED: [(&str, &str); 14] = [
                    ("space", "Space"),
                    ("enter", "Enter"),
                    ("escape", "Escape"),
                    ("tab", "Tab"),
                    ("home", "Home"),
                    ("end", "End"),
                    ("pageup", "PageUp"),
                    ("pagedown", "PageDown"),
                    ("delete", "Delete"),
                    ("backspace", "Backspace"),
                    ("arrowup", "ArrowUp"),
                    ("arrowdown", "ArrowDown"),
                    ("arrowleft", "ArrowLeft"),
                    ("arrowright", "ArrowRight"),
                ];
                let canonical = if let Some((_, name)) = NAMED.iter().find(|(l, _)| *l == lower) {
                    name.to_string()
                } else if lower.len() >= 2 && lower.starts_with('f') && lower[1..].parse::<u8>().is_ok_and(|n| (1..=24).contains(&n)) {
                    lower.to_ascii_uppercase()
                } else if p.chars().count() == 1 && (p.chars().all(|c| c.is_ascii_alphanumeric()) || "[],.=-/;'\\`".contains(p)) {
                    p.to_ascii_uppercase()
                } else if p == "+" {
                    "+".into()
                } else {
                    return Err(bad());
                };
                key = Some(canonical);
            }
        }
    }
    let key = key.ok_or_else(bad)?;
    let mut out = String::new();
    for (on, name) in mods.iter().zip(["Ctrl", "Alt", "Shift", "Meta"]) {
        if *on {
            out.push_str(name);
            out.push('+');
        }
    }
    out.push_str(&key);
    Ok(out)
}

/// Tekensets die in de instellingen te kiezen zijn (leeg = automatisch).
pub const SUBTITLE_ENCODINGS: &[&str] =
    &["UTF-8", "Windows-1250", "Windows-1251", "Windows-1252", "Windows-1253", "Windows-1254", "ISO-8859-1", "ISO-8859-2", "ISO-8859-15"];

impl AppSettings {
    /// LibVLC-opties voor de ondertitels. Ze gelden vanaf het opstarten van de speler.
    /// Bij VLC betekent een kleiner getal een groter lettertype (16 = normaal).
    pub fn subtitle_vlc_args(&self) -> Vec<String> {
        let rel = (16.0 / self.subtitle_scale.clamp(0.5, 3.0)).round().clamp(4.0, 40.0) as u32;
        let rgb = u32::from_str_radix(self.subtitle_color.trim_start_matches('#'), 16).unwrap_or(0x00ff_ffff);
        let mut args = vec![format!("--freetype-rel-fontsize={rel}"), format!("--freetype-color={rgb}")];
        if !self.subtitle_encoding.is_empty() {
            args.push(format!("--subsdec-encoding={}", self.subtitle_encoding));
        }
        args
    }

    /// LibVLC-opties voor het geluid. Ze gelden vanaf het opstarten van de speler.
    pub fn audio_vlc_args(&self) -> Vec<String> {
        let mut args = vec![format!("--audio-replay-gain-mode={}", self.replay_gain)];
        if self.audio_passthrough {
            args.push("--spdif".into());
        }
        args
    }

    /// Controleert en normaliseert. Geeft een duidelijke fout bij onmogelijke waarden.
    pub fn validate(&mut self) -> Result<()> {
        let bad = |m: &str| Err(CoreError::Invalid(m.to_string()));
        if !is_hex_color(&self.accent) {
            return bad("De accentkleur moet de vorm #rrggbb hebben");
        }
        if !is_hex_color(&self.subtitle_color) {
            return bad("De ondertitelkleur moet de vorm #rrggbb hebben");
        }
        if !["none", "track", "album"].contains(&self.replay_gain.as_str()) {
            return bad("ReplayGain moet uit, per nummer of per album zijn");
        }
        if !self.subtitle_encoding.is_empty() && !SUBTITLE_ENCODINGS.contains(&self.subtitle_encoding.as_str()) {
            return bad("Onbekende tekenset voor ondertitels");
        }
        if !(self.ui_scale.is_finite() && (0.75..=2.0).contains(&self.ui_scale)) {
            return bad("De interfaceschaal moet tussen 75% en 200% liggen");
        }
        if !(self.subtitle_scale.is_finite() && (0.5..=3.0).contains(&self.subtitle_scale)) {
            return bad("De ondertitelgrootte moet tussen 50% en 300% liggen");
        }
        if !(1..=300).contains(&self.seek_short_s) || !(1..=3600).contains(&self.seek_long_s) {
            return bad("De sprongintervallen liggen buiten het toegestane bereik");
        }
        if self.autoplay_next_delay_s > 120 {
            return bad("De wachttijd voor de volgende aflevering mag maximaal 120 seconden zijn");
        }
        if self.default_volume > 200 {
            return bad("Het standaardvolume mag maximaal 200% zijn");
        }
        if self.crossfade_ms > 12_000 {
            return bad("Crossfade mag maximaal 12 seconden duren");
        }
        if !(100..=60_000).contains(&self.network_caching_ms) {
            return bad("De netwerkbuffer moet tussen 100 en 60000 ms liggen");
        }
        if let Some(m) = self.periodic_scan_minutes {
            if !(5..=10_080).contains(&m) {
                return bad("Periodiek scannen kan elke 5 minuten tot 1 week");
            }
        }
        if !(1..=4).contains(&self.convert_parallel) {
            return bad("Het aantal gelijktijdige conversies moet tussen 1 en 4 liggen");
        }
        if !["nl", "en"].contains(&self.language.as_str()) {
            return bad("Onbekende taal");
        }
        for l in self.preferred_audio_languages.iter().chain(self.preferred_subtitle_languages.iter()) {
            if !is_lang_code(l) {
                return Err(CoreError::Invalid(format!("Ongeldige taalcode '{l}' (gebruik bv. nld of eng)")));
            }
        }
        for e in &self.extensions {
            if e.is_empty() || !e.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(CoreError::Invalid(format!("Ongeldige bestandsextensie '{e}'")));
            }
        }
        self.extensions = self.extensions.iter().map(|e| e.to_ascii_lowercase()).collect();
        if let Some(d) = &self.convert_output_dir {
            crate::validate::validate_path_shape(d)?;
        }
        if !self.convert_template.contains("{name}") {
            return bad("Het sjabloon voor bestandsnamen moet {name} bevatten");
        }
        // sneltoetsen: eerst normaliseren, dan op dubbelen controleren
        let mut seen: BTreeMap<String, String> = BTreeMap::new();
        let mut normalized = BTreeMap::new();
        for (action, key) in &self.shortcuts {
            let n = normalize_accelerator(key)?;
            if let Some(other) = seen.insert(n.clone(), action.clone()) {
                return Err(CoreError::Invalid(format!("Sneltoets {n} wordt al gebruikt voor '{other}' en '{action}'")));
            }
            normalized.insert(action.clone(), n);
        }
        self.shortcuts = normalized;
        Ok(())
    }

    /// Vult ontbrekende sneltoetsen aan met de standaardwaarden (bij een nieuwere app-versie).
    pub fn fill_missing_shortcuts(&mut self) {
        for (a, k) in default_shortcuts() {
            if !self.shortcuts.contains_key(&a) && !self.shortcuts.values().any(|v| *v == k) {
                self.shortcuts.insert(a, k);
            }
        }
    }
}

/// Laadt de instellingen. Bij een beschadigde opslag: standaardwaarden, en de oude tekst blijft bewaard.
pub fn load(db: &Database) -> Result<(AppSettings, Option<String>)> {
    let Some(raw) = db.get_setting(SETTINGS_KEY)? else {
        return Ok((AppSettings::default(), None));
    };
    let parsed = serde_json::from_str::<AppSettings>(&raw).map_err(|e| CoreError::Invalid(e.to_string())).and_then(|mut s| {
        s.validate()?;
        s.fill_missing_shortcuts();
        Ok(s)
    });
    match parsed {
        Ok(s) => Ok((s, None)),
        Err(e) => {
            db.set_setting(CORRUPT_BACKUP_KEY, &raw)?;
            Ok((
                AppSettings::default(),
                Some(format!(
                    "De opgeslagen instellingen waren ongeldig ({e}). Standaardwaarden zijn geladen; de oude instellingen zijn bewaard."
                )),
            ))
        }
    }
}

pub fn save(db: &Database, settings: &AppSettings) -> Result<()> {
    let mut s = settings.clone();
    s.validate()?;
    db.set_setting(SETTINGS_KEY, &serde_json::to_string(&s).map_err(|e| CoreError::Invalid(e.to_string()))?)
}

/// Instellingen als leesbare JSON, voor een back-upbestand.
pub fn export_json(settings: &AppSettings) -> Result<String> {
    let mut s = settings.clone();
    s.validate()?;
    serde_json::to_string_pretty(&s).map_err(|e| CoreError::Invalid(e.to_string()))
}

/// Leest instellingen uit een back-up. Alles wordt eerst gevalideerd; bij een fout verandert er niets.
pub fn import_json(text: &str) -> Result<AppSettings> {
    let mut s: AppSettings = serde_json::from_str(text).map_err(|e| CoreError::Invalid(format!("Geen geldig instellingenbestand: {e}")))?;
    s.validate()?;
    s.fill_missing_shortcuts();
    Ok(s)
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_old_settings_file_without_high_contrast_still_loads_with_it_off() {
        let s: super::AppSettings = serde_json::from_str(r#"{"compact": true}"#).unwrap();
        assert!(s.compact && !s.high_contrast);
    }

    use super::*;

    #[test]
    fn defaults_are_valid_and_stable_through_validation() {
        let mut s = AppSettings::default();
        let before = s.clone();
        s.validate().unwrap();
        assert_eq!(s, before);
    }

    #[test]
    fn accelerators_are_normalized_to_one_spelling() {
        for (input, expected) in [
            ("ctrl+o", "Ctrl+O"),
            ("Shift+ctrl+s", "Ctrl+Shift+S"),
            ("space", "Space"),
            ("arrowup", "ArrowUp"),
            ("f11", "F11"),
            (" Ctrl + , ", "Ctrl+,"),
            ("pagedown", "PageDown"),
            ("alt+shift+arrowleft", "Alt+Shift+ArrowLeft"),
            ("[", "["),
            ("cmd+m", "Meta+M"),
        ] {
            assert_eq!(normalize_accelerator(input).unwrap(), expected, "{input}");
        }
    }

    #[test]
    fn bad_accelerators_are_rejected() {
        for bad in ["", "Ctrl+", "Ctrl+Shift", "A+B", "F25", "Ctrl+Ctrl+X+Y", "Hyper+Q", "Ctrl+Space+Enter", "ééé"] {
            assert!(normalize_accelerator(bad).is_err(), "{bad:?} moest worden geweigerd");
        }
    }

    #[test]
    fn the_default_shortcuts_cover_delays_chapters_and_go_to_time() {
        let d = default_shortcuts();
        for action in [
            "go_to_time",
            "subtitle_delay_down",
            "subtitle_delay_up",
            "audio_delay_down",
            "audio_delay_up",
            "chapter_next",
            "chapter_previous",
        ] {
            assert!(d.contains_key(action), "{action} ontbreekt");
        }
        // de vaste toetsen van de interface (I = mediagegevens, ?/F1 = sneltoetsen) mogen niet in gebruik zijn
        for fixed in ["I", "?", "F1", "Escape"] {
            assert!(!d.values().any(|k| k == fixed), "{fixed} is gereserveerd");
        }
        // een bestaande gebruiker krijgt de nieuwe toetsen erbij
        let mut s = AppSettings { shortcuts: [("play_pause".to_string(), "Space".to_string())].into(), ..Default::default() };
        s.fill_missing_shortcuts();
        assert_eq!(s.shortcuts["go_to_time"], "Ctrl+T");
    }

    #[test]
    fn default_shortcuts_are_conflict_free() {
        let d = default_shortcuts();
        let normalized: std::collections::HashSet<_> = d.values().map(|k| normalize_accelerator(k).unwrap()).collect();
        assert_eq!(normalized.len(), d.len());
    }

    #[test]
    fn duplicate_shortcuts_are_detected_even_with_different_spelling() {
        let mut s = AppSettings::default();
        s.shortcuts.insert("stop".into(), "shift+ctrl+x".into());
        s.shortcuts.insert("mute".into(), "ctrl+shift+X".into());
        let err = s.validate().unwrap_err().to_string();
        assert!(err.contains("Ctrl+Shift+X") && err.contains("wordt al gebruikt"), "{err}");
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        type Mutation = Box<dyn Fn(&mut AppSettings)>;
        let cases: Vec<Mutation> = vec![
            Box::new(|s| s.accent = "rood".into()),
            Box::new(|s| s.ui_scale = 5.0),
            Box::new(|s| s.ui_scale = f32::NAN),
            Box::new(|s| s.seek_short_s = 0),
            Box::new(|s| s.default_volume = 500),
            Box::new(|s| s.crossfade_ms = 60_000),
            Box::new(|s| s.network_caching_ms = 5),
            Box::new(|s| s.autoplay_next_delay_s = 500),
            Box::new(|s| s.periodic_scan_minutes = Some(1)),
            Box::new(|s| s.convert_parallel = 9),
            Box::new(|s| s.language = "xx".into()),
            Box::new(|s| s.preferred_subtitle_languages = vec!["Nederlands".into()]),
            Box::new(|s| s.extensions = vec!["mp4;rm".into()]),
            Box::new(|s| s.convert_output_dir = Some("relatief/pad".into())),
            Box::new(|s| s.convert_template = "vast".into()),
        ];
        for (i, mutate) in cases.iter().enumerate() {
            let mut s = AppSettings::default();
            mutate(&mut s);
            assert!(s.validate().is_err(), "geval {i} moest worden geweigerd");
        }
    }

    #[test]
    fn the_next_episode_countdown_defaults_to_ten_seconds_and_can_be_switched_off() {
        let mut s = AppSettings::default();
        assert_eq!(s.autoplay_next_delay_s, 10);
        s.autoplay_next_delay_s = 0; // 0 = direct doorgaan
        assert!(s.validate().is_ok());
    }

    #[test]
    fn subtitle_settings_become_libvlc_options() {
        let mut s = AppSettings::default();
        assert_eq!(s.subtitle_vlc_args(), ["--freetype-rel-fontsize=16", "--freetype-color=16777215"]);
        s.subtitle_scale = 2.0; // groter = kleiner getal bij VLC
        s.subtitle_color = "#ffcc00".into();
        assert_eq!(s.subtitle_vlc_args(), ["--freetype-rel-fontsize=8", "--freetype-color=16763904"]);
        s.subtitle_scale = 0.5;
        assert_eq!(s.subtitle_vlc_args()[0], "--freetype-rel-fontsize=32");
        s.subtitle_scale = 3.0;
        assert_eq!(s.subtitle_vlc_args()[0], "--freetype-rel-fontsize=5");
    }

    #[test]
    fn subtitle_encoding_is_automatic_by_default_and_passed_on_when_chosen() {
        let mut s = AppSettings::default();
        assert_eq!(s.subtitle_encoding, "");
        assert!(!s.subtitle_vlc_args().iter().any(|a| a.contains("subsdec-encoding")));
        s.subtitle_encoding = "Windows-1252".into();
        assert!(s.validate().is_ok());
        assert!(s.subtitle_vlc_args().contains(&"--subsdec-encoding=Windows-1252".to_string()));
    }

    #[test]
    fn replay_gain_is_off_by_default_and_maps_to_the_libvlc_option() {
        let mut s = AppSettings::default();
        assert_eq!(s.replay_gain, "none");
        assert_eq!(s.audio_vlc_args(), ["--audio-replay-gain-mode=none"]);
        s.replay_gain = "album".into();
        assert!(s.validate().is_ok());
        assert_eq!(s.audio_vlc_args(), ["--audio-replay-gain-mode=album"]);
        s.replay_gain = "luid".into();
        assert!(s.validate().is_err());
    }

    #[test]
    fn passthrough_adds_the_spdif_option_and_is_off_by_default() {
        let mut s = AppSettings::default();
        assert!(!s.audio_passthrough);
        assert!(!s.audio_vlc_args().iter().any(|a| a == "--spdif"));
        s.audio_passthrough = true;
        assert!(s.audio_vlc_args().contains(&"--spdif".to_string()));
    }

    #[test]
    fn unknown_subtitle_encodings_are_rejected() {
        let mut s = AppSettings { subtitle_encoding: "x; rm -rf".into(), ..Default::default() };
        assert!(s.validate().is_err());
    }

    #[test]
    fn save_and_load_roundtrip_through_the_database() {
        let db = Database::open_in_memory().unwrap();
        let (fresh, warn) = load(&db).unwrap();
        assert_eq!((fresh, warn), (AppSettings::default(), None));
        let mut s = AppSettings { theme: Theme::Light, ui_scale: 1.25, ..Default::default() };
        s.shortcuts.insert("mute".into(), "ctrl+m".into()); // botst met mini_player: Ctrl+M
        assert!(save(&db, &s).is_err()); // ongeldig wordt niet opgeslagen
        s.shortcuts.insert("mute".into(), "Ctrl+Alt+M".into());
        save(&db, &s).unwrap();
        let (loaded, warn) = load(&db).unwrap();
        assert!(warn.is_none());
        assert_eq!((loaded.theme, loaded.ui_scale), (Theme::Light, 1.25));
        assert_eq!(loaded.shortcuts["mute"], "Ctrl+Alt+M");
    }

    #[test]
    fn corrupt_stored_settings_fall_back_to_defaults_and_keep_the_old_text() {
        let db = Database::open_in_memory().unwrap();
        db.set_setting(SETTINGS_KEY, "{dit is geen json").unwrap();
        let (s, warn) = load(&db).unwrap();
        assert_eq!(s, AppSettings::default());
        assert!(warn.unwrap().contains("bewaard"));
        assert_eq!(db.get_setting(CORRUPT_BACKUP_KEY).unwrap().as_deref(), Some("{dit is geen json"));
        // ook geldige JSON met onmogelijke waarde telt als beschadigd
        db.set_setting(SETTINGS_KEY, r##"{"ui_scale": 99.0}"##).unwrap();
        assert!(load(&db).unwrap().1.is_some());
    }

    #[test]
    fn older_settings_without_new_fields_load_with_defaults() {
        let db = Database::open_in_memory().unwrap();
        db.set_setting(SETTINGS_KEY, r#"{"theme":"light","shortcuts":{"play_pause":"Space"}}"#).unwrap();
        let (s, warn) = load(&db).unwrap();
        assert!(warn.is_none());
        assert_eq!(s.theme, Theme::Light);
        assert_eq!(s.seek_short_s, 5); // standaard voor ontbrekend veld
        assert!(s.shortcuts.len() > 1); // ontbrekende toetsen aangevuld
    }

    #[test]
    fn export_and_import_roundtrip_and_reject_bad_files() {
        let s = AppSettings { accent: "#00aa55".into(), ..Default::default() };
        let json = export_json(&s).unwrap();
        assert_eq!(import_json(&json).unwrap().accent, "#00aa55");
        assert!(import_json("geen json").is_err());
        assert!(import_json(r##"{"accent":"groen"}"##).is_err());
    }
}
