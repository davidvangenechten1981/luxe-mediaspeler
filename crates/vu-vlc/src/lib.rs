//! LibVLC (3.x) als afspeelengine.
//!
//! De bibliotheek wordt tijdens het draaien geladen. Zo kan de app een duidelijke
//! fout tonen als de DLL ontbreekt, en blijft de LGPL-scheiding tussen onze code
//! en LibVLC helder (alleen dynamisch gekoppeld).

mod engine;
mod ffi;

pub use engine::{Adjust, Rotation, Visualization, VlcConfig, VlcEngine};
