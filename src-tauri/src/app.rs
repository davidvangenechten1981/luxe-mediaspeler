//! Opstart van de applicatie: plugins, staat, vensters en afsluiten.

use crate::commands::{self, Launch};
use crate::launch;
use crate::logging;
use crate::sink::TauriSink;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, RunEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};
use vu_app::services::player::{self, PlayerCommand};
use vu_app::{AppDirs, AppState, Tools};

#[cfg(windows)]
struct VideoSyncHandle(Mutex<Option<crate::winvideo::VideoSync>>);

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
        .manage(Launch(Mutex::new(launch_request)))
        .invoke_handler(tauri::generate_handler![
            commands::api,
            commands::take_launch_files,
            commands::app_paths,
            commands::shell_status,
            commands::shell_set,
            commands::open_default_apps
        ])
        .setup(|app| {
            let base = std::env::var_os("VU_DATA_DIR").map(PathBuf::from).map(Ok).unwrap_or_else(|| app.path().app_data_dir())?;
            let dirs = AppDirs::new(&base)?;
            logging::init(&dirs.logs);
            let tools = Tools::discover(app.path().resource_dir().ok().as_deref());
            logging::info(&format!("tools: ffmpeg={:?} ffprobe={:?} vlc={:?}", tools.ffmpeg, tools.ffprobe, tools.vlc_dir));
            let sink = Arc::new(TauriSink(app.handle().clone()));
            let state = Arc::new(AppState::init(env!("CARGO_PKG_VERSION"), dirs, tools, sink, vec![])?);
            for w in &state.warnings {
                logging::info(&format!("waarschuwing: {w}"));
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
            if let Some(state) = handle.try_state::<Arc<AppState>>() {
                state.shutdown();
            }
        }
    });
}
