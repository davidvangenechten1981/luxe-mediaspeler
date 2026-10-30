//! Zoekt een albumhoes naast een muziekbestand (`folder.jpg`, `cover.png`, …).

use std::path::{Path, PathBuf};

const NAMES: &[&str] = &["cover", "folder", "front", "album", "albumart", "thumb"];
const EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const MAX_BYTES: u64 = 15 << 20;

/// Eerste passende afbeelding in `dir`; hoofdletters maken niet uit en de voorkeursvolgorde van `NAMES` geldt.
pub fn find_folder_cover(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<(usize, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let (Some(stem), Some(ext)) = (path.file_stem()?.to_str(), path.extension().and_then(|e| e.to_str())) else { continue };
        let (stem, ext) = (stem.to_lowercase(), ext.to_lowercase());
        let Some(rank) = NAMES.iter().position(|n| *n == stem) else { continue };
        if EXTS.contains(&ext.as_str()) && entry.metadata().is_ok_and(|m| m.is_file() && m.len() > 0 && m.len() <= MAX_BYTES) {
            found.push((rank, path));
        }
    }
    found.sort();
    found.into_iter().next().map(|(_, p)| p)
}

/// Poster naast een video: `<naam>-poster.jpg`, `<naam>.jpg` of `poster.jpg` in dezelfde map.
pub fn find_video_poster(video: &Path) -> Option<PathBuf> {
    let dir = video.parent()?;
    let stem = video.file_stem()?.to_str()?.to_lowercase();
    let wanted = [format!("{stem}-poster"), stem.clone(), "poster".to_string()];
    let mut found: Vec<(usize, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let (Some(s), Some(ext)) = (path.file_stem().and_then(|s| s.to_str()), path.extension().and_then(|e| e.to_str())) else { continue };
        if !EXTS.contains(&ext.to_lowercase().as_str()) {
            continue;
        }
        if let Some(rank) = wanted.iter().position(|w| *w == s.to_lowercase()) {
            if entry.metadata().is_ok_and(|m| m.is_file() && m.len() > 0 && m.len() <= MAX_BYTES) {
                found.push((rank, path));
            }
        }
    }
    found.sort();
    found.into_iter().next().map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), b"img").unwrap();
    }

    #[test]
    fn finds_common_cover_names_case_insensitively() {
        let d = tempfile::tempdir().unwrap();
        touch(d.path(), "Folder.JPG");
        assert_eq!(find_folder_cover(d.path()).unwrap().file_name().unwrap(), "Folder.JPG");
    }

    #[test]
    fn prefers_cover_over_folder_and_ignores_other_files() {
        let d = tempfile::tempdir().unwrap();
        touch(d.path(), "folder.png");
        touch(d.path(), "cover.jpg");
        touch(d.path(), "vakantie.jpg");
        touch(d.path(), "cover.txt");
        assert_eq!(find_folder_cover(d.path()).unwrap().file_name().unwrap(), "cover.jpg");
    }

    #[test]
    fn empty_files_and_empty_folders_give_nothing() {
        let d = tempfile::tempdir().unwrap();
        assert!(find_folder_cover(d.path()).is_none());
        std::fs::write(d.path().join("cover.jpg"), b"").unwrap();
        assert!(find_folder_cover(d.path()).is_none());
        assert!(find_folder_cover(&d.path().join("bestaat-niet")).is_none());
    }

    #[test]
    fn finds_a_poster_for_a_specific_video_before_the_generic_one() {
        let d = tempfile::tempdir().unwrap();
        touch(d.path(), "poster.jpg");
        touch(d.path(), "Film.2020-poster.PNG");
        touch(d.path(), "Andere.jpg");
        let v = d.path().join("Film.2020.mkv");
        assert_eq!(find_video_poster(&v).unwrap().file_name().unwrap(), "Film.2020-poster.PNG");
        assert_eq!(find_video_poster(&d.path().join("Nieuw.mkv")).unwrap().file_name().unwrap(), "poster.jpg");
    }

    #[test]
    fn no_poster_gives_none() {
        let d = tempfile::tempdir().unwrap();
        touch(d.path(), "notities.txt");
        assert!(find_video_poster(&d.path().join("x.mkv")).is_none());
    }
}
