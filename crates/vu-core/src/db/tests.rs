use super::*;
use std::collections::HashSet;

fn media(path: &str, kind: MediaKind) -> NewMedia {
    NewMedia {
        path: path.into(),
        kind,
        title: Some(path.rsplit('/').next().unwrap().into()),
        artist: None,
        album: None,
        genre: None,
        year: None,
        track: None,
        duration_ms: 60_000,
        width: if kind == MediaKind::Video { Some(1920) } else { None },
        height: if kind == MediaKind::Video { Some(1080) } else { None },
        codec: Some("h264".into()),
        container: Some("matroska".into()),
        bitrate: Some(1000),
        size: 1000,
        mtime: 1,
    }
}

fn db() -> Database {
    Database::open_in_memory().unwrap()
}

#[test]
fn new_database_is_migrated_to_latest_version() {
    let d = db();
    assert_eq!(d.schema_version().unwrap() as usize, MIGRATIONS.len());
    d.integrity_check().unwrap();
}

#[test]
fn reopening_a_file_database_keeps_data_and_does_not_remigrate() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("lib.db");
    {
        let mut d = Database::open(&p).unwrap();
        d.upsert_media(&media("/m/a.mkv", MediaKind::Video)).unwrap();
    }
    let d = Database::open(&p).unwrap();
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 1);
}

#[test]
fn database_from_a_newer_app_version_is_refused_without_changes() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("future.db");
    {
        let c = Connection::open(&p).unwrap();
        c.pragma_update(None, "user_version", 999).unwrap();
    }
    match Database::open(&p) {
        Err(CoreError::Invalid(m)) => assert!(m.contains("nieuwere versie")),
        other => panic!("verwachtte weigering, kreeg {:?}", other.map(|_| ())),
    }
}

#[test]
fn upsert_updates_existing_row_and_keeps_favorite_and_play_count() {
    let mut d = db();
    let id = d.upsert_media(&media("/m/a.mkv", MediaKind::Video)).unwrap();
    d.set_favorite(id, true).unwrap();
    d.record_play("/m/a.mkv", Some("a")).unwrap();
    let mut changed = media("/m/a.mkv", MediaKind::Video);
    changed.title = Some("Nieuwe titel".into());
    let id2 = d.upsert_media(&changed).unwrap();
    assert_eq!(id, id2);
    let m = d.get_media(id).unwrap().unwrap();
    assert_eq!(m.title.as_deref(), Some("Nieuwe titel"));
    assert!(m.favorite);
    assert_eq!(m.play_count, 1);
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 1);
}

#[test]
fn search_requires_every_word_and_treats_wildcards_literally() {
    let mut d = db();
    let mut a = media("/m/Zomer Vakantie 2023.mp4", MediaKind::Video);
    a.artist = Some("Jan".into());
    d.upsert_media(&a).unwrap();
    d.upsert_media(&media("/m/100%_echt.mp4", MediaKind::Video)).unwrap();
    d.upsert_media(&media("/m/andere.mp4", MediaKind::Video)).unwrap();
    let q = |t: &str| MediaQuery { text: Some(t.into()), ..Default::default() };
    assert_eq!(d.query_media(&q("zomer 2023")).unwrap().len(), 1);
    assert_eq!(d.query_media(&q("jan")).unwrap().len(), 1); // op artiest
    assert_eq!(d.query_media(&q("zomer onzin")).unwrap().len(), 0);
    assert_eq!(d.query_media(&q("%")).unwrap().len(), 1); // alleen het item met een echte %
    assert_eq!(d.query_media(&q("_")).unwrap().len(), 1);
    assert_eq!(d.query_media(&q("'; DROP TABLE media; --")).unwrap().len(), 0);
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 3);
}

#[test]
fn filters_by_kind_duration_height_and_favorites() {
    let mut d = db();
    let v = d.upsert_media(&media("/m/film.mkv", MediaKind::Video)).unwrap();
    let mut short = media("/m/clip.mkv", MediaKind::Video);
    short.duration_ms = 5_000;
    short.height = Some(480);
    d.upsert_media(&short).unwrap();
    d.upsert_media(&media("/m/lied.mp3", MediaKind::Audio)).unwrap();
    d.set_favorite(v, true).unwrap();
    let count = |q: MediaQuery| d.query_media(&q).unwrap().len();
    assert_eq!(count(MediaQuery { kind: Some(MediaKind::Audio), ..Default::default() }), 1);
    assert_eq!(count(MediaQuery { min_duration_ms: Some(10_000), kind: Some(MediaKind::Video), ..Default::default() }), 1);
    assert_eq!(count(MediaQuery { min_height: Some(1080), ..Default::default() }), 1);
    assert_eq!(count(MediaQuery { favorite_only: true, ..Default::default() }), 1);
}

#[test]
fn sorts_and_paginates_stably() {
    let mut d = db();
    for (i, name) in ["c", "a", "b"].iter().enumerate() {
        let mut m = media(&format!("/m/{name}.mkv"), MediaKind::Video);
        m.size = (i as u64 + 1) * 100;
        d.upsert_media(&m).unwrap();
    }
    let titles = |q: MediaQuery| d.query_media(&q).unwrap().into_iter().map(|m| m.title.unwrap()).collect::<Vec<_>>();
    assert_eq!(titles(MediaQuery::default()), ["a.mkv", "b.mkv", "c.mkv"]);
    assert_eq!(titles(MediaQuery { sort: SortField::Size, descending: true, ..Default::default() }), ["b.mkv", "a.mkv", "c.mkv"]);
    assert_eq!(titles(MediaQuery { limit: Some(1), offset: Some(1), ..Default::default() }), ["b.mkv"]);
}

#[test]
fn missing_files_are_flagged_hidden_and_restored_on_rescan() {
    let mut d = db();
    d.upsert_media(&media("/lib/a.mkv", MediaKind::Video)).unwrap();
    d.upsert_media(&media("/lib/b.mkv", MediaKind::Video)).unwrap();
    d.upsert_media(&media("/other/c.mkv", MediaKind::Video)).unwrap();
    let seen: HashSet<String> = ["/lib/a.mkv".to_string()].into();
    assert_eq!(d.mark_missing_in_folder("/lib", &seen).unwrap(), 1);
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 2); // b verborgen, c blijft
    assert_eq!(d.count_media(&MediaQuery { include_missing: true, ..Default::default() }).unwrap(), 3);
    d.upsert_media(&media("/lib/b.mkv", MediaKind::Video)).unwrap();
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 3);
}

#[test]
fn folder_prefix_does_not_match_sibling_folders() {
    let mut d = db();
    d.upsert_media(&media("/lib/a.mkv", MediaKind::Video)).unwrap();
    d.upsert_media(&media("/library2/b.mkv", MediaKind::Video)).unwrap();
    d.add_library_folder("/lib").unwrap();
    assert_eq!(d.remove_library_folder("/lib").unwrap(), 1);
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 1);
    assert!(d.list_library_folders().unwrap().is_empty());
}

#[test]
fn relinking_moves_playlist_items_and_resume_with_the_file() {
    let mut d = db();
    let id = d.upsert_media(&media("/old/a.mkv", MediaKind::Video)).unwrap();
    let pl = d.create_playlist("lijst").unwrap();
    d.add_playlist_items(pl, &[("/old/a.mkv".into(), None)]).unwrap();
    d.save_resume("/old/a.mkv", 30_000, 100_000).unwrap();
    d.relink_media(id, "/new/a.mkv").unwrap();
    assert_eq!(d.playlist_items(pl).unwrap()[0].location, "/new/a.mkv");
    assert!(d.get_resume("/new/a.mkv").unwrap().is_some());
    assert!(d.get_resume("/old/a.mkv").unwrap().is_none());
    // een tweede item mag niet op een bestaand pad worden gekoppeld
    let other = d.upsert_media(&media("/x/b.mkv", MediaKind::Video)).unwrap();
    assert!(d.relink_media(other, "/new/a.mkv").is_err());
}

#[test]
fn finds_duplicates_by_size_and_duration() {
    let mut d = db();
    for p in ["/a/x.mkv", "/b/x-kopie.mkv"] {
        d.upsert_media(&media(p, MediaKind::Video)).unwrap();
    }
    let mut uniek = media("/a/y.mkv", MediaKind::Video);
    uniek.size = 5;
    d.upsert_media(&uniek).unwrap();
    let groups = d.find_duplicates().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].len(), 2);
}

#[test]
fn resume_saves_position_and_marks_near_end_as_completed() {
    let d = db();
    d.save_resume("/m/a.mkv", 30_000, 100_000).unwrap();
    let r = d.get_resume("/m/a.mkv").unwrap().unwrap();
    assert_eq!((r.position_ms, r.completed), (30_000, false));
    d.save_resume("/m/a.mkv", 97_000, 100_000).unwrap(); // laatste 5%
    let r = d.get_resume("/m/a.mkv").unwrap().unwrap();
    assert_eq!((r.position_ms, r.completed), (0, true));
    d.save_resume("/m/kort.mkv", 55_000, 60_000).unwrap(); // laatste 10 s
    assert!(d.get_resume("/m/kort.mkv").unwrap().unwrap().completed);
}

#[test]
fn continue_watching_lists_only_unfinished_with_real_progress() {
    let d = db();
    d.save_resume("/m/a.mkv", 30_000, 100_000).unwrap();
    d.save_resume("/m/b.mkv", 2_000, 100_000).unwrap(); // te weinig voortgang
    d.save_resume("/m/c.mkv", 99_000, 100_000).unwrap(); // voltooid
    let list = d.continue_watching(10).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].location, "/m/a.mkv");
    d.mark_completed("/m/a.mkv", true).unwrap();
    assert!(d.continue_watching(10).unwrap().is_empty());
}

#[test]
fn history_can_be_disabled_cleared_and_pruned_per_item() {
    let mut d = db();
    d.upsert_media(&media("/m/a.mkv", MediaKind::Video)).unwrap();
    d.record_play("/m/a.mkv", Some("a")).unwrap();
    d.record_play("/m/b.mkv", None).unwrap();
    let h = d.list_history(10).unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].location, "/m/b.mkv"); // nieuwste eerst
    d.delete_history_entry(h[0].id).unwrap();
    assert_eq!(d.list_history(10).unwrap().len(), 1);

    d.set_setting("history_enabled", "false").unwrap();
    d.record_play("/m/c.mkv", None).unwrap();
    d.save_resume("/m/c.mkv", 30_000, 100_000).unwrap();
    assert_eq!(d.list_history(10).unwrap().len(), 1); // niets erbij
    assert!(d.get_resume("/m/c.mkv").unwrap().is_none());
    d.set_setting("history_enabled", "true").unwrap();

    d.save_resume("/m/a.mkv", 20_000, 100_000).unwrap();
    d.clear_history().unwrap();
    assert!(d.list_history(10).unwrap().is_empty());
    assert!(d.get_resume("/m/a.mkv").unwrap().is_none());
    let m = d.get_media_by_path("/m/a.mkv").unwrap().unwrap();
    assert_eq!((m.play_count, m.last_played), (0, None)); // bibliotheek zelf blijft
}

#[test]
fn playlist_create_rename_duplicate_delete() {
    let mut d = db();
    let id = d.create_playlist("  Mijn lijst  ").unwrap();
    assert_eq!(d.list_playlists().unwrap()[0].name, "Mijn lijst");
    assert!(d.create_playlist("   ").is_err());
    assert!(d.create_playlist(&"x".repeat(200)).is_err());
    d.rename_playlist(id, "Nieuwe naam").unwrap();
    assert!(d.rename_playlist(9999, "x").is_err());
    d.add_playlist_items(id, &[("/m/a.mkv".into(), Some("A".into())), ("/m/b.mkv".into(), None)]).unwrap();
    let copy = d.duplicate_playlist(id).unwrap();
    assert_eq!(d.playlist_items(copy).unwrap().len(), 2);
    assert_eq!(d.list_playlists().unwrap().len(), 2);
    d.delete_playlist(id).unwrap();
    assert_eq!(d.playlist_items(id).unwrap().len(), 0); // cascade
    assert_eq!(d.playlist_items(copy).unwrap().len(), 2); // kopie blijft
}

#[test]
fn playlist_items_keep_order_allow_duplicates_and_can_be_moved_and_removed() {
    let mut d = db();
    let id = d.create_playlist("p").unwrap();
    let items: Vec<(String, Option<String>)> = ["a", "b", "c", "a"].iter().map(|n| (format!("/m/{n}.mkv"), None)).collect();
    d.add_playlist_items(id, &items).unwrap();
    let locs = |d: &Database| d.playlist_items(id).unwrap().into_iter().map(|i| i.location.chars().nth(3).unwrap()).collect::<String>();
    assert_eq!(locs(&d), "abca");
    let first = d.playlist_items(id).unwrap()[0].id;
    d.move_playlist_item(id, first, 2).unwrap();
    assert_eq!(locs(&d), "bcaa");
    d.move_playlist_item(id, first, 99).unwrap(); // voorbij het einde: achteraan
    assert_eq!(locs(&d), "bcaa");
    let b = d.playlist_items(id).unwrap()[0].id;
    d.remove_playlist_item(id, b).unwrap();
    let rest = d.playlist_items(id).unwrap();
    assert_eq!(rest.iter().map(|i| i.position).collect::<Vec<_>>(), [0, 1, 2]); // geen gaten
    assert!(d.add_playlist_items(9999, &items).is_err());
}

#[test]
fn bookmarks_are_listed_in_time_order_per_file() {
    let d = db();
    d.add_bookmark("/m/a.mkv", 90_000, Some("Einde")).unwrap();
    let first = d.add_bookmark("/m/a.mkv", 10_000, None).unwrap();
    d.add_bookmark("/m/b.mkv", 5_000, None).unwrap();
    let list = d.list_bookmarks("/m/a.mkv").unwrap();
    assert_eq!(list.iter().map(|b| b.position_ms).collect::<Vec<_>>(), [10_000, 90_000]);
    d.delete_bookmark(first).unwrap();
    assert_eq!(d.list_bookmarks("/m/a.mkv").unwrap().len(), 1);
}

#[test]
fn settings_roundtrip_and_overwrite() {
    let d = db();
    assert_eq!(d.get_setting("thema").unwrap(), None);
    d.set_setting("thema", "donker").unwrap();
    d.set_setting("thema", "licht").unwrap();
    assert_eq!(d.get_setting("thema").unwrap().as_deref(), Some("licht"));
    assert_eq!(d.all_settings().unwrap(), vec![("thema".to_string(), "licht".to_string())]);
}

#[test]
fn backup_and_restore_roundtrip_and_refuse_garbage() {
    let dir = tempfile::tempdir().unwrap();
    let backup = dir.path().join("backup.db");
    let mut d = db();
    d.upsert_media(&media("/m/a.mkv", MediaKind::Video)).unwrap();
    d.backup_to(&backup).unwrap();
    d.upsert_media(&media("/m/b.mkv", MediaKind::Video)).unwrap();
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 2);
    d.restore_from(&backup).unwrap();
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 1);

    let garbage = dir.path().join("kapot.db");
    std::fs::write(&garbage, b"dit is geen database").unwrap();
    assert!(d.restore_from(&garbage).is_err());
    assert_eq!(d.count_media(&MediaQuery::default()).unwrap(), 1); // ongewijzigd
}

#[test]
fn unknown_ids_give_clear_errors() {
    let d = db();
    assert!(matches!(d.set_favorite(42, true), Err(CoreError::NotFound(_))));
    assert!(d.get_media(42).unwrap().is_none());
}
