//! Songteksten naast het muziekbestand: `lied.lrc` (met tijden) of `lied.txt` (gewone tekst).

use crate::state::AppState;
use serde::Serialize;
use std::path::Path;
use vu_core::error::Result;
use vu_core::lyrics::{parse_lrc, LyricLine};
use vu_core::validate::validate_media_path;

#[derive(Debug, Serialize)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    /// Alleen true als de tekst tijden heeft en dus mee kan scrollen.
    pub synced: bool,
}

const MAX_BYTES: u64 = 1 << 20;

pub fn get(_st: &AppState, path: &str) -> Result<Option<Lyrics>> {
    let media = validate_media_path(path)?;
    for ext in ["lrc", "txt"] {
        let candidate = media.with_extension(ext);
        if let Some(text) = read_small(&candidate) {
            let lines = parse_lrc(&text);
            if !lines.is_empty() {
                let synced = lines.iter().any(|l| l.time_ms.is_some());
                return Ok(Some(Lyrics { lines, synced }));
            }
        }
    }
    Ok(None)
}

fn read_small(p: &Path) -> Option<String> {
    let meta = std::fs::metadata(p).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    Some(String::from_utf8_lossy(&std::fs::read(p).ok()?).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::app;

    #[test]
    fn finds_a_synced_lrc_next_to_the_song() {
        let a = app();
        let song = a.dir.path().join("lied.mp3");
        std::fs::write(&song, b"x").unwrap();
        std::fs::write(a.dir.path().join("lied.lrc"), "[00:01.00]Hallo\n[00:02.00]Wereld").unwrap();
        let l = get(&a.state, &song.to_string_lossy()).unwrap().unwrap();
        assert!(l.synced);
        assert_eq!(l.lines.len(), 2);
    }

    #[test]
    fn falls_back_to_plain_text_and_reports_not_synced() {
        let a = app();
        let song = a.dir.path().join("lied.flac");
        std::fs::write(&song, b"x").unwrap();
        std::fs::write(a.dir.path().join("lied.txt"), "regel een\nregel twee").unwrap();
        let l = get(&a.state, &song.to_string_lossy()).unwrap().unwrap();
        assert!(!l.synced);
    }

    #[test]
    fn no_lyrics_file_is_not_an_error() {
        let a = app();
        let song = a.dir.path().join("stil.mp3");
        std::fs::write(&song, b"x").unwrap();
        assert!(get(&a.state, &song.to_string_lossy()).unwrap().is_none());
    }

    #[test]
    fn rejects_paths_that_are_not_media_files() {
        let a = app();
        assert!(get(&a.state, "relatief/pad.mp3").is_err());
    }
}
