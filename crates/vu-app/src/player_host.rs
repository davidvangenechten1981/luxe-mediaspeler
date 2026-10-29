//! Bezit de speler (sessie + LibVLC-engine) en draait de klok die elke 250 ms de sessie bijwerkt.

use crate::events::{EventSink, EV_PLAYER_EVENT, EV_PLAYER_STATE};
use crate::tools::Tools;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use vu_core::db::Database;
use vu_core::player::session::{Session, SessionView};
use vu_core::settings::{AppSettings, HwDecoding};
use vu_vlc::{VlcConfig, VlcEngine};

pub struct PlayerHost {
    session: Arc<Mutex<Session<VlcEngine>>>,
    stop: Arc<AtomicBool>,
    ticker: Mutex<Option<JoinHandle<()>>>,
}

fn hw_mode(h: HwDecoding) -> &'static str {
    match h {
        HwDecoding::Auto => "any",
        HwDecoding::None => "none",
        HwDecoding::D3d11va => "d3d11va",
        HwDecoding::Dxva2 => "dxva2",
    }
}

fn seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
}

impl PlayerHost {
    pub fn start(
        tools: &Tools,
        vlc_args: Vec<String>,
        db: Arc<Mutex<Database>>,
        sink: Arc<dyn EventSink>,
        settings: &AppSettings,
    ) -> Result<PlayerHost, String> {
        let engine = VlcEngine::new(&VlcConfig { library_dir: tools.vlc_dir.clone(), extra_args: vlc_args }).map_err(|e| e.to_string())?;
        let mut session = Session::new(engine, Box::new(db), seed());
        apply_settings(&mut session, settings);
        let session = Arc::new(Mutex::new(session));
        let stop = Arc::new(AtomicBool::new(false));

        let (s2, stop2) = (session.clone(), stop.clone());
        let ticker = std::thread::spawn(move || {
            let started = Instant::now();
            let mut last: Option<SessionView> = None;
            while !stop2.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(250));
                let (view, events) = {
                    let Ok(mut s) = s2.lock() else { break };
                    s.tick(started.elapsed().as_millis() as u64);
                    (s.view(), s.take_events())
                };
                for e in events {
                    sink.emit(EV_PLAYER_EVENT, json!(e));
                }
                if last.as_ref() != Some(&view) {
                    sink.emit(EV_PLAYER_STATE, json!(view));
                    last = Some(view);
                }
            }
        });
        Ok(PlayerHost { session, stop, ticker: Mutex::new(Some(ticker)) })
    }

    /// Voert een bewerking uit op de sessie.
    pub fn with<R>(&self, f: impl FnOnce(&mut Session<VlcEngine>) -> R) -> Result<R, vu_core::error::CoreError> {
        let mut s =
            self.session.lock().map_err(|_| vu_core::error::CoreError::Engine("De speler is vergrendeld door een eerdere fout".into()))?;
        Ok(f(&mut s))
    }

    pub fn view(&self) -> Result<SessionView, vu_core::error::CoreError> {
        self.with(|s| s.view())
    }

    pub fn apply_settings(&self, settings: &AppSettings) -> Result<(), vu_core::error::CoreError> {
        self.with(|s| apply_settings(s, settings))
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.ticker.lock().ok().and_then(|mut t| t.take()) {
            let _ = t.join();
        }
        if let Ok(mut s) = self.session.lock() {
            s.shutdown();
        }
    }
}

impl Drop for PlayerHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn apply_settings(s: &mut Session<VlcEngine>, a: &AppSettings) {
    s.resume_enabled = a.resume_playback;
    s.network_caching_ms = Some(a.network_caching_ms);
    s.preferred_audio = a.preferred_audio_languages.clone();
    s.preferred_subtitles = a.preferred_subtitle_languages.clone();
    let _ = s.engine_mut().set_hw_decoding(hw_mode(a.hw_decoding));
    let _ = s.engine_mut().set_normalize(a.normalize_realtime);
}
