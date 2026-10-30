//! Brug tussen de backend-gebeurtenissen en de interface.

use serde_json::Value;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use vu_app::EventSink;

pub struct TauriSink {
    handle: AppHandle,
    last_state: Mutex<String>,
}

impl TauriSink {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle, last_state: Mutex::new(String::new()) }
    }

    /// Statuswijzigingen van de speler komen in het logbestand (niet elke tik, alleen bij verandering).
    fn log(&self, event: &str, payload: &Value) {
        match event {
            "player://state" => {
                let state = payload.pointer("/status/state").and_then(Value::as_str).unwrap_or("?");
                let mut last = self.last_state.lock().unwrap_or_else(|e| e.into_inner());
                if *last != state {
                    *last = state.to_string();
                    crate::logging::info(&format!("speler: {state} ({})", payload.get("location").and_then(Value::as_str).unwrap_or("-")));
                }
            }
            "player://event" => crate::logging::info(&format!("speler-gebeurtenis: {payload}")),
            _ => {}
        }
    }
}

impl EventSink for TauriSink {
    fn emit(&self, event: &str, payload: Value) {
        self.log(event, &payload);
        // Een mislukte emit (bijv. tijdens afsluiten) mag nooit de backend laten crashen.
        let _ = self.handle.emit(event, payload);
    }
}
