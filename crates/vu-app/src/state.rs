//! Gedeelde staat van de applicatie: database, tools, wachtrij, speler.

use crate::events::{EventSink, EV_CONVERT_JOB};
use crate::player_host::PlayerHost;
use crate::tools::Tools;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use vu_core::convert::queue::TaskManager;
use vu_core::db::Database;
use vu_core::error::{CoreError, Result};
use vu_core::settings::{self, AppSettings};

#[derive(Debug, Clone)]
pub struct AppDirs {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub backups: PathBuf,
}

impl AppDirs {
    pub fn new(base: &Path) -> Result<AppDirs> {
        let d = AppDirs { data: base.join("data"), cache: base.join("cache"), logs: base.join("logs"), backups: base.join("backups") };
        for p in [&d.data, &d.cache, &d.logs, &d.backups] {
            std::fs::create_dir_all(p)?;
        }
        Ok(d)
    }
    pub fn db_path(&self) -> PathBuf {
        self.data.join("library.db")
    }
    pub fn thumbnails(&self) -> PathBuf {
        self.cache.join("thumbnails")
    }
}

pub struct ScanRun {
    pub cancel: Arc<AtomicBool>,
    pub handle: JoinHandle<()>,
}

pub struct AppState {
    pub version: String,
    pub dirs: AppDirs,
    pub tools: Tools,
    pub sink: Arc<dyn EventSink>,
    pub db: Arc<Mutex<Database>>,
    pub tasks: Option<TaskManager>,
    pub player: std::result::Result<PlayerHost, String>,
    pub scan: Mutex<Option<ScanRun>>,
    /// Meldingen van het opstarten (bijv. een beschadigde database die is hersteld).
    pub warnings: Vec<String>,
}

/// Opent de database. Een beschadigd bestand wordt hernoemd (niet verwijderd) en er komt een nieuwe.
pub fn open_database_recovering(path: &Path) -> Result<(Database, Option<String>)> {
    let attempt = Database::open(path).and_then(|d| d.integrity_check().map(|_| d));
    match attempt {
        Ok(d) => Ok((d, None)),
        Err(e) if path.exists() => {
            let stamp = vu_core::db::now();
            let aside = path.with_extension(format!("db.beschadigd-{stamp}"));
            std::fs::rename(path, &aside)?;
            // WAL- en SHM-bestanden horen bij het oude bestand
            for ext in ["db-wal", "db-shm"] {
                let side = path.with_extension(ext);
                if side.exists() {
                    let _ = std::fs::rename(&side, aside.with_extension(format!("{ext}-{stamp}")));
                }
            }
            let fresh = Database::open(path)?;
            Ok((
                fresh,
                Some(format!(
                    "De database was beschadigd ({e}). Een nieuwe database is aangemaakt; het oude bestand is bewaard als {}.",
                    aside.display()
                )),
            ))
        }
        Err(e) => Err(e),
    }
}

/// Maakt een back-up als de laatste ouder is dan `max_age_days`; houdt de nieuwste `keep` bestanden.
pub fn rotate_backups(db: &Database, dir: &Path, max_age_days: i64, keep: usize) -> Result<Option<PathBuf>> {
    let mut existing: Vec<(PathBuf, std::time::SystemTime)> = std::fs::read_dir(dir)?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("library-") && e.path().extension().is_some_and(|x| x == "db"))
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
        .collect();
    existing.sort_by_key(|(_, t)| *t);
    let newest_age_secs = existing.last().and_then(|(_, t)| t.elapsed().ok()).map(|d| d.as_secs());
    let mut created = None;
    if newest_age_secs.is_none_or(|s| s as i64 >= max_age_days * 86_400) {
        let target = dir.join(format!("library-{}.db", vu_core::db::now()));
        db.backup_to(&target)?;
        existing.push((target.clone(), std::time::SystemTime::now()));
        created = Some(target);
    }
    while existing.len() > keep {
        let (old, _) = existing.remove(0);
        let _ = std::fs::remove_file(old);
    }
    Ok(created)
}

impl AppState {
    /// Start alle onderdelen. Ontbrekende tools geven een duidelijke melding in plaats van een crash.
    pub fn init(version: &str, dirs: AppDirs, tools: Tools, sink: Arc<dyn EventSink>, vlc_args: Vec<String>) -> Result<AppState> {
        let mut warnings = vec![];
        let (db, warn) = open_database_recovering(&dirs.db_path())?;
        warnings.extend(warn);
        let _ = rotate_backups(&db, &dirs.backups, 7, 5);
        let (app_settings, settings_warn) = settings::load(&db)?;
        warnings.extend(settings_warn);
        let db = Arc::new(Mutex::new(db));

        let tasks = match (&tools.ffmpeg, &tools.ffprobe) {
            (Some(ff), Some(fp)) => {
                let sink2 = sink.clone();
                match TaskManager::new(
                    ff.clone(),
                    fp.clone(),
                    app_settings.convert_parallel as usize,
                    Some(dirs.logs.join("conversie.log")),
                    Arc::new(move |snap| sink2.emit(EV_CONVERT_JOB, json!(snap))),
                ) {
                    Ok(t) => Some(t),
                    Err(e) => {
                        warnings.push(format!("De converter is niet beschikbaar: {e}"));
                        None
                    }
                }
            }
            _ => {
                warnings.push("FFmpeg is niet gevonden: conversie, miniaturen en mediainformatie zijn niet beschikbaar.".into());
                None
            }
        };

        let player = PlayerHost::start(&tools, vlc_args, db.clone(), sink.clone(), &app_settings);
        if let Err(e) = &player {
            warnings.push(format!("De speler is niet beschikbaar: {e}"));
        }
        let state = AppState { version: version.to_string(), dirs, tools, sink, db, tasks, player, scan: Mutex::new(None), warnings };
        // een eerder ingestelde equalizer meteen weer toepassen
        let _ = crate::services::audio::restore_on_startup(&state);
        Ok(state)
    }

    pub fn lock_db(&self) -> Result<std::sync::MutexGuard<'_, Database>> {
        self.db.lock().map_err(|_| CoreError::Invalid("De database is vergrendeld door een eerdere fout".into()))
    }

    pub fn require_tasks(&self) -> Result<&TaskManager> {
        self.tasks.as_ref().ok_or_else(|| CoreError::Ffmpeg("FFmpeg is niet beschikbaar".into()))
    }

    pub fn require_player(&self) -> Result<&PlayerHost> {
        self.player.as_ref().map_err(|e| CoreError::Engine(e.clone()))
    }

    pub fn require_ffmpeg(&self) -> Result<&Path> {
        self.tools.ffmpeg.as_deref().ok_or_else(|| CoreError::Ffmpeg("FFmpeg is niet beschikbaar".into()))
    }

    pub fn require_ffprobe(&self) -> Result<&Path> {
        self.tools.ffprobe.as_deref().ok_or_else(|| CoreError::Ffmpeg("ffprobe is niet beschikbaar".into()))
    }

    pub fn settings(&self) -> Result<AppSettings> {
        Ok(settings::load(&*self.lock_db()?)?.0)
    }

    /// Netjes afsluiten: scan annuleren, conversies stoppen, positie opslaan, speler stoppen.
    pub fn shutdown(&self) {
        if let Ok(mut s) = self.scan.lock() {
            if let Some(run) = s.take() {
                run.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = run.handle.join();
            }
        }
        if let Some(t) = &self.tasks {
            t.shutdown();
        }
        if let Ok(p) = &self.player {
            p.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corrupt_database_is_set_aside_not_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");
        std::fs::write(&path, b"dit is geen sqlite-database, maar wel kostbare gebruikersdata").unwrap();
        let (db, warn) = open_database_recovering(&path).unwrap();
        assert!(warn.unwrap().contains("bewaard als"));
        db.integrity_check().unwrap();
        let aside: Vec<_> =
            std::fs::read_dir(dir.path()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains("beschadigd")).collect();
        assert_eq!(aside.len(), 1);
        assert_eq!(std::fs::read(aside[0].path()).unwrap(), b"dit is geen sqlite-database, maar wel kostbare gebruikersdata");
    }

    #[test]
    fn a_healthy_database_opens_without_warning() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");
        drop(Database::open(&path).unwrap());
        assert!(open_database_recovering(&path).unwrap().1.is_none());
    }

    #[test]
    fn backups_are_created_when_due_and_pruned_to_the_newest_few() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_in_memory().unwrap();
        assert!(rotate_backups(&db, dir.path(), 7, 3).unwrap().is_some()); // eerste keer
        assert!(rotate_backups(&db, dir.path(), 7, 3).unwrap().is_none()); // nog vers
        for i in 0..5 {
            std::fs::write(dir.path().join(format!("library-{}.db", 100 + i)), b"x").unwrap();
            // oude exemplaren
        }
        rotate_backups(&db, dir.path(), 7, 3).unwrap();
        let n = std::fs::read_dir(dir.path()).unwrap().flatten().count();
        assert_eq!(n, 3);
    }

    #[test]
    fn app_dirs_are_created() {
        let base = tempfile::tempdir().unwrap();
        let d = AppDirs::new(base.path()).unwrap();
        assert!(d.data.is_dir() && d.cache.is_dir() && d.logs.is_dir() && d.backups.is_dir());
    }
}
