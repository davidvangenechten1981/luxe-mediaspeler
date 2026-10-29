//! Brug tussen de backend-gebeurtenissen en de interface.

use serde_json::Value;
use tauri::{AppHandle, Emitter};
use vu_app::EventSink;

pub struct TauriSink(pub AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, event: &str, payload: Value) {
        // Een mislukte emit (bijv. tijdens afsluiten) mag nooit de backend laten crashen.
        let _ = self.0.emit(event, payload);
    }
}
