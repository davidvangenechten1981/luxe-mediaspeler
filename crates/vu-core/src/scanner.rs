//! Bibliotheekscanner: loopt door een map, analyseert nieuwe/gewijzigde bestanden
//! en houdt de database bij (inclusief ontbrekende bestanden).

use crate::db::{Database, MediaKind, NewMedia};
use crate::error::{CoreError, Result};
use crate::probe::{self, MediaInfo};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

pub const DEFAULT_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "webm", "mpg", "mpeg", "ts", "m2ts", "wmv", "flv", "ogv", "m4v", "mp3", "aac", "flac", "wav",
    "alac", "ogg", "opus", "wma", "aiff", "aif", "m4a",
];

pub trait Prober {
    fn probe(&self, path: &Path) -> Result<MediaInfo>;
}

pub struct FfProbe {
    pub bin: PathBuf,
}

impl Prober for FfProbe {
    fn probe(&self, path: &Path) -> Result<MediaInfo> {
        probe::probe_file(&self.bin, path)
    }
}

pub struct ScanOptions {
    pub extensions: HashSet<String>,
    pub cancel: Arc<AtomicBool>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            extensions: DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct ScanReport {
    pub added: u64,
    pub updated: u64,
    pub unchanged: u64,
    pub missing: u64,
    pub failed: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub done: u64,
    pub total: u64,
    pub current: String,
}

fn mtime_secs(m: &std::fs::Metadata) -> i64 {
    m.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn to_new_media(path: &Path, info: &MediaInfo, size: u64, mtime: i64) -> Result<NewMedia> {
    let has_audio = info.count(probe::StreamKind::Audio) > 0;
    if !info.has_video() && !has_audio {
        return Err(CoreError::Probe("geen audio of video gevonden".into()));
    }
    let kind = if info.has_video() { MediaKind::Video } else { MediaKind::Audio };
    let video = info.streams.iter().find(|s| s.kind == probe::StreamKind::Video && s.codec != "mjpeg" && s.codec != "png");
    let main = video.or_else(|| info.streams.iter().find(|s| s.kind == probe::StreamKind::Audio));
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
    Ok(NewMedia {
        path: path.to_string_lossy().into_owned(),
        kind,
        title: info.tags.title.clone().or(stem),
        artist: info.tags.artist.clone(),
        album: info.tags.album.clone(),
        genre: info.tags.genre.clone(),
        year: info.tags.year,
        track: info.tags.track,
        duration_ms: info.duration_ms,
        width: video.and_then(|v| v.width),
        height: video.and_then(|v| v.height),
        codec: main.map(|s| s.codec.clone()),
        container: info.format.split(',').next().map(str::to_string),
        bitrate: info.bitrate,
        size,
        mtime,
    })
}

pub fn scan_folder(
    db: &mut Database,
    folder: &Path,
    prober: &dyn Prober,
    opts: &ScanOptions,
    progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanReport> {
    if !folder.is_dir() {
        return Err(CoreError::NotFound(folder.display().to_string()));
    }
    // Eerst verzamelen (snel), dan analyseren (traag): zo is het totaal bekend.
    let mut candidates: Vec<(PathBuf, u64, i64)> = vec![];
    for entry in WalkDir::new(folder).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let ext = entry.path().extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if !opts.extensions.contains(&ext) {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            candidates.push((entry.path().to_path_buf(), meta.len(), mtime_secs(&meta)));
        }
    }
    candidates.sort();

    let total = candidates.len() as u64;
    let mut report = ScanReport::default();
    let mut seen: HashSet<String> = HashSet::new();
    for (i, (path, size, mtime)) in candidates.iter().enumerate() {
        if opts.cancel.load(Ordering::Relaxed) {
            return Err(CoreError::Cancelled);
        }
        let key = path.to_string_lossy().into_owned();
        progress(ScanProgress { done: i as u64, total, current: key.clone() });
        seen.insert(key.clone());
        let existing = db.get_media_by_path(&key)?;
        if let Some(m) = &existing {
            if !m.missing && m.size == *size && m.mtime == *mtime {
                report.unchanged += 1;
                continue;
            }
        }
        match prober.probe(path).and_then(|info| to_new_media(path, &info, *size, *mtime)) {
            Ok(new) => {
                db.upsert_media(&new)?;
                if existing.is_some() { report.updated += 1 } else { report.added += 1 }
            }
            Err(e) => report.failed.push((key, e.to_string())),
        }
    }
    progress(ScanProgress { done: total, total, current: String::new() });
    report.missing = db.mark_missing_in_folder(&folder.to_string_lossy(), &seen)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MediaQuery;
    use std::cell::Cell;

    struct CountingProber {
        calls: Cell<u32>,
        inner: FfProbe,
    }
    impl Prober for CountingProber {
        fn probe(&self, path: &Path) -> Result<MediaInfo> {
            self.calls.set(self.calls.get() + 1);
            self.inner.probe(path)
        }
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
    }

    fn have_ffprobe() -> bool {
        std::process::Command::new("ffprobe").arg("-version").output().is_ok()
    }

    fn prober() -> CountingProber {
        CountingProber { calls: Cell::new(0), inner: FfProbe { bin: "ffprobe".into() } }
    }

    fn setup() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::copy(fixture("sample.mkv"), dir.path().join("film.mkv")).unwrap();
        std::fs::copy(fixture("sample.mp3"), dir.path().join("sub/lied.MP3")).unwrap();
        std::fs::write(dir.path().join("notities.txt"), "geen media").unwrap();
        (dir, Database::open_in_memory().unwrap())
    }

    #[test]
    fn scans_nested_folders_by_extension_and_stores_metadata() {
        if !have_ffprobe() {
            return;
        }
        let (dir, mut db) = setup();
        let p = prober();
        let mut ticks = vec![];
        let r = scan_folder(&mut db, dir.path(), &p, &ScanOptions::default(), &mut |pr| ticks.push((pr.done, pr.total))).unwrap();
        assert_eq!((r.added, r.updated, r.unchanged, r.missing), (2, 0, 0, 0));
        assert!(r.failed.is_empty());
        assert_eq!(p.calls.get(), 2); // txt wordt niet geanalyseerd
        assert_eq!(ticks.last(), Some(&(2, 2)));

        let all = db.query_media(&MediaQuery::default()).unwrap();
        let film = all.iter().find(|m| m.path.ends_with("film.mkv")).unwrap();
        assert_eq!((film.kind, film.width, film.height), (MediaKind::Video, Some(320), Some(180)));
        assert_eq!(film.title.as_deref(), Some("Testfilm")); // titel uit tags
        let lied = all.iter().find(|m| m.path.ends_with("lied.MP3")).unwrap(); // hoofdletter-extensie
        assert_eq!((lied.kind, lied.artist.as_deref(), lied.year), (MediaKind::Audio, Some("Zanger"), Some(2020)));
    }

    #[test]
    fn rescan_skips_unchanged_files_without_probing() {
        if !have_ffprobe() {
            return;
        }
        let (dir, mut db) = setup();
        let p = prober();
        scan_folder(&mut db, dir.path(), &p, &ScanOptions::default(), &mut |_| {}).unwrap();
        let before = p.calls.get();
        let r = scan_folder(&mut db, dir.path(), &p, &ScanOptions::default(), &mut |_| {}).unwrap();
        assert_eq!((r.added, r.unchanged), (0, 2));
        assert_eq!(p.calls.get(), before);
    }

    #[test]
    fn changed_file_is_reprobed_and_deleted_file_becomes_missing() {
        if !have_ffprobe() {
            return;
        }
        let (dir, mut db) = setup();
        let p = prober();
        scan_folder(&mut db, dir.path(), &p, &ScanOptions::default(), &mut |_| {}).unwrap();
        std::fs::remove_file(dir.path().join("sub/lied.MP3")).unwrap();
        std::fs::copy(fixture("sample_small_change.mkv"), dir.path().join("film.mkv")).unwrap();
        let r = scan_folder(&mut db, dir.path(), &p, &ScanOptions::default(), &mut |_| {}).unwrap();
        assert_eq!((r.updated, r.missing), (1, 1));
        assert_eq!(db.count_media(&MediaQuery::default()).unwrap(), 1);
        assert_eq!(db.count_media(&MediaQuery { include_missing: true, ..Default::default() }).unwrap(), 2);
    }

    #[test]
    fn corrupt_files_are_reported_but_do_not_stop_the_scan() {
        if !have_ffprobe() {
            return;
        }
        let (dir, mut db) = setup();
        std::fs::write(dir.path().join("kapot.mp4"), b"dit is geen video").unwrap();
        let r = scan_folder(&mut db, dir.path(), &prober(), &ScanOptions::default(), &mut |_| {}).unwrap();
        assert_eq!(r.added, 2);
        assert_eq!(r.failed.len(), 1);
        assert!(r.failed[0].0.ends_with("kapot.mp4"));
        assert_eq!(r.missing, 0); // een kapot bestand dat nog bestaat is niet "ontbrekend"
    }

    #[test]
    fn cancellation_stops_before_work_and_keeps_library_intact() {
        let (dir, mut db) = setup();
        let opts = ScanOptions::default();
        opts.cancel.store(true, Ordering::Relaxed);
        let p = CountingProber { calls: Cell::new(0), inner: FfProbe { bin: "ffprobe".into() } };
        assert!(matches!(scan_folder(&mut db, dir.path(), &p, &opts, &mut |_| {}), Err(CoreError::Cancelled)));
        assert_eq!(p.calls.get(), 0);
    }

    #[test]
    fn missing_folder_is_an_error() {
        let mut db = Database::open_in_memory().unwrap();
        let r = scan_folder(&mut db, Path::new("/bestaat/echt/niet"), &prober(), &ScanOptions::default(), &mut |_| {});
        assert!(matches!(r, Err(CoreError::NotFound(_))));
    }
}
