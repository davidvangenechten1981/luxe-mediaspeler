//! Applicatielaag: alle functionaliteit achter de interface, testbaar zonder Tauri.

pub mod events;
pub mod player_host;
pub mod services;
pub mod state;
pub mod tools;

pub use events::{EventSink, NullSink};
pub use state::{AppDirs, AppState};
pub use tools::Tools;
