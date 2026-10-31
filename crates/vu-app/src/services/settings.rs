//! Instellingen, back-up en herstel, diagnose.

use crate::state::AppState;
use crate::tools::version_line;
use serde::Serialize;
use std::path::Path;
use vu_core::error::{CoreError, Result};
use vu_core::settings::{self, AppSettings};
use vu_core::validate::{validate_media_path, validate_path_shape};

pub fn get(st: &AppState) -> Result<(AppSettings, Option<String>)> {
    settings::load(&*st.lock_db()?)
}

/// Slaat op na validatie en past meteen toe op de speler.
pub fn save(st: &AppState, new: &AppSettings) -> Result<AppSettings> {
    let mut checked = new.clone();
    checked.validate()?;
    settings::save(&*st.lock_db()?, &checked)?;
    if let Ok(p) = &st.player {
        p.apply_settings(&checked)?;
    }
    Ok(checked)
}

pub fn reset(st: &AppState) -> Result<AppSettings> {
    save(st, &AppSettings::default())
}

pub fn export_file(st: &AppState, dest: &str) -> Result<()> {
    let dest = validate_path_shape(dest)?;
    let (s, _) = get(st)?;
    std::fs::write(dest, settings::export_json(&s)?)?;
    Ok(())
}

/// Leest een back-up. Pas als alles geldig is wordt er iets overschreven.
pub fn import_file(st: &AppState, src: &str) -> Result<AppSettings> {
    let src = validate_media_path(src)?;
    let imported = settings::import_json(&std::fs::read_to_string(src)?)?;
    save(st, &imported)
}

pub fn backup_database(st: &AppState, dest: &str) -> Result<()> {
    let dest = validate_path_shape(dest)?;
    if dest.exists() {
        return Err(CoreError::Invalid("Het back-upbestand bestaat al; kies een nieuwe naam".into()));
    }
    st.lock_db()?.backup_to(&dest)
}

/// Vervangt de bibliotheek door een back-up. Een beschadigde back-up wordt geweigerd; de huidige data blijft dan staan.
pub fn restore_database(st: &AppState, src: &str) -> Result<()> {
    let src = validate_media_path(src)?;
    let mut db = st.lock_db()?;
    // eerst een veiligheidskopie van de huidige toestand
    let safety = st.dirs.backups.join(format!("voor-herstel-{}.db", vu_core::db::now()));
    db.backup_to(&safety)?;
    db.restore_from(&src)
}

#[derive(Debug, Serialize)]
pub struct Diagnostics {
    pub app_version: String,
    pub ffmpeg: Option<String>,
    pub ffprobe: Option<String>,
    pub ffmpeg_path: Option<String>,
    pub vlc_dir: Option<String>,
    pub player_available: bool,
    pub player_error: Option<String>,
    pub schema_version: u32,
    pub data_dir: String,
    pub warnings: Vec<String>,
    pub encoders: Vec<String>,
    pub containers: Vec<String>,
}

pub fn diagnostics(st: &AppState, app_version: &str) -> Result<Diagnostics> {
    let caps = st.tasks.as_ref().and_then(|t| t.capabilities().ok());
    let mut encoders: Vec<String> = caps
        .map(|c| c.encoders.iter().filter(|e| e.starts_with("lib") || *e == "aac" || *e == "flac").cloned().collect())
        .unwrap_or_default();
    encoders.sort();
    let containers = vu_core::convert::Container::ALL
        .iter()
        .filter(|c| caps.is_some_and(|cp| cp.container_available(**c)))
        .map(|c| c.ext().to_string())
        .collect();
    Ok(Diagnostics {
        app_version: app_version.to_string(),
        ffmpeg: st.tools.ffmpeg.as_deref().and_then(version_line),
        ffprobe: st.tools.ffprobe.as_deref().and_then(version_line),
        ffmpeg_path: st.tools.ffmpeg.as_deref().map(Path::to_string_lossy).map(|s| s.into_owned()),
        vlc_dir: st.tools.vlc_dir.as_deref().map(|p| p.to_string_lossy().into_owned()),
        player_available: st.player.is_ok(),
        player_error: st.player.as_ref().err().cloned(),
        schema_version: st.lock_db()?.schema_version()?,
        data_dir: st.dirs.data.to_string_lossy().into_owned(),
        warnings: st.warnings.clone(),
        encoders,
        containers,
    })
}

/// Laatste `n` regels van een logbestand (leeg als het niet bestaat).
fn tail_lines(path: &Path, n: usize) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// Eén tekstbestand met alles wat nodig is om een probleem te onderzoeken. Bevat geen mediabestanden,
/// geen instellingen en geen wachtwoorden; wel de paden die in de logs staan.
pub fn diagnostic_report(st: &AppState, app_version: &str) -> Result<String> {
    let d = diagnostics(st, app_version)?;
    let logs = &st.dirs.logs;
    let mut out = String::new();
    out.push_str(&format!(
        "VLC Ultimate - diagnose\nVersie: {}\nSysteem: {} {}\n",
        d.app_version,
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    out.push_str(&format!(
        "Speler beschikbaar: {}\n",
        if d.player_available { "ja".to_string() } else { format!("nee ({})", d.player_error.clone().unwrap_or_default()) }
    ));
    out.push_str(&format!(
        "FFmpeg: {}\nffprobe: {}\nLibVLC-map: {}\n",
        d.ffmpeg.unwrap_or_else(|| "niet gevonden".into()),
        d.ffprobe.unwrap_or_else(|| "niet gevonden".into()),
        d.vlc_dir.unwrap_or_else(|| "niet gevonden".into())
    ));
    out.push_str(&format!("Databaseschema: {}\nGegevensmap: {}\n", d.schema_version, d.data_dir));
    for w in &d.warnings {
        out.push_str(&format!("Waarschuwing: {w}\n"));
    }
    out.push_str("\n--- app.log (laatste 200 regels) ---\n");
    out.push_str(&tail_lines(&logs.join("app.log"), 200));
    for name in ["crash.log", "crash-vorige.log"] {
        let t = tail_lines(&logs.join(name), 80);
        if !t.is_empty() {
            out.push_str(&format!("\n\n--- {name} ---\n{t}"));
        }
    }
    out.push('\n');
    Ok(out)
}

pub fn export_diagnostics(st: &AppState, app_version: &str, dest: &str) -> Result<()> {
    let dest = validate_path_shape(dest)?;
    std::fs::write(dest, diagnostic_report(st, app_version)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;

    #[test]
    fn the_diagnostic_report_contains_version_log_lines_and_an_earlier_crash() {
        let app = app();
        std::fs::create_dir_all(&app.state.dirs.logs).unwrap();
        std::fs::write(app.state.dirs.logs.join("app.log"), "[2026-10-30 10:00:00] eerste\n[2026-10-30 10:00:01] tweede\n").unwrap();
        std::fs::write(app.state.dirs.logs.join("crash-vorige.log"), "PANIC: voorbeeld").unwrap();
        let r = diagnostic_report(&app.state, "9.9.9").unwrap();
        assert!(r.contains("Versie: 9.9.9"));
        assert!(r.contains("tweede"));
        assert!(r.contains("--- crash-vorige.log ---") && r.contains("PANIC: voorbeeld"));
        let dest = app.dir.path().join("diagnose.txt");
        export_diagnostics(&app.state, "9.9.9", dest.to_str().unwrap()).unwrap();
        assert!(std::fs::read_to_string(&dest).unwrap().contains("Versie: 9.9.9"));
        assert!(export_diagnostics(&app.state, "9.9.9", "relatief.txt").is_err(), "alleen absolute paden");
    }

    #[test]
    fn tail_keeps_only_the_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.log");
        std::fs::write(&p, "a\nb\nc\nd\n").unwrap();
        assert_eq!(tail_lines(&p, 2), "c\nd");
        assert_eq!(tail_lines(&dir.path().join("nietbestaand.log"), 2), "");
    }

    #[test]
    fn saving_validates_and_persists() {
        let app = app();
        let (mut s, warn) = get(&app.state).unwrap();
        assert!(warn.is_none());
        s.ui_scale = 5.0;
        assert!(save(&app.state, &s).is_err());
        assert_eq!(get(&app.state).unwrap().0.ui_scale, 1.0); // niets veranderd
        s.ui_scale = 1.25;
        s.shortcuts.insert("mute".into(), "ctrl+alt+m".into());
        let saved = save(&app.state, &s).unwrap();
        assert_eq!(saved.shortcuts["mute"], "Ctrl+Alt+M");
        assert_eq!(get(&app.state).unwrap().0.ui_scale, 1.25);
        assert_eq!(reset(&app.state).unwrap(), AppSettings::default());
    }

    #[test]
    fn settings_export_and_import_roundtrip_and_refuse_garbage() {
        let app = app();
        let dir = app.dir.path();
        let (mut s, _) = get(&app.state).unwrap();
        s.accent = "#12ab34".into();
        save(&app.state, &s).unwrap();
        let file = dir.join("instellingen.json");
        export_file(&app.state, file.to_str().unwrap()).unwrap();
        reset(&app.state).unwrap();
        assert_eq!(import_file(&app.state, file.to_str().unwrap()).unwrap().accent, "#12ab34");
        let bad = dir.join("kapot.json");
        std::fs::write(&bad, "{niet geldig").unwrap();
        assert!(import_file(&app.state, bad.to_str().unwrap()).is_err());
        assert_eq!(get(&app.state).unwrap().0.accent, "#12ab34"); // ongewijzigd
    }

    #[test]
    fn database_backup_and_restore_keep_a_safety_copy_and_reject_garbage() {
        let app = app();
        let backup = app.dir.path().join("mijn-backup.db");
        app.state.lock_db().unwrap().create_playlist("Voor").unwrap();
        backup_database(&app.state, backup.to_str().unwrap()).unwrap();
        assert!(backup_database(&app.state, backup.to_str().unwrap()).is_err()); // niet overschrijven
        app.state.lock_db().unwrap().create_playlist("Na").unwrap();
        restore_database(&app.state, backup.to_str().unwrap()).unwrap();
        assert_eq!(app.state.lock_db().unwrap().list_playlists().unwrap().len(), 1);
        let safety: Vec<_> = std::fs::read_dir(&app.state.dirs.backups)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("voor-herstel"))
            .collect();
        assert_eq!(safety.len(), 1);
        let garbage = app.dir.path().join("rommel.db");
        std::fs::write(&garbage, b"geen database").unwrap();
        assert!(restore_database(&app.state, garbage.to_str().unwrap()).is_err());
        assert_eq!(app.state.lock_db().unwrap().list_playlists().unwrap().len(), 1);
        // huidige data intact
    }

    #[test]
    fn diagnostics_report_real_versions_and_capabilities() {
        let app = app();
        let d = diagnostics(&app.state, "0.1.0").unwrap();
        assert_eq!(d.app_version, "0.1.0");
        assert_eq!(d.schema_version, 7);
        if app.state.tools.ffmpeg.is_some() {
            assert!(d.ffmpeg.unwrap().starts_with("ffmpeg version"));
            assert!(d.containers.contains(&"mp4".to_string()));
            assert!(d.encoders.contains(&"libx264".to_string()));
        }
    }
}

#[cfg(test)]
mod sync_tests {
    use super::*;
    use crate::services::testutil::app;

    #[test]
    fn sync_needs_a_chosen_folder_and_then_writes_the_status_file() {
        let a = app();
        assert!(crate::services::sync::run(&a.state).is_err()); // nog geen map gekozen
        let folder = tempfile::tempdir().unwrap();
        let mut s = get(&a.state).unwrap().0;
        s.sync_folder = Some(folder.path().to_string_lossy().into_owned());
        save(&a.state, &s).unwrap();
        let r = crate::services::sync::run(&a.state).unwrap();
        assert_eq!((r.exported, r.imported), (0, 0));
        assert!(std::fs::read_dir(folder.path())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("vlc-ultimate-kijkstatus-")));
    }
}
