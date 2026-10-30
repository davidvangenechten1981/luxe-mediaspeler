//! De Tauri-opdrachten. Alles loopt via het getypeerde `api`-verzoek; de overige opdrachten
//! zijn klein en kennen elk één vaste taak.

use crate::launch::LaunchRequest;
use crate::shell_integration::{self, Identity, IntegrationStatus};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use vu_app::AppState;

pub struct Launch(pub Mutex<LaunchRequest>);

/// Wat de interface bij het opstarten moet weten (nu: of de vorige sessie is gecrasht).
pub struct StartupInfo(pub Mutex<Option<String>>);

/// Geeft eenmalig de tekst van een eerdere crash terug (of niets).
#[tauri::command]
pub fn startup_info(info: State<'_, StartupInfo>) -> Value {
    let crash = info.0.lock().unwrap_or_else(|e| e.into_inner()).take();
    json!({ "previous_crash": crash })
}

/// Start de app opnieuw op (bijvoorbeeld voor instellingen die pas na een herstart gelden).
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

/// Het ene toegangspunt van de interface tot de backend (zie vu-app/src/api.rs).
#[tauri::command(async)]
pub fn api(state: State<'_, Arc<AppState>>, request: Value) -> Result<Value, String> {
    let name = request.get("type").and_then(Value::as_str).unwrap_or("?").to_string();
    let result = vu_app::api::handle_json(&state, request);
    // Alleen mislukte aanroepen worden gelogd (naam en foutmelding), zodat problemen te herleiden zijn
    if let Err(e) = &result {
        crate::logging::info(&format!("api {name} mislukt: {e}"));
    }
    result
}

/// Bestanden die bij het starten zijn meegegeven; worden één keer uitgeleverd.
#[tauri::command]
pub fn take_launch_files(launch: State<'_, Launch>) -> LaunchRequest {
    let req = std::mem::take(&mut *launch.0.lock().unwrap_or_else(|e| e.into_inner()));
    crate::logging::info(&format!("startbestanden opgehaald door de interface: {}", req.files.len()));
    req
}

#[tauri::command]
pub fn app_paths(app: AppHandle, state: State<'_, Arc<AppState>>) -> Value {
    let p = app.path();
    let s = |r: tauri::Result<std::path::PathBuf>| r.ok().map(|p| p.to_string_lossy().into_owned());
    json!({
        "data": state.dirs.data,
        "logs": state.dirs.logs,
        "backups": state.dirs.backups,
        "thumbnails": state.dirs.thumbnails(),
        "pictures": s(p.picture_dir()),
        "videos": s(p.video_dir()),
        "music": s(p.audio_dir()),
        "documents": s(p.document_dir()),
        "downloads": s(p.download_dir()),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellKind {
    OpenWith,
    ContextMenu,
}

#[tauri::command]
pub fn shell_status() -> IntegrationStatus {
    shell_integration::status(&Identity::production())
}

/// Registreert of verwijdert de Windows-integratie. Alleen aanroepen na expliciete bevestiging door de gebruiker.
#[tauri::command]
pub fn shell_set(kind: ShellKind, enable: bool) -> Result<IntegrationStatus, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let id = Identity::production();
    match kind {
        ShellKind::OpenWith => shell_integration::set_open_with(&id, &exe, enable),
        ShellKind::ContextMenu => shell_integration::set_context_menu(&id, &exe, enable),
    }
    .map_err(|e| format!("De Windows-integratie kon niet worden aangepast: {e}"))?;
    Ok(shell_integration::status(&id))
}

/// Opent de Windows-instellingen voor standaard-apps; de gebruiker kiest daar zelf.
#[tauri::command]
pub fn open_default_apps(app: AppHandle) -> Result<(), String> {
    app.opener().open_url("ms-settings:defaultapps", None::<&str>).map_err(|e| e.to_string())
}
