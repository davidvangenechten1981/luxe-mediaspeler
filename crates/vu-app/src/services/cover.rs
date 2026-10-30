//! Albumhoes: eerst een afbeelding in dezelfde map, anders de ingebedde hoes (via FFmpeg), gecachet.

use crate::services::video::{fnv1a, run_with_timeout};
use crate::state::AppState;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use vu_core::cover::{find_folder_cover, find_video_poster};
use vu_core::db::MediaKind;
use vu_core::error::Result;
use vu_core::nfo::{parse_nfo, Nfo};
use vu_core::validate::validate_media_path;

const AUDIO_EXTS: &[&str] = &["mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma", "aiff"];

/// Pad naar een afbeelding met de hoes, of `None` als er geen is.
pub fn get(st: &AppState, path: &str) -> Result<Option<PathBuf>> {
    let media = validate_media_path(path)?;
    if let Some(poster) = find_video_poster(&media) {
        return Ok(Some(poster));
    }
    if let Some(dir) = media.parent() {
        if let Some(found) = find_folder_cover(dir) {
            return Ok(Some(found));
        }
    }
    // Bij video is "het eerste beeld" de film zelf, geen hoes: alleen muziek heeft een ingebedde hoes.
    let kind = st.lock_db()?.get_media_by_path(path)?.map(|m| m.kind);
    let ext = media.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let is_audio = kind == Some(MediaKind::Audio) || (kind.is_none() && AUDIO_EXTS.contains(&ext.as_str()));
    if !is_audio {
        return Ok(None);
    }
    let Ok(ffmpeg) = st.require_ffmpeg() else { return Ok(None) };
    let meta = std::fs::metadata(&media)?;
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let key = fnv1a(format!("cover|{}|{}|{}", media.display(), meta.len(), mtime).as_bytes());
    let dir = st.dirs.thumbnails();
    std::fs::create_dir_all(&dir)?;
    let out = dir.join(format!("cover-{key:016x}.jpg"));
    if std::fs::metadata(&out).is_ok_and(|m| m.len() > 0) {
        return Ok(Some(out));
    }
    let tmp = out.with_extension("tmp.jpg");
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"])
        .arg(&media)
        .args(["-map", "0:v:0", "-frames:v", "1", "-vf", "scale=600:-2", "-q:v", "3", "-y"])
        .arg(&tmp);
    // Geen ingebedde hoes is geen fout: FFmpeg stopt dan met een foutcode.
    if run_with_timeout(cmd, Duration::from_secs(15)).is_err() || std::fs::metadata(&tmp).map_or(true, |m| m.len() == 0) {
        let _ = std::fs::remove_file(&tmp);
        return Ok(None);
    }
    std::fs::rename(&tmp, &out)?;
    Ok(Some(out))
}

/// Gegevens uit `<naam>.nfo` of `movie.nfo` naast een film of aflevering.
pub fn nfo(_st: &AppState, path: &str) -> Result<Option<Nfo>> {
    let media = validate_media_path(path)?;
    let candidates = [media.with_extension("nfo"), media.with_file_name("movie.nfo")];
    for c in candidates {
        if std::fs::metadata(&c).is_ok_and(|m| m.is_file() && m.len() <= 1 << 20) {
            if let Some(n) = std::fs::read(&c).ok().and_then(|b| parse_nfo(&String::from_utf8_lossy(&b))) {
                return Ok(Some(n));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::{app, fixture};

    #[test]
    fn uses_a_folder_image_when_present() {
        let a = app();
        let song = a.dir.path().join("lied.mp3");
        std::fs::copy(fixture("sample.mp3"), &song).unwrap();
        std::fs::write(a.dir.path().join("folder.jpg"), b"img").unwrap();
        let c = get(&a.state, &song.to_string_lossy()).unwrap().unwrap();
        assert_eq!(c.file_name().unwrap(), "folder.jpg");
    }

    #[test]
    fn a_file_without_any_cover_gives_none_not_an_error() {
        let a = app();
        let song = a.dir.path().join("kaal.mp3");
        std::fs::copy(fixture("sample.mp3"), &song).unwrap();
        assert!(get(&a.state, &song.to_string_lossy()).unwrap().is_none());
    }

    #[test]
    fn extracts_an_embedded_cover_and_caches_it() {
        let a = app();
        let Some(ffmpeg) = a.state.require_ffmpeg().ok() else { return };
        let img = a.dir.path().join("src.png");
        let ok = Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=red:s=64x64", "-frames:v", "1", "-y"])
            .arg(&img)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let sub = a.dir.path().join("muziek");
        std::fs::create_dir(&sub).unwrap();
        let song = sub.join("met-hoes.mp3");
        let ok = Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-i"])
            .arg(fixture("sample.mp3"))
            .arg("-i")
            .arg(&img)
            .args(["-map", "0:a", "-map", "1:v", "-c", "copy", "-id3v2_version", "3", "-y"])
            .arg(&song)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let c = get(&a.state, &song.to_string_lossy()).unwrap().unwrap();
        assert!(std::fs::metadata(&c).unwrap().len() > 0);
        let again = get(&a.state, &song.to_string_lossy()).unwrap().unwrap();
        assert_eq!(c, again);
    }

    #[test]
    fn a_film_poster_is_used_and_videos_never_get_their_own_frame_as_cover() {
        let a = app();
        let film = a.dir.path().join("film.mkv");
        std::fs::copy(fixture("sample.mkv"), &film).unwrap();
        assert!(get(&a.state, &film.to_string_lossy()).unwrap().is_none());
        std::fs::write(a.dir.path().join("film-poster.jpg"), b"img").unwrap();
        assert_eq!(get(&a.state, &film.to_string_lossy()).unwrap().unwrap().file_name().unwrap(), "film-poster.jpg");
    }

    #[test]
    fn nfo_next_to_the_film_is_read() {
        let a = app();
        let film = a.dir.path().join("film.mkv");
        std::fs::write(&film, b"x").unwrap();
        assert!(nfo(&a.state, &film.to_string_lossy()).unwrap().is_none());
        std::fs::write(a.dir.path().join("film.nfo"), "<movie><title>Mooi</title><year>2001</year></movie>").unwrap();
        let n = nfo(&a.state, &film.to_string_lossy()).unwrap().unwrap();
        assert_eq!((n.title.as_deref(), n.year), (Some("Mooi"), Some(2001)));
    }
}
