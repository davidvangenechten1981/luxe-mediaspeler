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
}
