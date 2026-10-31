//! Tags van muziekbestanden lezen en (met reservekopie) aanpassen.

use crate::state::AppState;
use std::path::PathBuf;
use vu_core::error::Result;
use vu_core::tags::{read_tags, write_tags, CurrentTags, TagUpdate};
use vu_core::validate::validate_media_path;

pub fn get(_st: &AppState, path: &str) -> Result<CurrentTags> {
    read_tags(&validate_media_path(path)?)
}

/// Schrijft de tags en geeft het pad van de reservekopie terug.
pub fn set(st: &AppState, path: &str, update: &TagUpdate) -> Result<PathBuf> {
    let p = validate_media_path(path)?;
    write_tags(&p, update, &st.dirs.backups.join("tags"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::{app, fixture};

    #[test]
    fn reading_and_writing_go_through_path_validation_and_keep_a_backup() {
        let a = app();
        let song = a.dir.path().join("lied.mp3");
        std::fs::copy(fixture("sample.mp3"), &song).unwrap();
        let p = song.to_string_lossy().into_owned();
        let backup = set(&a.state, &p, &TagUpdate { title: Some("Anders".into()), ..Default::default() }).unwrap();
        assert!(backup.is_file());
        assert!(backup.starts_with(a.state.dirs.backups.join("tags")));
        assert_eq!(get(&a.state, &p).unwrap().title.as_deref(), Some("Anders"));
        assert!(get(&a.state, "relatief.mp3").is_err());
        assert!(set(&a.state, "relatief.mp3", &TagUpdate::default()).is_err());
    }
}
