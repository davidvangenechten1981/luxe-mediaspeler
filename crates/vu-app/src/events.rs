//! Gebeurtenissen van de backend naar de interface.

use serde_json::Value;

pub const EV_PLAYER_STATE: &str = "player://state";
pub const EV_PLAYER_EVENT: &str = "player://event";
pub const EV_SCAN_PROGRESS: &str = "library://scan-progress";
pub const EV_SCAN_DONE: &str = "library://scan-done";
pub const EV_CONVERT_JOB: &str = "convert://job";

/// De interface (Tauri) implementeert dit; tests gebruiken een opnemende variant.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: &str, payload: Value);
}

pub struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: &str, _payload: Value) {}
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct RecordingSink(pub Mutex<Vec<(String, Value)>>);

    impl EventSink for RecordingSink {
        fn emit(&self, event: &str, payload: Value) {
            self.0.lock().unwrap().push((event.to_string(), payload));
        }
    }

    impl RecordingSink {
        pub fn named(&self, name: &str) -> Vec<Value> {
            self.0.lock().unwrap().iter().filter(|(n, _)| n == name).map(|(_, v)| v.clone()).collect()
        }
    }
}
