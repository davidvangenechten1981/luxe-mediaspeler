//! Kijkstatus delen tussen pc's via een klein JSON-bestand per pc in een gedeelde map.
//! Het SQLite-bestand zelf wordt nooit gedeeld (kans op corruptie). Bestanden worden gekoppeld op bestandsnaam,
//! omdat de mappen op elke pc anders kunnen heten. Bij verschil wint de nieuwste wijziging.

use crate::db::Database;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const FILE_PREFIX: &str = "vlc-ultimate-kijkstatus-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncEntry {
    /// Bestandsnaam in kleine letters.
    pub key: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub completed: bool,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SyncFile {
    version: u32,
    device: String,
    entries: Vec<SyncEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SyncReport {
    pub exported: usize,
    pub imported: usize,
}

fn key_of(location: &str) -> Option<String> {
    if location.contains("://") {
        return None; // streams hebben geen vaste bestandsnaam
    }
    location.rsplit(['/', '\\']).next().filter(|n| !n.is_empty()).map(str::to_lowercase)
}

fn safe_device(name: &str) -> String {
    let s: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(40).collect();
    if s.is_empty() {
        "pc".into()
    } else {
        s
    }
}

pub fn local_entries(db: &Database) -> Result<Vec<SyncEntry>> {
    let mut best: HashMap<String, SyncEntry> = HashMap::new();
    for r in db.all_resume()? {
        let Some(key) = key_of(&r.location) else { continue };
        let e = SyncEntry {
            key: key.clone(),
            position_ms: r.position_ms,
            duration_ms: r.duration_ms,
            completed: r.completed,
            updated_at: r.updated_at,
        };
        if best.get(&key).is_none_or(|old| old.updated_at < e.updated_at) {
            best.insert(key, e);
        }
    }
    let mut v: Vec<_> = best.into_values().collect();
    v.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(v)
}

/// Past nieuwere gegevens van een andere pc toe op alle lokale bestanden met dezelfde naam. Geeft het aantal wijzigingen.
pub fn apply_entries(db: &Database, entries: &[SyncEntry]) -> Result<usize> {
    let mut by_key: HashMap<String, Vec<String>> = HashMap::new();
    for m in db.all_media_paths()? {
        if let Some(k) = key_of(&m) {
            by_key.entry(k).or_default().push(m);
        }
    }
    let mut changed = 0;
    for e in entries {
        for loc in by_key.get(&e.key).into_iter().flatten() {
            let newer = db.get_resume(loc)?.is_none_or(|r| r.updated_at < e.updated_at);
            if newer {
                db.set_resume_exact(loc, e.position_ms, e.duration_ms, e.completed, e.updated_at)?;
                changed += 1;
            }
        }
    }
    Ok(changed)
}

/// Schrijft de eigen status weg en leest die van andere pc's in.
pub fn sync_folder(db: &Database, folder: &Path, device: &str) -> Result<SyncReport> {
    if !folder.is_dir() {
        return Err(CoreError::NotFound(folder.display().to_string()));
    }
    let device = safe_device(device);
    let own = format!("{FILE_PREFIX}{device}.json");
    let mut report = SyncReport::default();
    for entry in std::fs::read_dir(folder)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(FILE_PREFIX) || !name.ends_with(".json") || name == own {
            continue;
        }
        if entry.metadata().is_ok_and(|m| m.len() > 8 << 20) {
            continue;
        }
        // Een kapot of vreemd bestand van een andere pc mag de synchronisatie niet stoppen.
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let Ok(file) = serde_json::from_str::<SyncFile>(&text) else { continue };
        if file.version == 1 {
            report.imported += apply_entries(db, &file.entries)?;
        }
    }
    let entries = local_entries(db)?;
    report.exported = entries.len();
    let body = serde_json::to_string_pretty(&SyncFile { version: 1, device, entries }).map_err(|e| CoreError::Invalid(e.to_string()))?;
    // Eerst naar een tijdelijk bestand en dan hernoemen, zodat een andere pc nooit een half bestand leest.
    let tmp = folder.join(format!("{own}.tmp"));
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, folder.join(&own))?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{MediaKind, NewMedia};

    fn db_with(paths: &[&str]) -> Database {
        let mut d = Database::open_in_memory().unwrap();
        for p in paths {
            let m = NewMedia {
                path: (*p).into(),
                kind: MediaKind::Video,
                title: None,
                artist: None,
                album: None,
                genre: None,
                year: None,
                track: None,
                duration_ms: 100_000,
                width: None,
                height: None,
                codec: None,
                container: None,
                bitrate: None,
                size: 1,
                mtime: 1,
            };
            d.upsert_media(&m).unwrap();
        }
        d
    }

    #[test]
    fn watch_position_travels_between_two_pcs_with_different_folders() {
        let folder = tempfile::tempdir().unwrap();
        let pc1 = db_with(&["D:\\Films\\Film.mkv"]);
        let pc2 = db_with(&["E:\\Media\\film.MKV"]);
        pc1.set_resume_exact("D:\\Films\\Film.mkv", 40_000, 100_000, false, 1_000).unwrap();
        assert_eq!(sync_folder(&pc1, folder.path(), "pc1").unwrap().exported, 1);
        let r = sync_folder(&pc2, folder.path(), "pc2").unwrap();
        assert_eq!(r.imported, 1);
        let got = pc2.get_resume("E:\\Media\\film.MKV").unwrap().unwrap();
        assert_eq!((got.position_ms, got.completed), (40_000, false));
    }

    #[test]
    fn the_newest_change_wins_in_both_directions() {
        let folder = tempfile::tempdir().unwrap();
        let pc1 = db_with(&["/a/x.mkv"]);
        let pc2 = db_with(&["/b/x.mkv"]);
        pc1.set_resume_exact("/a/x.mkv", 10_000, 100_000, false, 1_000).unwrap();
        pc2.set_resume_exact("/b/x.mkv", 70_000, 100_000, false, 2_000).unwrap();
        sync_folder(&pc2, folder.path(), "pc2").unwrap();
        let r = sync_folder(&pc1, folder.path(), "pc1").unwrap();
        assert_eq!(r.imported, 1);
        assert_eq!(pc1.get_resume("/a/x.mkv").unwrap().unwrap().position_ms, 70_000);
        // pc2 leest daarna pc1 en houdt zijn eigen nieuwere stand niet kwijt.
        sync_folder(&pc1, folder.path(), "pc1").unwrap();
        let r2 = sync_folder(&pc2, folder.path(), "pc2").unwrap();
        assert_eq!(pc2.get_resume("/b/x.mkv").unwrap().unwrap().position_ms, 70_000);
        assert_eq!(r2.imported, 0);
    }

    #[test]
    fn unknown_files_streams_and_broken_peer_files_are_ignored() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("vlc-ultimate-kijkstatus-kapot.json"), "{ geen json").unwrap();
        let pc = db_with(&["/a/x.mkv"]);
        pc.set_resume_exact("http://stream/live", 5_000, 0, false, 5).unwrap();
        let r = sync_folder(&pc, folder.path(), "pc").unwrap();
        assert_eq!((r.exported, r.imported), (0, 0)); // stream telt niet mee
    }

    #[test]
    fn a_missing_folder_is_an_error_and_device_names_are_sanitised() {
        let pc = db_with(&[]);
        assert!(sync_folder(&pc, Path::new("/bestaat/echt/niet"), "pc").is_err());
        assert_eq!(safe_device("../../evil"), "evil");
        assert_eq!(safe_device(""), "pc");
    }
}
