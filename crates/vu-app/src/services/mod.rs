//! Alle bewerkingen die de interface kan aanroepen. Elke dienst valideert zijn invoer.

pub mod audio;
pub mod converter;
pub mod cover;
pub mod history;
pub mod library;
pub mod lyrics;
pub mod player;
pub mod playlists;
pub mod series;
pub mod settings;
pub mod video;

#[cfg(test)]
pub mod testutil {
    use crate::events::testing::RecordingSink;
    use crate::state::{AppDirs, AppState};
    use crate::tools::Tools;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    pub fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../vu-core/tests/fixtures").join(name)
    }

    pub struct TestApp {
        pub state: AppState,
        pub sink: Arc<RecordingSink>,
        pub dir: tempfile::TempDir,
    }

    /// Volledige app met echte database, FFmpeg en (als beschikbaar) LibVLC met dummy-uitvoer.
    pub fn app() -> TestApp {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(RecordingSink::default());
        let dirs = AppDirs::new(dir.path()).unwrap();
        let mut vlc_args = vec!["--aout=dummy".to_string(), "--vout=dummy".to_string()];
        #[cfg(not(windows))]
        vlc_args.push("--no-xlib".into());
        let state = AppState::init("0.0.0-test", dirs, Tools::discover(None), sink.clone(), vlc_args).unwrap();
        TestApp { state, sink, dir }
    }

    /// Map met kopieën van de testbestanden.
    pub fn media_dir(app: &TestApp) -> PathBuf {
        let d = app.dir.path().join("media");
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::copy(fixture("sample.mkv"), d.join("film.mkv")).unwrap();
        std::fs::copy(fixture("sample.mp3"), d.join("sub/lied.mp3")).unwrap();
        d
    }

    pub fn wait_until(secs: u64, mut f: impl FnMut() -> bool) -> bool {
        let t = std::time::Instant::now();
        while t.elapsed() < std::time::Duration::from_secs(secs) {
            if f() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        false
    }
}
