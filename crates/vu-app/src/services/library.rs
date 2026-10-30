//! Bibliotheek: mappen, zoeken, favorieten, dubbelen, opnieuw koppelen en scannen.

use crate::events::{EV_SCAN_DONE, EV_SCAN_PROGRESS};
use crate::state::{AppState, ScanRun};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vu_core::db::{Database, Media, MediaQuery};
use vu_core::error::{CoreError, Result};
use vu_core::scanner::{scan_folder, FfProbe, ScanOptions};
use vu_core::validate::{validate_folder_path, validate_media_path};

pub fn add_folder(st: &AppState, path: &str) -> Result<()> {
    let p = validate_folder_path(path)?;
    st.lock_db()?.add_library_folder(&p.to_string_lossy())?;
    Ok(())
}

/// Haalt een map uit de bibliotheek. Bestanden op schijf blijven onaangeroerd.
pub fn remove_folder(st: &AppState, path: &str) -> Result<u64> {
    st.lock_db()?.remove_library_folder(path)
}

pub fn list_folders(st: &AppState) -> Result<Vec<String>> {
    st.lock_db()?.list_library_folders()
}

pub fn query(st: &AppState, q: &MediaQuery) -> Result<Vec<Media>> {
    st.lock_db()?.query_media(q)
}

pub fn count(st: &AppState, q: &MediaQuery) -> Result<u64> {
    st.lock_db()?.count_media(q)
}

pub fn stats(st: &AppState) -> Result<vu_core::db::LibraryStats> {
    st.lock_db()?.library_stats()
}

pub fn set_favorite(st: &AppState, id: i64, favorite: bool) -> Result<()> {
    st.lock_db()?.set_favorite(id, favorite)
}

pub fn duplicates(st: &AppState) -> Result<Vec<Vec<Media>>> {
    st.lock_db()?.find_duplicates()
}

pub fn relink(st: &AppState, id: i64, new_path: &str) -> Result<()> {
    let p = validate_media_path(new_path)?;
    st.lock_db()?.relink_media(id, &p.to_string_lossy())
}

/// Verwijdert een item uit de bibliotheek (niet van schijf).
pub fn remove_media(st: &AppState, id: i64) -> Result<()> {
    st.lock_db()?.remove_media(id)
}

pub fn scan_running(st: &AppState) -> bool {
    st.scan.lock().map(|s| s.as_ref().is_some_and(|r| !r.handle.is_finished())).unwrap_or(false)
}

/// Start een scan op de achtergrond. `folder = None` scant alle bibliotheekmappen.
/// De scan gebruikt een eigen databaseverbinding, zodat de interface niet wacht.
pub fn start_scan(st: &AppState, folder: Option<&str>) -> Result<()> {
    let ffprobe = st.require_ffprobe()?.to_path_buf();
    if scan_running(st) {
        return Err(CoreError::Invalid("Er loopt al een scan".into()));
    }
    let folders: Vec<PathBuf> = match folder {
        Some(f) => {
            let p = validate_folder_path(f)?;
            st.lock_db()?.add_library_folder(&p.to_string_lossy())?;
            vec![p]
        }
        None => st.lock_db()?.list_library_folders()?.into_iter().map(PathBuf::from).collect(),
    };
    if folders.is_empty() {
        return Err(CoreError::Invalid("Er zijn nog geen bibliotheekmappen toegevoegd".into()));
    }
    let extensions = st.settings()?.extensions.into_iter().collect();
    let cancel = Arc::new(AtomicBool::new(false));
    let (db_path, sink, cancel2) = (st.dirs.db_path(), st.sink.clone(), cancel.clone());
    let handle = std::thread::spawn(move || {
        let outcome = (|| -> Result<Vec<serde_json::Value>> {
            let mut db = Database::open(&db_path)?;
            let prober = FfProbe { bin: ffprobe };
            let opts = ScanOptions { extensions, cancel: cancel2 };
            let mut reports = vec![];
            for f in &folders {
                let sink2 = sink.clone();
                let report = scan_folder(&mut db, f, &prober, &opts, &mut |p| sink2.emit(EV_SCAN_PROGRESS, json!(p)))?;
                reports.push(json!({ "folder": f, "report": report }));
            }
            Ok(reports)
        })();
        match outcome {
            Ok(reports) => sink.emit(EV_SCAN_DONE, json!({ "ok": true, "reports": reports })),
            Err(CoreError::Cancelled) => sink.emit(EV_SCAN_DONE, json!({ "ok": false, "cancelled": true })),
            Err(e) => sink.emit(EV_SCAN_DONE, json!({ "ok": false, "error": e.to_string() })),
        }
    });
    *st.scan.lock().map_err(|_| CoreError::Invalid("scan-slot vergrendeld".into()))? = Some(ScanRun { cancel, handle });
    Ok(())
}

pub fn cancel_scan(st: &AppState) {
    if let Ok(s) = st.scan.lock() {
        if let Some(r) = s.as_ref() {
            r.cancel.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::testing::RecordingSink;
    use crate::services::testutil::*;

    fn finished(app: &TestApp) -> bool {
        wait_until(30, || !scan_running(&app.state) && !app.sink.named(EV_SCAN_DONE).is_empty())
    }

    #[test]
    fn scanning_runs_in_the_background_and_fills_the_library() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        start_scan(&app.state, Some(dir.to_str().unwrap())).unwrap();
        assert!(finished(&app));
        let done = &app.sink.named(EV_SCAN_DONE)[0];
        assert_eq!(done["ok"], true, "{done}");
        assert_eq!(done["reports"][0]["report"]["added"], 2);
        assert!(!app.sink.named(EV_SCAN_PROGRESS).is_empty());
        let items = query(&app.state, &MediaQuery::default()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(list_folders(&app.state).unwrap(), vec![dir.to_string_lossy().to_string()]);
    }

    #[test]
    fn the_ui_can_query_while_a_scan_runs() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        start_scan(&app.state, Some(dir.to_str().unwrap())).unwrap();
        // tijdens de scan blijven queries direct antwoorden (geen vergrendeling door de scan)
        for _ in 0..20 {
            let t = std::time::Instant::now();
            count(&app.state, &MediaQuery::default()).unwrap();
            assert!(t.elapsed() < std::time::Duration::from_secs(2));
        }
        assert!(finished(&app));
    }

    #[test]
    fn a_second_scan_is_refused_while_one_runs_and_cancel_works() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        start_scan(&app.state, Some(dir.to_str().unwrap())).unwrap();
        let second = start_scan(&app.state, Some(dir.to_str().unwrap()));
        // de eerste scan kan al klaar zijn; beide uitkomsten zijn geldig, maar nooit twee tegelijk
        if scan_running(&app.state) {
            assert!(second.is_err());
        }
        cancel_scan(&app.state);
        assert!(wait_until(30, || !scan_running(&app.state)));
    }

    #[test]
    fn invalid_and_missing_folders_are_rejected() {
        let app = app();
        assert!(add_folder(&app.state, "relatief").is_err());
        assert!(add_folder(&app.state, "/bestaat/echt/niet").is_err());
        assert!(start_scan(&app.state, None).is_err()); // nog geen mappen
        assert!(relink(&app.state, 1, "/tmp/../etc/passwd").is_err());
    }

    #[test]
    fn removing_a_folder_removes_its_items_but_never_the_files() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        start_scan(&app.state, Some(dir.to_str().unwrap())).unwrap();
        assert!(finished(&app));
        assert_eq!(remove_folder(&app.state, dir.to_str().unwrap()).unwrap(), 2);
        assert_eq!(count(&app.state, &MediaQuery::default()).unwrap(), 0);
        assert!(dir.join("film.mkv").exists() && dir.join("sub/lied.mp3").exists());
    }

    #[test]
    fn favorites_and_search_work_through_the_service() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        start_scan(&app.state, Some(dir.to_str().unwrap())).unwrap();
        assert!(finished(&app));
        let hits = query(&app.state, &MediaQuery { text: Some("liedje".into()), ..Default::default() }).unwrap();
        assert_eq!(hits.len(), 1); // titel uit de tags
        set_favorite(&app.state, hits[0].id, true).unwrap();
        assert_eq!(query(&app.state, &MediaQuery { favorite_only: true, ..Default::default() }).unwrap().len(), 1);
        remove_media(&app.state, hits[0].id).unwrap();
        assert_eq!(count(&app.state, &MediaQuery::default()).unwrap(), 1);
        let _ = RecordingSink::default();
    }
}
