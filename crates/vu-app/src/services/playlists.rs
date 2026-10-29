//! Playlists: beheren, importeren en exporteren (M3U/M3U8).

use crate::state::AppState;
use serde::Serialize;
use std::path::Path;
use vu_core::db::{Playlist, PlaylistItem};
use vu_core::error::{CoreError, Result};
use vu_core::m3u::{self, M3uEntry};
use vu_core::validate::{validate_media_path, validate_path_shape, validate_stream_url};

pub fn list(st: &AppState) -> Result<Vec<Playlist>> {
    st.lock_db()?.list_playlists()
}
pub fn create(st: &AppState, name: &str) -> Result<i64> {
    st.lock_db()?.create_playlist(name)
}
pub fn rename(st: &AppState, id: i64, name: &str) -> Result<()> {
    st.lock_db()?.rename_playlist(id, name)
}
pub fn delete(st: &AppState, id: i64) -> Result<()> {
    st.lock_db()?.delete_playlist(id)
}
pub fn duplicate(st: &AppState, id: i64) -> Result<i64> {
    st.lock_db()?.duplicate_playlist(id)
}
pub fn items(st: &AppState, id: i64) -> Result<Vec<PlaylistItem>> {
    st.lock_db()?.playlist_items(id)
}
pub fn remove_item(st: &AppState, playlist: i64, item: i64) -> Result<()> {
    st.lock_db()?.remove_playlist_item(playlist, item)
}
pub fn move_item(st: &AppState, playlist: i64, item: i64, to: u32) -> Result<()> {
    st.lock_db()?.move_playlist_item(playlist, item, to)
}

/// Voegt bestanden en/of stream-URL's toe. Ongeldige items stoppen de rest niet; ze komen in `rejected`.
#[derive(Debug, Serialize, PartialEq)]
pub struct AddResult {
    pub added: usize,
    pub rejected: Vec<(String, String)>,
}

pub fn add_items(st: &AppState, id: i64, locations: &[String]) -> Result<AddResult> {
    let allow_local = st.settings()?.allow_local_streams;
    let mut ok: Vec<(String, Option<String>)> = vec![];
    let mut rejected = vec![];
    for loc in locations {
        let checked = if loc.contains("://") {
            validate_stream_url(loc, allow_local).map(|u| u.to_string())
        } else {
            validate_media_path(loc).map(|p| p.to_string_lossy().into_owned())
        };
        match checked {
            Ok(c) => {
                let title = Path::new(&c).file_stem().map(|s| s.to_string_lossy().into_owned()).filter(|_| !c.contains("://"));
                ok.push((c, title));
            }
            Err(e) => rejected.push((loc.clone(), e.to_string())),
        }
    }
    if !ok.is_empty() {
        st.lock_db()?.add_playlist_items(id, &ok)?;
    }
    Ok(AddResult { added: ok.len(), rejected })
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub playlist_id: i64,
    pub imported: usize,
    pub missing: Vec<String>,
}

/// Leest een M3U/M3U8-bestand in een nieuwe playlist. Ontbrekende bestanden worden gemeld, niet geweigerd.
pub fn import_m3u(st: &AppState, file: &str, name: Option<&str>) -> Result<ImportResult> {
    let path = validate_media_path(file)?;
    let entries = m3u::read_file(&path)?;
    let missing: Vec<String> = m3u::missing_indexes(&entries).into_iter().map(|i| entries[i].location.clone()).collect();
    let name = name.map(str::to_string).or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned())).unwrap_or_else(|| "Geïmporteerd".into());
    let mut db = st.lock_db()?;
    let id = db.create_playlist(&name)?;
    let items: Vec<(String, Option<String>)> = entries.iter().map(|e| (e.location.clone(), e.title.clone())).collect();
    db.add_playlist_items(id, &items)?;
    Ok(ImportResult { playlist_id: id, imported: items.len(), missing })
}

/// Schrijft een playlist als M3U8. Met `relative` worden paden onder de map van het bestand relatief.
pub fn export_m3u(st: &AppState, id: i64, dest: &str, relative: bool) -> Result<()> {
    let dest = validate_path_shape(dest)?;
    if !matches!(dest.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref(), Some("m3u") | Some("m3u8")) {
        return Err(CoreError::Invalid("Kies een bestandsnaam die eindigt op .m3u of .m3u8".into()));
    }
    let items = st.lock_db()?.playlist_items(id)?;
    let entries: Vec<M3uEntry> = items.into_iter().map(|i| M3uEntry { location: i.location, title: i.title, duration_s: None }).collect();
    let text = m3u::export(&entries, relative.then(|| dest.parent()).flatten());
    std::fs::write(&dest, text)?;
    Ok(())
}

/// Item-id's van bestanden die niet meer bestaan.
pub fn missing_items(st: &AppState, id: i64) -> Result<Vec<i64>> {
    let items = st.lock_db()?.playlist_items(id)?;
    Ok(items.into_iter().filter(|i| !i.location.contains("://") && !Path::new(&i.location).exists()).map(|i| i.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;

    #[test]
    fn add_items_accepts_files_and_public_streams_and_reports_rejections() {
        let app = app();
        let dir = media_dir(&app);
        let id = create(&app.state, "Mix").unwrap();
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        let r = add_items(
            &app.state,
            id,
            &[film.clone(), "https://radio.example.com/stream".into(), "http://192.168.1.5/x".into(), "/bestaat/niet.mkv".into(), "file:///etc/passwd".into()],
        )
        .unwrap();
        assert_eq!(r.added, 2);
        assert_eq!(r.rejected.len(), 3);
        let list = items(&app.state, id).unwrap();
        assert_eq!(list[0].title.as_deref(), Some("film"));
        assert_eq!(list[1].title, None); // streams hebben geen bestandsnaam als titel
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn local_streams_can_be_allowed_by_setting() {
        let app = app();
        let id = create(&app.state, "p").unwrap();
        assert_eq!(add_items(&app.state, id, &["http://192.168.1.5/x".into()]).unwrap().added, 0);
        let mut s = app.state.settings().unwrap();
        s.allow_local_streams = true;
        vu_core::settings::save(&*app.state.lock_db().unwrap(), &s).unwrap();
        assert_eq!(add_items(&app.state, id, &["http://192.168.1.5/x".into()]).unwrap().added, 1);
    }

    #[test]
    fn export_then_import_roundtrips_with_relative_paths_and_detects_missing() {
        let app = app();
        let dir = media_dir(&app);
        let id = create(&app.state, "Bewaard").unwrap();
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        let lied = dir.join("sub/lied.mp3").to_string_lossy().into_owned();
        add_items(&app.state, id, &[film.clone(), lied.clone()]).unwrap();
        let out = dir.join("lijst.m3u8");
        export_m3u(&app.state, id, out.to_str().unwrap(), true).unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("\nfilm.mkv\n") && text.contains("sub/lied.mp3"), "{text}");
        let r = import_m3u(&app.state, out.to_str().unwrap(), None).unwrap();
        assert_eq!((r.imported, r.missing.len()), (2, 0));
        let imported = items(&app.state, r.playlist_id).unwrap();
        assert_eq!(imported[0].location, film); // relatief pad weer absoluut
        // bestand verdwijnt: wordt als ontbrekend herkend
        std::fs::remove_file(dir.join("film.mkv")).unwrap();
        assert_eq!(missing_items(&app.state, r.playlist_id).unwrap().len(), 1);
        let again = import_m3u(&app.state, out.to_str().unwrap(), Some("Tweede")).unwrap();
        assert_eq!(again.missing.len(), 1);
    }

    #[test]
    fn export_refuses_wrong_extensions_and_relative_destinations() {
        let app = app();
        let id = create(&app.state, "p").unwrap();
        assert!(export_m3u(&app.state, id, "/tmp/lijst.txt", false).is_err());
        assert!(export_m3u(&app.state, id, "lijst.m3u", false).is_err());
        assert!(export_m3u(&app.state, id, "/tmp/../etc/x.m3u", false).is_err());
        assert!(import_m3u(&app.state, "/bestaat/niet.m3u", None).is_err());
    }

    #[test]
    fn crud_flows_through_the_service() {
        let app = app();
        let id = create(&app.state, "A").unwrap();
        rename(&app.state, id, "B").unwrap();
        let copy = duplicate(&app.state, id).unwrap();
        assert_eq!(list(&app.state).unwrap().len(), 2);
        delete(&app.state, copy).unwrap();
        assert_eq!(list(&app.state).unwrap()[0].name, "B");
    }
}
