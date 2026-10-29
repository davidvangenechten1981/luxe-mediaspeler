//! Windows: een eigen videovenster achter het transparante Tauri-venster.
//!
//! LibVLC tekent in het videovenster. Het Tauri-venster (met de transparante WebView) is eigenaar-gekoppeld
//! en staat altijd direct erboven. Een synchronisatielus houdt positie, grootte, zichtbaarheid en
//! "altijd op voorgrond" gelijk. Bewezen op een Windows-runner; zie docs/adr/0001-architectuur.md.

#![cfg(windows)]

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn hwnd(h: isize) -> HWND {
    h as *mut c_void
}

pub fn create_video_window(x: i32, y: i32, w: i32, h: i32) -> Result<isize, String> {
    // SAFETY: standaard Win32-aanroepen met geldige, lokaal levende pointers.
    unsafe {
        let hinst = GetModuleHandleW(null());
        let class = wide("VlcUltimateVideo");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: GetStockObject(BLACK_BRUSH) as _,
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc); // 0 bij "bestaat al": geen fout voor ons
        let v = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class.as_ptr(),
            wide("VLC Ultimate video").as_ptr(),
            WS_POPUP | WS_VISIBLE,
            x,
            y,
            w,
            h,
            null_mut(),
            null_mut(),
            hinst,
            null(),
        );
        if v.is_null() {
            return Err("Het videovenster kon niet worden aangemaakt".into());
        }
        Ok(v as isize)
    }
}

/// Maakt het videovenster eigenaar van het UI-venster en zet het direct erachter.
pub fn attach_below(ui: isize, video: isize) {
    // SAFETY: beide vensters zijn van dit proces.
    unsafe {
        SetWindowLongPtrW(hwnd(ui), GWLP_HWNDPARENT, video);
        SetWindowPos(hwnd(video), hwnd(ui), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

#[derive(Default)]
struct SyncState {
    rect: (i32, i32, i32, i32),
    visible: bool,
    topmost: bool,
    tick: u32,
}

fn sync_once(video: isize, ui: isize, last: &mut SyncState) {
    // SAFETY: vensters van dit proces; alle aanroepen zijn thread-veilig (Windows stuurt berichten door).
    unsafe {
        let (ui_h, v_h) = (hwnd(ui), hwnd(video));
        let visible = IsWindowVisible(ui_h) != 0 && IsIconic(ui_h) == 0;
        if !visible {
            if last.visible {
                ShowWindow(v_h, SW_HIDE);
                last.visible = false;
            }
            return;
        }
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(ui_h, &mut r);
        let rect = (r.left, r.top, r.right - r.left, r.bottom - r.top);
        let topmost = GetWindowLongW(ui_h, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0;
        last.tick = last.tick.wrapping_add(1);
        // Bij wijziging meteen; verder elke ~0,5 s de z-volgorde herstellen.
        if !last.visible || rect != last.rect || topmost != last.topmost || last.tick % 30 == 0 {
            if topmost != last.topmost || !last.visible {
                SetWindowPos(
                    v_h,
                    if topmost { HWND_TOPMOST } else { HWND_NOTOPMOST },
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            SetWindowPos(v_h, ui_h, rect.0, rect.1, rect.2, rect.3, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            last.rect = rect;
            last.topmost = topmost;
            last.visible = true;
        }
    }
}

pub struct VideoSync {
    pub video: isize,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl VideoSync {
    pub fn start(ui: isize, video: isize) -> VideoSync {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let thread = std::thread::spawn(move || {
            let mut state = SyncState::default();
            while !stop2.load(Ordering::Relaxed) {
                sync_once(video, ui, &mut state);
                std::thread::sleep(Duration::from_millis(16));
            }
        });
        VideoSync { video, stop, thread: Some(thread) }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        // SAFETY: het videovenster is van ons en wordt één keer vernietigd.
        unsafe {
            DestroyWindow(hwnd(self.video));
        }
    }
}
