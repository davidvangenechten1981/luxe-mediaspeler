//! Tags (titel, artiest, album, …) van muziekbestanden bewerken. Er wordt altijd eerst een reservekopie gemaakt.

use crate::error::{CoreError, Result};
use lofty::config::WriteOptions;
use lofty::file::{TaggedFile, TaggedFileExt};
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Een veld dat `None` is blijft ongewijzigd; een lege tekst wist het veld.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TagUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CurrentTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
}

const AUDIO_EXTS: &[&str] = &["mp3", "flac", "m4a", "ogg", "opus", "wav", "aiff"];

fn check_ext(path: &Path) -> Result<()> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if AUDIO_EXTS.contains(&ext.as_str()) {
        Ok(())
    } else {
        Err(CoreError::Invalid("Tags bewerken kan alleen bij mp3, flac, m4a, ogg, opus, wav en aiff".into()))
    }
}

fn open(path: &Path) -> Result<TaggedFile> {
    lofty::read_from_path(path).map_err(|e| CoreError::Invalid(format!("Kan de tags niet lezen: {e}")))
}

fn year_of(tag: &Tag) -> Option<u32> {
    [ItemKey::Year, ItemKey::RecordingDate].iter().find_map(|k| tag.get_string(*k)).and_then(|s| s.get(..4)?.parse().ok())
}

pub fn read_tags(path: &Path) -> Result<CurrentTags> {
    check_ext(path)?;
    let file = open(path)?;
    let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) else { return Ok(CurrentTags::default()) };
    Ok(CurrentTags {
        title: tag.title().map(|s| s.into_owned()),
        artist: tag.artist().map(|s| s.into_owned()),
        album: tag.album().map(|s| s.into_owned()),
        genre: tag.genre().map(|s| s.into_owned()),
        year: year_of(tag),
        track: tag.track(),
    })
}

fn set_text(tag: &mut Tag, value: &Option<String>, set: fn(&mut Tag, String), remove: fn(&mut Tag)) {
    match value {
        None => {}
        Some(v) if v.trim().is_empty() => remove(tag),
        Some(v) => set(tag, v.trim().to_string()),
    }
}

/// Past de tags aan. Maakt eerst een reservekopie in `backup_dir` en geeft het pad ervan terug.
pub fn write_tags(path: &Path, update: &TagUpdate, backup_dir: &Path) -> Result<PathBuf> {
    check_ext(path)?;
    if update.year.is_some_and(|y| !(1000..=9999).contains(&y)) {
        return Err(CoreError::Invalid("Het jaar moet vier cijfers hebben".into()));
    }
    if update.track.is_some_and(|t| t == 0 || t > 999) {
        return Err(CoreError::Invalid("Het spoornummer moet tussen 1 en 999 liggen".into()));
    }
    let mut file = open(path)?; // eerst lezen: een kapot bestand laten we met rust
    std::fs::create_dir_all(backup_dir)?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("bestand");
    let backup = backup_dir.join(format!("{stamp}-{name}"));
    std::fs::copy(path, &backup)?;
    if file.primary_tag().is_none() {
        let tt = file.primary_tag_type();
        file.insert_tag(Tag::new(tt));
    }
    let tag = file.primary_tag_mut().ok_or_else(|| CoreError::Invalid("Dit bestandstype ondersteunt geen tags".into()))?;
    set_text(tag, &update.title, |t, v| t.set_title(v), |t| t.remove_title());
    set_text(tag, &update.artist, |t, v| t.set_artist(v), |t| t.remove_artist());
    set_text(tag, &update.album, |t, v| t.set_album(v), |t| t.remove_album());
    set_text(tag, &update.genre, |t, v| t.set_genre(v), |t| t.remove_genre());
    if let Some(y) = update.year {
        tag.insert_text(ItemKey::Year, y.to_string());
        tag.insert_text(ItemKey::RecordingDate, y.to_string());
    }
    if let Some(n) = update.track {
        tag.set_track(n);
    }
    if let Err(e) = file.save_to_path(path, WriteOptions::default()) {
        // Terugzetten: het bestand mag er niet slechter van worden.
        let _ = std::fs::copy(&backup, path);
        return Err(CoreError::Invalid(format!("Opslaan mislukt, het origineel is teruggezet: {e}")));
    }
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(dir: &Path) -> PathBuf {
        let dst = dir.join("lied.mp3");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.mp3"), &dst).unwrap();
        dst
    }

    #[test]
    fn edits_tags_and_keeps_a_backup_of_the_original() {
        let d = tempfile::tempdir().unwrap();
        let song = sample(d.path());
        let original = std::fs::read(&song).unwrap();
        let backup = write_tags(
            &song,
            &TagUpdate {
                title: Some("Nieuw lied".into()),
                artist: Some("Nieuwe band".into()),
                year: Some(2021),
                track: Some(7),
                ..Default::default()
            },
            &d.path().join("backup"),
        )
        .unwrap();
        let t = read_tags(&song).unwrap();
        assert_eq!(
            (t.title.as_deref(), t.artist.as_deref(), t.year, t.track),
            (Some("Nieuw lied"), Some("Nieuwe band"), Some(2021), Some(7))
        );
        assert_eq!(std::fs::read(backup).unwrap(), original);
    }

    #[test]
    fn untouched_fields_stay_and_empty_text_clears_a_field() {
        let d = tempfile::tempdir().unwrap();
        let song = sample(d.path());
        write_tags(&song, &TagUpdate { title: Some("Een".into()), album: Some("Plaat".into()), ..Default::default() }, &d.path().join("b"))
            .unwrap();
        write_tags(&song, &TagUpdate { album: Some("  ".into()), ..Default::default() }, &d.path().join("b")).unwrap();
        let t = read_tags(&song).unwrap();
        assert_eq!(t.title.as_deref(), Some("Een"));
        assert_eq!(t.album, None);
    }

    #[test]
    fn bad_input_and_wrong_file_types_are_refused_without_touching_anything() {
        let d = tempfile::tempdir().unwrap();
        let song = sample(d.path());
        let before = std::fs::read(&song).unwrap();
        assert!(write_tags(&song, &TagUpdate { year: Some(20), ..Default::default() }, &d.path().join("b")).is_err());
        assert!(write_tags(&song, &TagUpdate { track: Some(0), ..Default::default() }, &d.path().join("b")).is_err());
        assert_eq!(std::fs::read(&song).unwrap(), before);
        let film = d.path().join("film.mkv");
        std::fs::write(&film, b"x").unwrap();
        assert!(write_tags(&film, &TagUpdate::default(), &d.path().join("b")).is_err());
        let broken = d.path().join("kapot.mp3");
        std::fs::write(&broken, b"geen mp3").unwrap();
        assert!(write_tags(&broken, &TagUpdate { title: Some("x".into()), ..Default::default() }, &d.path().join("b")).is_err());
        assert_eq!(std::fs::read(&broken).unwrap(), b"geen mp3");
    }
}
