//! Opstart van de applicatie: plugins, staat, vensters en afsluiten.

use crate::commands::{self, Launch, StartupInfo};
use crate::launch;
use crate::logging;
use crate::sink::TauriSink;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, RunEvent, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};
use vu_app::services::player::{self, PlayerCommand};
use vu_app::window_state::{self, Monitor, WindowState};
use vu_app::{AppDirs, AppState, Tools};

#[cfg(windows)]
struct VideoSyncHandle(Mutex<Option<crate::winvideo::VideoSync>>);

/// Laatst bekende "gewone" (niet gemaximaliseerde) venstermaat, en waar die bewaard wordt.
struct WindowTracker {
    path: PathBuf,
    state: Mutex<Option<WindowState>>,
}

impl WindowTracker {
    /// Leest de huidige toestand van het venster. Minimaliseren en volledig scherm tellen niet mee.
    fn observe(&self, win: &WebviewWindow) {
        if win.is_minimized().unwrap_or(false) || win.is_fullscreen().unwrap_or(false) {
            return;
        }
        let maximized = win.is_maximized().unwrap_or(false);
        let mut cur = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if maximized {
            if let Some(c) = cur.as_mut() {
                c.maximized = true;
            }
            return;
        }
        if let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) {
            *cur = Some(WindowState { x: pos.x, y: pos.y, width: size.width, height: size.height, maximized: false });
        }
    }

    fn save(&self) {
        if let Some(s) = *self.state.lock().unwrap_or_else(|e| e.into_inner()) {
            if let Err(e) = window_state::save(&self.path, &s) {
                logging::info(&format!("venstergrootte opslaan mislukt: {e}"));
            }
        }
    }
}

fn restore_window(win: &WebviewWindow, path: &std::path::Path) {
    let Some(saved) = window_state::load(path) else {
        return;
    };
    let monitors: Vec<Monitor> = win
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| Monitor { x: m.position().x, y: m.position().y, width: m.size().width, height: m.size().height })
        .collect();
    if let Some(fit) = window_state::fit(&saved, &monitors) {
        let _ = win.set_size(PhysicalSize::new(fit.width, fit.height));
        let _ = win.set_position(PhysicalPosition::new(fit.x, fit.y));
        if fit.maximized {
            let _ = win.maximize();
        }
        logging::info(&format!("venster hersteld: {fit:?}"));
    }
}

fn media_key_command(code: Code) -> Option<PlayerCommand> {
    match code {
        Code::MediaPlayPause => Some(PlayerCommand::TogglePause),
        Code::MediaTrackNext => Some(PlayerCommand::Next),
        Code::MediaTrackPrevious => Some(PlayerCommand::Previous),
        Code::MediaStop => Some(PlayerCommand::Stop),
        _ => None,
    }
}

pub fn run() {
    let launch_request = launch::parse(&std::env::args().skip(1).collect::<Vec<_>>());

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Een tweede start (bijv. dubbelklik in Verkenner) stuurt zijn bestanden naar deze instantie.
            let req = launch::parse(&argv[1.min(argv.len())..]);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
            let _ = app.emit("app://open-files", req);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if let (Some(cmd), Some(state)) = (media_key_command(shortcut.key), app.try_state::<Arc<AppState>>()) {
                        let _ = player::command(&state, cmd);
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let Some(tracker) = window.try_state::<WindowTracker>() else {
                return;
            };
            match event {
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    if let Some(w) = window.app_handle().get_webview_window("main") {
                        tracker.observe(&w);
                    }
                }
                WindowEvent::CloseRequested { .. } => {
                    if let Some(w) = window.app_handle().get_webview_window("main") {
                        tracker.observe(&w);
                    }
                    tracker.save();
                }
                _ => {}
            }
        })
        .manage(Launch(Mutex::new(launch_request)))
        .invoke_handler(tauri::generate_handler![
            commands::api,
            commands::take_launch_files,
            commands::app_paths,
            commands::shell_status,
            commands::shell_set,
            commands::open_default_apps,
            commands::startup_info,
            commands::restart_app
        ])
        .setup(|app| {
            let base = std::env::var_os("VU_DATA_DIR").map(PathBuf::from).map(Ok).unwrap_or_else(|| app.path().app_data_dir())?;
            let dirs = AppDirs::new(&base)?;
            let previous_crash = logging::take_previous_crash(&dirs.logs);
            logging::init(&dirs.logs);
            if previous_crash.is_some() {
                logging::info("de vorige sessie is onverwacht gestopt (zie crash-vorige.log)");
            }
            app.manage(StartupInfo(Mutex::new(previous_crash)));
            let tools = Tools::discover(app.path().resource_dir().ok().as_deref());
            logging::info(&format!("tools: ffmpeg={:?} ffprobe={:?} vlc={:?}", tools.ffmpeg, tools.ffprobe, tools.vlc_dir));
            let sink = Arc::new(TauriSink::new(app.handle().clone()));
            let state = Arc::new(AppState::init(env!("CARGO_PKG_VERSION"), dirs, tools, sink, vec![])?);
            for w in &state.warnings {
                logging::info(&format!("waarschuwing: {w}"));
            }

            // Vensterplek en -grootte van de vorige keer, nog vóór het videovenster wordt gemaakt
            if let Some(win) = app.get_webview_window("main") {
                let path = state.dirs.data.join("window.json");
                restore_window(&win, &path);
                let tracker = WindowTracker { path, state: Mutex::new(None) };
                tracker.observe(&win);
                app.manage(tracker);
            }

            #[cfg(windows)]
            {
                let win = app.get_webview_window("main").ok_or("geen hoofdvenster")?;
                let ui = win.hwnd()?.0 as isize;
                let pos = win.outer_position()?;
                let size = win.outer_size()?;
                match crate::winvideo::create_video_window(pos.x, pos.y, size.width as i32, size.height as i32) {
                    Ok(video) => {
                        crate::winvideo::attach_below(ui, video);
                        if let Ok(player) = &state.player {
                            if let Err(e) = player.with(|s| s.engine_mut().set_window(video as u64)).and_then(|r| r) {
                                logging::info(&format!("videovenster koppelen mislukt: {e}"));
                            }
                        }
                        app.manage(VideoSyncHandle(Mutex::new(Some(crate::winvideo::VideoSync::start(ui, video)))));
                    }
                    Err(e) => logging::info(&format!("videovenster: {e}")),
                }
            }

            // Mediatoetsen (als een andere app ze al gebruikt, slaan we ze stil over)
            for code in [Code::MediaPlayPause, Code::MediaTrackNext, Code::MediaTrackPrevious, Code::MediaStop] {
                if let Err(e) = app.global_shortcut().register(Shortcut::new(None, code)) {
                    logging::info(&format!("mediatoets {code:?} niet beschikbaar: {e}"));
                }
            }

            app.manage(state);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("fout bij het starten van VLC Ultimate");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            #[cfg(windows)]
            if let Some(h) = handle.try_state::<VideoSyncHandle>() {
                if let Some(mut v) = h.0.lock().ok().and_then(|mut g| g.take()) {
                    v.stop();
                }
            }
            if let Some(t) = handle.try_state::<WindowTracker>() {
                t.save();
            }
            if let Some(state) = handle.try_state::<Arc<AppState>>() {
                state.shutdown();
            }
        }
    });
}
