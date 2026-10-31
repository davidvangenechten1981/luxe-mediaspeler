//! Kijkgeschiedenis, "kijk verder" en bladwijzers.

use crate::state::AppState;
use serde::Serialize;
use vu_core::db::{Bookmark, HistoryEntry, Media};
use vu_core::error::Result;

pub fn list(st: &AppState, limit: u32) -> Result<Vec<HistoryEntry>> {
    st.lock_db()?.list_history(limit)
}
pub fn delete_entry(st: &AppState, id: i64) -> Result<()> {
    st.lock_db()?.delete_history_entry(id)
}
pub fn clear(st: &AppState) -> Result<()> {
    st.lock_db()?.clear_history()
}
pub fn enabled(st: &AppState) -> Result<bool> {
    st.lock_db()?.history_enabled()
}
pub fn set_enabled(st: &AppState, on: bool) -> Result<()> {
    st.lock_db()?.set_setting("history_enabled", if on { "true" } else { "false" })
}
pub fn mark_completed(st: &AppState, location: &str, completed: bool) -> Result<()> {
    st.lock_db()?.mark_completed(location, completed)
}

#[derive(Debug, Serialize)]
pub struct ContinueItem {
    pub location: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub updated_at: i64,
    pub media: Option<Media>,
    /// Het bestand bestaat niet meer.
    pub missing: bool,
}

pub fn continue_watching(st: &AppState, limit: u32) -> Result<Vec<ContinueItem>> {
    let db = st.lock_db()?;
    db.continue_watching(limit)?
        .into_iter()
        .map(|r| {
            let media = db.get_media_by_path(&r.location)?;
            let missing = !r.location.contains("://") && !std::path::Path::new(&r.location).exists();
            Ok(ContinueItem {
                location: r.location,
                position_ms: r.position_ms,
                duration_ms: r.duration_ms,
                updated_at: r.updated_at,
                media,
                missing,
            })
        })
        .collect()
}

pub fn bookmark_add(st: &AppState, location: &str, position_ms: u64, label: Option<&str>) -> Result<i64> {
    st.lock_db()?.add_bookmark(location, position_ms, label)
}
pub fn bookmark_list(st: &AppState, location: &str) -> Result<Vec<Bookmark>> {
    st.lock_db()?.list_bookmarks(location)
}
pub fn bookmark_delete(st: &AppState, id: i64) -> Result<()> {
    st.lock_db()?.delete_bookmark(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;

    #[test]
    fn disabling_history_stops_recording_and_clear_wipes_it() {
        let app = app();
        {
            let mut db = app.state.lock_db().unwrap();
            db.record_play("/m/a.mkv", Some("a")).unwrap();
        }
        assert_eq!(list(&app.state, 10).unwrap().len(), 1);
        set_enabled(&app.state, false).unwrap();
        assert!(!enabled(&app.state).unwrap());
        app.state.lock_db().unwrap().record_play("/m/b.mkv", None).unwrap();
        assert_eq!(list(&app.state, 10).unwrap().len(), 1);
        clear(&app.state).unwrap();
        assert!(list(&app.state, 10).unwrap().is_empty());
    }

    #[test]
    fn continue_watching_joins_library_info_and_flags_missing_files() {
        let app = app();
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        {
            let db = app.state.lock_db().unwrap();
            db.save_resume(&film, 30_000, 100_000).unwrap();
            db.save_resume("/weg/film.mkv", 40_000, 100_000).unwrap();
        }
        let items = continue_watching(&app.state, 10).unwrap();
        assert_eq!(items.len(), 2);
        let missing: Vec<bool> = items.iter().map(|i| i.missing).collect();
        assert!(missing.contains(&true) && missing.contains(&false));
        mark_completed(&app.state, &film, true).unwrap();
        assert_eq!(continue_watching(&app.state, 10).unwrap().len(), 1);
    }

    #[test]
    fn bookmarks_roundtrip() {
        let app = app();
        let id = bookmark_add(&app.state, "/m/a.mkv", 5_000, Some("Begin")).unwrap();
        assert_eq!(bookmark_list(&app.state, "/m/a.mkv").unwrap()[0].label.as_deref(), Some("Begin"));
        bookmark_delete(&app.state, id).unwrap();
        assert!(bookmark_list(&app.state, "/m/a.mkv").unwrap().is_empty());
    }
}
