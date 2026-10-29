mod vlc;
#[cfg(windows)]
mod winvideo;

use std::path::PathBuf;
use tauri::Manager;

/// Spike-opties via de command line (alleen voor de Windows-CI-test).
struct SpikeArgs {
    mode: String,
    media: Option<String>,
    vlc_dir: Option<PathBuf>,
}

fn parse_args() -> SpikeArgs {
    let mut a = SpikeArgs { mode: "d2".into(), media: None, vlc_dir: None };
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("--mode=") {
            a.mode = v.into();
        } else if let Some(v) = arg.strip_prefix("--media=") {
            a.media = Some(v.into());
        } else if let Some(v) = arg.strip_prefix("--vlc-dir=") {
            a.vlc_dir = Some(v.into());
        }
    }
    a
}

/// De pagina meldt zichzelf zodra React draait: bewijst dat de interface echt geladen is.
#[tauri::command]
fn ui_ready(note: String) {
    eprintln!("ui_ready: {note}");
}

pub fn run() {
    let args = parse_args();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ui_ready])
        .setup(move |app| {
            #[cfg(windows)]
            if let (Some(media), Some(dir)) = (args.media.clone(), args.vlc_dir.clone()) {
                let win = app.get_webview_window("main").ok_or("geen hoofdvenster")?;
                let ui_hwnd = win.hwnd()?;
                let target: isize = if args.mode == "d1" {
                    ui_hwnd.0 as isize
                } else {
                    let pos = win.outer_position()?;
                    let size = win.inner_size()?;
                    let v = winvideo::create_video_window(pos.x, pos.y, size.width as i32, size.height as i32)?;
                    winvideo::set_owner(ui_hwnd.0 as _, v);
                    v
                };
                let vlc = vlc::Vlc::load(&dir, &["--quiet", "--no-video-title-show", "--no-osd"])?;
                eprintln!("libvlc versie: {}", vlc.version());
                let player = vlc.play_in_window(&media, target as *mut _)?;
                app.manage(std::sync::Mutex::new(Some((vlc, player))));
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_secs(8));
                    for w in winvideo::describe_process_windows() {
                        eprintln!("venster: {w}");
                    }
                });
            }
            #[cfg(not(windows))]
            let _ = (&args, app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("fout bij het starten van VLC Ultimate");
}
