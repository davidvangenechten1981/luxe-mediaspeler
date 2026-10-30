//! Houdt Windows wakker tijdens het afspelen (zie `vu_app::power` voor de regels).
//!
//! `SetThreadExecutionState` geldt voor de aanroepende thread; daarom heeft de bewaker een eigen,
//! vaste thread. Valt de app weg, dan eindigt de thread en wordt de wens vanzelf losgelaten.

use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use vu_app::power::{PowerNeed, PowerTracker};

pub struct PowerGuard {
    tx: Sender<PowerNeed>,
    tracker: Mutex<PowerTracker>,
}

impl PowerGuard {
    pub fn start() -> PowerGuard {
        let (tx, rx) = channel::<PowerNeed>();
        std::thread::Builder::new()
            .name("slaapstand-bewaker".into())
            .spawn(move || {
                while let Ok(need) = rx.recv() {
                    apply(need);
                }
                apply(PowerNeed::None);
            })
            .ok();
        PowerGuard { tx, tracker: Mutex::new(PowerTracker::default()) }
    }

    pub fn set(&self, need: PowerNeed) {
        let changed = self.tracker.lock().unwrap_or_else(|e| e.into_inner()).update(need);
        if let Some(n) = changed {
            let _ = self.tx.send(n);
            crate::logging::info(&format!("slaapstand: {n:?}"));
        }
    }
}

#[cfg(windows)]
fn apply(need: PowerNeed) {
    use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
    let flags = match need {
        PowerNeed::None => ES_CONTINUOUS,
        PowerNeed::System => ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
        PowerNeed::Display => ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED,
    };
    // SAFETY: eenvoudige Win32-aanroep zonder pointers.
    unsafe {
        SetThreadExecutionState(flags);
    }
}

#[cfg(not(windows))]
fn apply(_need: PowerNeed) {}
