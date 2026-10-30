//! Het enige toegangspunt van de interface tot de backend.
//!
//! Elk verzoek is een variant van `Request`. Serde weigert onbekende types en
//! verkeerd getypeerde velden, en elke dienst valideert zijn invoer opnieuw.
//! Er bestaat geen manier om een shellcommando of een vrij FFmpeg-argument mee te geven.

use crate::services::{audio, converter, history, library, lyrics, player, playlists, series, settings, video};
use crate::state::AppState;
use serde::Deserialize;
use serde_json::{json, Value};
use vu_core::audio::EqConfig;
use vu_core::db::MediaQuery;
use vu_core::error::{CoreError, Result};
use vu_core::settings::AppSettings;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    // app
    AppInfo,
    Diagnostics,
    // bibliotheek
    LibraryAddFolder { path: String },
    LibraryRemoveFolder { path: String },
    LibraryFolders,
    LibraryQuery { query: MediaQuery },
    LibraryCount { query: MediaQuery },
    LibraryStats,
    LibrarySetFavorite { id: i64, favorite: bool },
    LibraryDuplicates,
    LibraryRelink { id: i64, path: String },
    LibraryRemoveMedia { id: i64 },
    LibraryScanStart { folder: Option<String> },
    LibraryScanCancel,
    LibraryScanRunning,
    SeriesList,
    CollectionsList,
    CollectionCreate { name: String, query: MediaQuery },
    CollectionDelete { id: i64 },
    CollectionRun { id: i64 },
    // playlists
    PlaylistList,
    PlaylistCreate { name: String },
    PlaylistRename { id: i64, name: String },
    PlaylistDelete { id: i64 },
    PlaylistDuplicate { id: i64 },
    PlaylistItems { id: i64 },
    PlaylistAddItems { id: i64, locations: Vec<String> },
    PlaylistRemoveItem { playlist: i64, item: i64 },
    PlaylistMoveItem { playlist: i64, item: i64, to: u32 },
    PlaylistImport { file: String, name: Option<String> },
    PlaylistExport { id: i64, dest: String, relative: bool },
    PlaylistMissing { id: i64 },
    // geschiedenis en bladwijzers
    HistoryList { limit: u32 },
    HistoryDelete { id: i64 },
    HistoryClear,
    HistoryEnabled,
    HistorySetEnabled { on: bool },
    ContinueWatching { limit: u32 },
    MarkCompleted { location: String, completed: bool },
    BookmarkAdd { location: String, position_ms: u64, label: Option<String> },
    BookmarkList { location: String },
    BookmarkDelete { id: i64 },
    // speler
    PlayerView,
    PlayerCommand { command: player::PlayerCommand },
    PlayLocations { locations: Vec<String>, start: usize },
    PlayLibrary { ids: Vec<i64>, start: usize },
    PlayPlaylist { id: i64, start: usize },
    Enqueue { locations: Vec<String> },
    QueueItems,
    Snapshot { dir: String },
    SidecarSubtitles { path: String },
    // audio
    EqGet,
    EqApply { enabled: bool, config: EqConfig },
    EqStoreSlot { slot: char },
    EqRecallSlot { slot: char },
    EqPresets,
    EqSavePreset { name: String, genre: Option<String>, config: EqConfig },
    EqRenamePreset { old: String, new: String },
    EqDeletePreset { name: String },
    AudioDevices,
    AudioSetDevice { id: String },
    AudioSetVisualization { mode: audio::VisualizationMode },
    AudioSetNormalize { on: bool },
    // video
    MediaInfo { path: String },
    LyricsGet { path: String },
    Thumbnail { path: String, at_ms: u64, width: u32 },
    ThumbnailStrip { path: String, count: u32, width: u32 },
    ClearThumbnails,
    VideoAspect { ratio: Option<String> },
    VideoCrop { ratio: Option<String> },
    VideoAdjust { which: video::AdjustChoice, value: f32 },
    VideoDeinterlace { mode: String },
    VideoRotation { rotation: video::RotationChoice },
    // converter
    ConvertProfiles,
    ConvertCapabilities,
    ConvertAdd { request: converter::AddRequest },
    ConvertAddAudio { request: converter::AudioProcessRequest },
    ConvertList,
    ConvertCancel { id: u64 },
    ConvertRetry { id: u64 },
    ConvertRemove { id: u64 },
    ConvertClearFinished,
    ConvertCheckOutputDir { dir: String },
    // instellingen
    SettingsGet,
    SettingsSave { settings: AppSettings },
    SettingsReset,
    SettingsExport { dest: String },
    SettingsImport { src: String },
    DatabaseBackup { dest: String },
    DatabaseRestore { src: String },
    DiagnosticsExport { dest: String },
}

fn json_of<T: serde::Serialize>(v: T) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| CoreError::Invalid(format!("Antwoord kon niet worden opgebouwd: {e}")))
}

fn unit() -> Result<Value> {
    Ok(Value::Null)
}

/// Verwerkt één verzoek van de interface.
pub fn handle(st: &AppState, req: Request) -> Result<Value> {
    use Request::*;
    match req {
        AppInfo => Ok(
            json!({ "version": st.version, "player_available": st.player.is_ok(), "converter_available": st.tasks.is_some(), "warnings": st.warnings }),
        ),
        Diagnostics => json_of(settings::diagnostics(st, &st.version)?),

        LibraryAddFolder { path } => library::add_folder(st, &path).and_then(|_| unit()),
        LibraryRemoveFolder { path } => json_of(library::remove_folder(st, &path)?),
        LibraryFolders => json_of(library::list_folders(st)?),
        LibraryQuery { query } => json_of(library::query(st, &query)?),
        LibraryCount { query } => json_of(library::count(st, &query)?),
        LibraryStats => json_of(library::stats(st)?),
        LibrarySetFavorite { id, favorite } => library::set_favorite(st, id, favorite).and_then(|_| unit()),
        LibraryDuplicates => json_of(library::duplicates(st)?),
        LibraryRelink { id, path } => library::relink(st, id, &path).and_then(|_| unit()),
        LibraryRemoveMedia { id } => library::remove_media(st, id).and_then(|_| unit()),
        LibraryScanStart { folder } => library::start_scan(st, folder.as_deref()).and_then(|_| unit()),
        LibraryScanCancel => {
            library::cancel_scan(st);
            unit()
        }
        LibraryScanRunning => json_of(library::scan_running(st)),
        CollectionsList => json_of(
            st.lock_db()?
                .list_collections()?
                .into_iter()
                .map(|(id, name, query)| json!({ "id": id, "name": name, "query": query }))
                .collect::<Vec<_>>(),
        ),
        CollectionCreate { name, query } => json_of(st.lock_db()?.create_collection(&name, &query)?),
        CollectionDelete { id } => st.lock_db()?.delete_collection(id).and_then(|_| unit()),
        CollectionRun { id } => json_of(st.lock_db()?.run_collection(id)?),

        SeriesList => json_of(series::list(st)?),
        PlaylistList => json_of(playlists::list(st)?),
        PlaylistCreate { name } => json_of(playlists::create(st, &name)?),
        PlaylistRename { id, name } => playlists::rename(st, id, &name).and_then(|_| unit()),
        PlaylistDelete { id } => playlists::delete(st, id).and_then(|_| unit()),
        PlaylistDuplicate { id } => json_of(playlists::duplicate(st, id)?),
        PlaylistItems { id } => json_of(playlists::items(st, id)?),
        PlaylistAddItems { id, locations } => json_of(playlists::add_items(st, id, &locations)?),
        PlaylistRemoveItem { playlist, item } => playlists::remove_item(st, playlist, item).and_then(|_| unit()),
        PlaylistMoveItem { playlist, item, to } => playlists::move_item(st, playlist, item, to).and_then(|_| unit()),
        PlaylistImport { file, name } => json_of(playlists::import_m3u(st, &file, name.as_deref())?),
        PlaylistExport { id, dest, relative } => playlists::export_m3u(st, id, &dest, relative).and_then(|_| unit()),
        PlaylistMissing { id } => json_of(playlists::missing_items(st, id)?),

        HistoryList { limit } => json_of(history::list(st, limit)?),
        HistoryDelete { id } => history::delete_entry(st, id).and_then(|_| unit()),
        HistoryClear => history::clear(st).and_then(|_| unit()),
        HistoryEnabled => json_of(history::enabled(st)?),
        HistorySetEnabled { on } => history::set_enabled(st, on).and_then(|_| unit()),
        ContinueWatching { limit } => json_of(history::continue_watching(st, limit)?),
        MarkCompleted { location, completed } => history::mark_completed(st, &location, completed).and_then(|_| unit()),
        BookmarkAdd { location, position_ms, label } => json_of(history::bookmark_add(st, &location, position_ms, label.as_deref())?),
        BookmarkList { location } => json_of(history::bookmark_list(st, &location)?),
        BookmarkDelete { id } => history::bookmark_delete(st, id).and_then(|_| unit()),

        PlayerView => json_of(player::view(st)?),
        PlayerCommand { command } => json_of(player::command(st, command)?),
        PlayLocations { locations, start } => json_of(player::play_locations(st, locations, start)?),
        PlayLibrary { ids, start } => json_of(player::play_library(st, ids, start)?),
        PlayPlaylist { id, start } => json_of(player::play_playlist(st, id, start)?),
        Enqueue { locations } => json_of(player::enqueue_locations(st, locations)?),
        QueueItems => json_of(player::queue_items(st)?),
        Snapshot { dir } => json_of(player::snapshot(st, &dir)?),
        SidecarSubtitles { path } => json_of(player::sidecar_subtitles(&path)?),

        EqGet => json_of(audio::get(st)?),
        EqApply { enabled, config } => json_of(audio::apply(st, enabled, config)?),
        EqStoreSlot { slot } => json_of(audio::store_slot(st, slot)?),
        EqRecallSlot { slot } => json_of(audio::recall_slot(st, slot)?),
        EqPresets => json_of(audio::presets(st)?),
        EqSavePreset { name, genre, config } => audio::save_preset(st, &name, genre.as_deref(), &config).and_then(|_| unit()),
        EqRenamePreset { old, new } => audio::rename_preset(st, &old, &new).and_then(|_| unit()),
        EqDeletePreset { name } => audio::delete_preset(st, &name).and_then(|_| unit()),
        AudioDevices => json_of(audio::devices(st)?),
        AudioSetDevice { id } => audio::set_device(st, &id).and_then(|_| unit()),
        AudioSetVisualization { mode } => audio::set_visualization(st, mode).and_then(|_| unit()),
        AudioSetNormalize { on } => audio::set_normalize(st, on).and_then(|_| unit()),

        LyricsGet { path } => json_of(lyrics::get(st, &path)?),
        MediaInfo { path } => json_of(video::media_info(st, &path)?),
        Thumbnail { path, at_ms, width } => json_of(video::thumbnail(st, &path, at_ms, width)?),
        ThumbnailStrip { path, count, width } => json_of(video::thumbnail_strip(st, &path, count, width)?),
        ClearThumbnails => json_of(video::clear_thumbnail_cache(st)?),
        VideoAspect { ratio } => video::set_aspect_ratio(st, ratio.as_deref()).and_then(|_| unit()),
        VideoCrop { ratio } => video::set_crop(st, ratio.as_deref()).and_then(|_| unit()),
        VideoAdjust { which, value } => video::set_adjust(st, which, value).and_then(|_| unit()),
        VideoDeinterlace { mode } => video::set_deinterlace(st, &mode).and_then(|_| unit()),
        VideoRotation { rotation } => video::set_rotation(st, rotation).and_then(|_| unit()),

        ConvertProfiles => json_of(converter::profiles()),
        ConvertCapabilities => json_of(converter::capabilities(st)?),
        ConvertAdd { request } => json_of(converter::add(st, request)?),
        ConvertAddAudio { request } => json_of(converter::add_audio_processing(st, request)?),
        ConvertList => json_of(converter::list(st)?),
        ConvertCancel { id } => converter::cancel(st, id).and_then(|_| unit()),
        ConvertRetry { id } => converter::retry(st, id).and_then(|_| unit()),
        ConvertRemove { id } => converter::remove(st, id).and_then(|_| unit()),
        ConvertClearFinished => converter::clear_finished(st).and_then(|_| unit()),
        ConvertCheckOutputDir { dir } => converter::check_output_dir(&dir).and_then(|_| unit()),

        SettingsGet => {
            let (s, warning) = settings::get(st)?;
            Ok(json!({ "settings": s, "warning": warning }))
        }
        SettingsSave { settings: s } => json_of(settings::save(st, &s)?),
        SettingsReset => json_of(settings::reset(st)?),
        DiagnosticsExport { dest } => settings::export_diagnostics(st, &st.version, &dest).and_then(|_| unit()),
        SettingsExport { dest } => settings::export_file(st, &dest).and_then(|_| unit()),
        SettingsImport { src } => json_of(settings::import_file(st, &src)?),
        DatabaseBackup { dest } => settings::backup_database(st, &dest).and_then(|_| unit()),
        DatabaseRestore { src } => settings::restore_database(st, &src).and_then(|_| unit()),
    }
}

/// Verwerkt een ruw JSON-verzoek, zoals de interface het stuurt. Fouten komen als tekst terug.
pub fn handle_json(st: &AppState, raw: Value) -> std::result::Result<Value, String> {
    let req: Request = serde_json::from_value(raw).map_err(|e| format!("Ongeldig verzoek: {e}"))?;
    handle(st, req).map_err(|e| e.to_string())
}

/// Alle verzoektypes, voor de controle dat de TypeScript-client ze allemaal kent.
pub fn request_type_names() -> Vec<&'static str> {
    // De lijst wordt in een test vergeleken met de enum, zodat ze niet uit elkaar kunnen lopen.
    include_str!("api_names.txt").lines().filter(|l| !l.trim().is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;
    use serde_json::json;

    fn call(app: &TestApp, req: Value) -> std::result::Result<Value, String> {
        handle_json(&app.state, req)
    }

    #[test]
    fn unknown_and_malformed_requests_are_rejected_before_anything_runs() {
        let app = app();
        for bad in [
            json!({"type": "rm_rf", "path": "/"}),
            json!({"type": "library_add_folder"}),
            json!({"type": "library_add_folder", "path": 5}),
            json!({"type": "library_add_folder", "path": "/tmp", "extra": true}),
            json!({"type": "settings_save", "settings": {"ui_scale": "groot"}}),
            json!({"foo": "bar"}),
            json!("tekst"),
            json!(null),
            json!({"type": "player_command", "command": {"type": "shell", "cmd": "ls"}}),
        ] {
            let e = call(&app, bad.clone()).unwrap_err();
            assert!(e.starts_with("Ongeldig verzoek"), "{bad}: {e}");
        }
    }

    #[test]
    fn app_info_and_diagnostics_answer() {
        let app = app();
        let info = call(&app, json!({"type": "app_info"})).unwrap();
        assert_eq!(info["version"], "0.0.0-test");
        let d = call(&app, json!({"type": "diagnostics"})).unwrap();
        assert_eq!(d["schema_version"], 3);
    }

    #[test]
    fn library_flow_through_json() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        call(&app, json!({"type": "library_scan_start", "folder": dir.to_str().unwrap()})).unwrap();
        assert!(wait_until(30, || call(&app, json!({"type": "library_scan_running"})).unwrap() == json!(false)
            && !app.sink.named(crate::events::EV_SCAN_DONE).is_empty()));
        let n = call(&app, json!({"type": "library_count", "query": {}})).unwrap();
        assert_eq!(n, json!(2));
        let hits = call(&app, json!({"type": "library_query", "query": {"kind": "audio", "sort": "title"}})).unwrap();
        assert_eq!(hits.as_array().unwrap().len(), 1);
        assert_eq!(hits[0]["artist"], "Zanger");
        let id = hits[0]["id"].as_i64().unwrap();
        call(&app, json!({"type": "library_set_favorite", "id": id, "favorite": true})).unwrap();
        let fav = call(&app, json!({"type": "library_query", "query": {"favorite_only": true}})).unwrap();
        assert_eq!(fav.as_array().unwrap().len(), 1);
        // padmanipulatie via de API wordt geweigerd
        assert!(call(&app, json!({"type": "library_add_folder", "path": "/tmp/../etc"})).is_err());
    }

    #[test]
    fn settings_flow_through_json_including_validation_errors() {
        let app = app();
        let got = call(&app, json!({"type": "settings_get"})).unwrap();
        let mut s = got["settings"].clone();
        s["ui_scale"] = json!(1.5);
        let saved = call(&app, json!({"type": "settings_save", "settings": s.clone()})).unwrap();
        assert_eq!(saved["ui_scale"], 1.5);
        s["ui_scale"] = json!(9.0);
        let e = call(&app, json!({"type": "settings_save", "settings": s})).unwrap_err();
        assert!(e.contains("interfaceschaal"), "{e}");
        assert_eq!(call(&app, json!({"type": "settings_get"})).unwrap()["settings"]["ui_scale"], 1.5);
    }

    #[test]
    fn playlist_and_history_flow_through_json() {
        let app = app();
        let dir = media_dir(&app);
        let id = call(&app, json!({"type": "playlist_create", "name": "Test"})).unwrap();
        let r =
            call(&app, json!({"type": "playlist_add_items", "id": id, "locations": [dir.join("film.mkv").to_str().unwrap(), "/weg.mkv"]}))
                .unwrap();
        assert_eq!((r["added"].as_u64(), r["rejected"].as_array().unwrap().len()), (Some(1), 1));
        let items = call(&app, json!({"type": "playlist_items", "id": id})).unwrap();
        assert_eq!(items[0]["title"], "film");
        call(&app, json!({"type": "history_set_enabled", "on": false})).unwrap();
        assert_eq!(call(&app, json!({"type": "history_enabled"})).unwrap(), json!(false));
        call(&app, json!({"type": "bookmark_add", "location": "/m/a.mkv", "position_ms": 1000, "label": null})).unwrap();
        assert_eq!(call(&app, json!({"type": "bookmark_list", "location": "/m/a.mkv"})).unwrap().as_array().unwrap().len(), 1);
    }

    #[test]
    fn audio_and_converter_requests_work_through_json() {
        let app = app();
        let eq = json!({"preamp_db": -5.0, "bands_db": [0,0,0,0,4,0,0,0,0,0]});
        let v = call(&app, json!({"type": "eq_apply", "enabled": true, "config": eq})).unwrap();
        assert_eq!(v["state"]["enabled"], true);
        let bad = json!({"preamp_db": 50.0, "bands_db": [0,0,0,0,0,0,0,0,0,0]});
        assert!(call(&app, json!({"type": "eq_apply", "enabled": true, "config": bad})).is_err());
        let presets = call(&app, json!({"type": "eq_presets"})).unwrap();
        assert!(presets.as_array().unwrap().len() >= 12);
        if app.state.tasks.is_some() {
            let profiles = call(&app, json!({"type": "convert_profiles"})).unwrap();
            assert_eq!(profiles[0]["id"], "remux_mkv");
            let caps = call(&app, json!({"type": "convert_capabilities"})).unwrap();
            assert!(caps["containers"].as_array().unwrap().iter().any(|c| c["id"] == "mp4" && c["available"] == true));
            let dir = media_dir(&app);
            let out = app.dir.path().join("uit");
            let add = call(
                &app,
                json!({"type": "convert_add", "request": {
                    "inputs": [dir.join("film.mkv").to_str().unwrap()],
                    "profile_id": "remux_mp4",
                    "output": {"dir": out.to_str().unwrap(), "overwrite": "rename"}
                }}),
            )
            .unwrap();
            let id = add["job_ids"][0].as_u64().unwrap();
            assert!(wait_until(60, || {
                let list = call(&app, json!({"type": "convert_list"})).unwrap();
                list.as_array().unwrap().iter().any(|j| j["id"] == id && j["state"] == "done")
            }));
            assert!(out.join("film.mp4").exists());
            // "vervangen" bestaat als expliciete keuze; een onbekende waarde wordt geweigerd
            assert!(call(&app, json!({"type": "convert_add", "request": {"inputs": [], "profile_id": "remux_mp4", "output": {"overwrite": "verwijder_alles"}}})).is_err());
        }
    }

    #[test]
    fn player_requests_work_through_json_with_the_real_engine() {
        let app = app();
        if app.state.player.is_err() {
            return;
        }
        let dir = media_dir(&app);
        let r = call(&app, json!({"type": "play_locations", "locations": [dir.join("film.mkv").to_str().unwrap()], "start": 0})).unwrap();
        assert_eq!(r["queued"], 1);
        assert!(wait_until(10, || call(&app, json!({"type": "player_view"})).unwrap()["status"]["state"] == "playing"));
        let v = call(&app, json!({"type": "player_command", "command": {"type": "volume", "percent": 65}})).unwrap();
        assert_eq!(v["status"]["volume"], 65);
        let v = call(&app, json!({"type": "player_command", "command": {"type": "seek_to", "ms": 1500}})).unwrap();
        assert!(v["status"]["length_ms"].as_u64().unwrap() > 3000);
        let q = call(&app, json!({"type": "queue_items"})).unwrap();
        assert_eq!(q.as_array().unwrap().len(), 1);
        // de interface krijgt ook echte statusberichten van de achtergrondklok
        assert!(wait_until(5, || !app.sink.named(crate::events::EV_PLAYER_STATE).is_empty()));
    }

    #[test]
    fn every_request_type_is_listed_for_the_typescript_client() {
        // Elke variant moet in api_names.txt staan, en omgekeerd.
        let listed: std::collections::BTreeSet<&str> = request_type_names().into_iter().collect();
        let source = include_str!("api.rs");
        let enum_body = source.split("pub enum Request {").nth(1).unwrap().split("\n}\n").next().unwrap();
        let mut from_enum = std::collections::BTreeSet::new();
        for line in enum_body.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") {
                continue;
            }
            let name: String = t.chars().take_while(|c| c.is_alphanumeric()).collect();
            let snake = name.chars().enumerate().fold(String::new(), |mut acc, (i, c)| {
                if c.is_uppercase() && i > 0 {
                    acc.push('_');
                }
                acc.push(c.to_ascii_lowercase());
                acc
            });
            from_enum.insert(snake);
        }
        let listed: std::collections::BTreeSet<String> = listed.into_iter().map(String::from).collect();
        assert_eq!(listed, from_enum, "api_names.txt loopt niet gelijk met de Request-enum");
    }
}
