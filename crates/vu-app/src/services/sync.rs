//! Kijkstatus delen met een andere pc via een gedeelde map (zie `vu_core::sync`).

use crate::state::AppState;
use vu_core::error::{CoreError, Result};
use vu_core::sync::{sync_folder, SyncReport};
use vu_core::validate::validate_folder_path;

pub fn run(st: &AppState) -> Result<SyncReport> {
    let folder = crate::services::settings::get(st)?
        .0
        .sync_folder
        .ok_or_else(|| CoreError::Invalid("Kies eerst een map voor synchronisatie".into()))?;
    let folder = validate_folder_path(&folder)?;
    let device = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "pc".into());
    let db = st.lock_db()?;
    sync_folder(&db, &folder, &device)
}
