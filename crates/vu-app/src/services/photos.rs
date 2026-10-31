//! Foto's: afbeeldingen in je bibliotheekmappen, nieuwste eerst. Ze komen niet in de database; de lijst wordt
//! bij het openen van de pagina opgebouwd. Weergave gaat via de miniatuurcache (`video::thumbnail` kan ook afbeeldingen).

use crate::state::AppState;
use serde::Serialize;
use std::time::UNIX_EPOCH;
use vu_core::error::Result;
use walkdir::WalkDir;

pub const PHOTO_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "gif"];
const MAX_PHOTOS: usize = 3000;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Photo {
    pub path: String,
    pub name: String,
    pub modified: i64,
    pub size: u64,
}

pub fn list(st: &AppState) -> Result<Vec<Photo>> {
    let folders = st.lock_db()?.list_library_folders()?;
    let mut out = vec![];
    for folder in folders {
        for e in WalkDir::new(&folder).follow_links(false).into_iter().filter_map(|e| e.ok()) {
            if !e.file_type().is_file() {
                continue;
            }
            let ext = e.path().extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if !PHOTO_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
            out.push(Photo {
                path: e.path().to_string_lossy().into_owned(),
                name: e.file_name().to_string_lossy().into_owned(),
                modified,
                size: meta.len(),
            });
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.path.cmp(&b.path)));
    out.dedup_by(|a, b| a.path == b.path);
    out.truncate(MAX_PHOTOS);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::app;

    #[test]
    fn lists_only_images_from_library_folders_newest_first() {
        let app = app();
        let dir = app.dir.path().join("fotos");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("oud.JPG"), b"x").unwrap();
        std::fs::write(dir.join("sub/nieuw.png"), b"xx").unwrap();
        std::fs::write(dir.join("film.mkv"), b"x").unwrap();
        std::fs::write(dir.join("notities.txt"), b"x").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(1000);
        std::fs::File::options().write(true).open(dir.join("oud.JPG")).unwrap().set_modified(old).unwrap();
        // een map buiten de bibliotheek telt niet mee
        let other = app.dir.path().join("andere");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("vreemd.jpg"), b"x").unwrap();
        app.state.lock_db().unwrap().add_library_folder(dir.to_str().unwrap()).unwrap();
        let names: Vec<_> = list(&app.state).unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["nieuw.png", "oud.JPG"]);
    }

    #[test]
    fn no_library_folders_gives_an_empty_list() {
        assert!(list(&app().state).unwrap().is_empty());
    }

    #[test]
    fn a_real_photo_can_be_turned_into_a_thumbnail() {
        let app = app();
        let Ok(ffmpeg) = app.state.require_ffmpeg().map(|p| p.to_path_buf()) else { return };
        let img = app.dir.path().join("f.png");
        let ok = vu_core::proc::quiet(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=red:s=640x480", "-frames:v", "1", "-y"])
            .arg(&img)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok);
        let t = crate::services::video::thumbnail(&app.state, img.to_str().unwrap(), 0, 320).unwrap();
        assert!(std::fs::metadata(t).unwrap().len() > 0);
    }
}
