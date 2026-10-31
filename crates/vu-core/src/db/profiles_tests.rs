use super::*;

#[test]
fn a_default_profile_exists_and_is_active() {
    let d = Database::open_in_memory().unwrap();
    let p = d.list_profiles().unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(d.profile(), p[0].id);
}

#[test]
fn watch_state_is_separate_per_profile() {
    let mut d = Database::open_in_memory().unwrap();
    d.save_resume("/a.mkv", 60_000, 600_000).unwrap();
    d.mark_completed("/b.mkv", true).unwrap();
    d.record_play("/a.mkv", Some("A")).unwrap();
    let kids = d.create_profile("Kinderen").unwrap();
    d.set_profile(kids).unwrap();
    assert!(d.get_resume("/a.mkv").unwrap().is_none());
    assert!(d.completed_locations().unwrap().is_empty());
    assert!(d.list_history(10).unwrap().is_empty());
    assert!(d.continue_watching(10).unwrap().is_empty());
    d.save_resume("/a.mkv", 90_000, 600_000).unwrap();
    assert_eq!(d.get_resume("/a.mkv").unwrap().unwrap().position_ms, 90_000);
    d.set_profile(1).unwrap();
    assert_eq!(d.get_resume("/a.mkv").unwrap().unwrap().position_ms, 60_000);
    assert_eq!(d.list_history(10).unwrap().len(), 1);
}

#[test]
fn the_active_profile_survives_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    let id = {
        let mut d = Database::open(&path).unwrap();
        let id = d.create_profile("Ouders").unwrap();
        d.set_profile(id).unwrap();
        id
    };
    assert_eq!(Database::open(&path).unwrap().profile(), id);
}

#[test]
fn profile_names_are_validated_and_unique() {
    let mut d = Database::open_in_memory().unwrap();
    assert!(d.create_profile("  ").is_err());
    d.create_profile("Anna").unwrap();
    assert!(d.create_profile("anna").is_err());
}

#[test]
fn deleting_removes_its_data_but_not_the_default_or_active_profile() {
    let mut d = Database::open_in_memory().unwrap();
    let id = d.create_profile("Tijdelijk").unwrap();
    d.set_profile(id).unwrap();
    d.save_resume("/a.mkv", 60_000, 600_000).unwrap();
    assert!(d.delete_profile(id).is_err()); // actief
    d.set_profile(1).unwrap();
    assert!(d.delete_profile(1).is_err()); // standaard
    d.delete_profile(id).unwrap();
    assert_eq!(d.list_profiles().unwrap().len(), 1);
    assert!(d.set_profile(id).is_err());
    let left: i64 = d.conn.query_row("SELECT COUNT(*) FROM resume WHERE profile_id = ?1", [id], |r| r.get(0)).unwrap();
    assert_eq!(left, 0);
}

#[test]
fn renaming_a_file_keeps_every_profiles_position() {
    let mut d = Database::open_in_memory().unwrap();
    let id = d
        .upsert_media(&NewMedia {
            path: "/o.mkv".into(),
            kind: MediaKind::Video,
            title: None,
            artist: None,
            album: None,
            genre: None,
            year: None,
            track: None,
            duration_ms: 1000,
            width: None,
            height: None,
            codec: None,
            container: None,
            bitrate: None,
            size: 1,
            mtime: 1,
        })
        .unwrap();
    d.save_resume("/o.mkv", 5_000, 600_000).unwrap();
    let p = d.create_profile("B").unwrap();
    d.set_profile(p).unwrap();
    d.save_resume("/o.mkv", 7_000, 600_000).unwrap();
    d.relink_media(id, "/n.mkv").unwrap();
    assert_eq!(d.get_resume("/n.mkv").unwrap().unwrap().position_ms, 7_000);
    d.set_profile(1).unwrap();
    assert_eq!(d.get_resume("/n.mkv").unwrap().unwrap().position_ms, 5_000);
}
