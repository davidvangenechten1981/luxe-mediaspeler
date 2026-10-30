//! Draagbare versie: staat er een bestand `portable.txt` naast het programma, dan komen alle gegevens
//! (bibliotheek, instellingen, logboeken) in de map `data` ernaast. Zo kun je de app op een USB-stick meenemen
//! zonder dat er iets in je Windows-profiel achterblijft.

use std::path::{Path, PathBuf};

pub const MARKER: &str = "portable.txt";

/// De gegevensmap voor een draagbare installatie, of `None` als dit een gewone installatie is.
pub fn data_dir(exe_dir: &Path) -> Option<PathBuf> {
    exe_dir.join(MARKER).is_file().then(|| exe_dir.join("data"))
}

/// Kiest de gegevensmap: eerst een expliciete override (`VU_DATA_DIR`, voor tests), dan draagbaar, dan het Windows-profiel.
pub fn resolve(env_override: Option<PathBuf>, exe_dir: Option<&Path>, default: impl FnOnce() -> Option<PathBuf>) -> Option<PathBuf> {
    env_override.or_else(|| exe_dir.and_then(data_dir)).or_else(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marker_file_makes_the_install_portable() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(data_dir(d.path()), None);
        std::fs::write(d.path().join(MARKER), "x").unwrap();
        assert_eq!(data_dir(d.path()), Some(d.path().join("data")));
    }

    #[test]
    fn a_folder_named_like_the_marker_does_not_count() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join(MARKER)).unwrap();
        assert_eq!(data_dir(d.path()), None);
    }

    #[test]
    fn the_override_beats_portable_which_beats_the_profile_folder() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(MARKER), "x").unwrap();
        let profile = || Some(PathBuf::from("/profiel"));
        assert_eq!(resolve(Some("/test".into()), Some(d.path()), profile), Some(PathBuf::from("/test")));
        assert_eq!(resolve(None, Some(d.path()), profile), Some(d.path().join("data")));
        let leeg = tempfile::tempdir().unwrap();
        assert_eq!(resolve(None, Some(leeg.path()), profile), Some(PathBuf::from("/profiel")));
        assert_eq!(resolve(None, None, profile), Some(PathBuf::from("/profiel")));
    }
}
