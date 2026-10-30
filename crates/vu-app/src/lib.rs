//! Applicatielaag: alle functionaliteit achter de interface, testbaar zonder Tauri.

pub mod api;
pub mod events;
pub mod media_session;
pub mod player_host;
pub mod power;
pub mod services;
pub mod state;
pub mod tools;
pub mod window_state;

pub use events::{EventSink, NullSink};
pub use state::{AppDirs, AppState};
pub use tools::Tools;
