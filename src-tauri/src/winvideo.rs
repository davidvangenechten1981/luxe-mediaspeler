//! Een eigen Win32-venster voor de video (variant D2).
//! De webinterface staat als eigendom-venster (owner) erboven, transparant.

#![cfg(windows)]

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
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
        let hwnd = CreateWindowExW(
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
        if hwnd.is_null() {
            return Err("Videovenster kon niet worden aangemaakt".into());
        }
        Ok(hwnd as isize)
    }
}

/// Zet `owner` als eigenaar van `ui`: het UI-venster blijft altijd boven het videovenster.
pub fn set_owner(ui: HWND, owner: isize) {
    // SAFETY: beide HWND's zijn geldige vensters van dit proces.
    unsafe {
        SetWindowLongPtrW(ui, GWLP_HWNDPARENT, owner);
    }
}

pub fn move_window(hwnd: isize, x: i32, y: i32, w: i32, h: i32) {
    // SAFETY: hwnd is een geldig venster van dit proces.
    unsafe {
        SetWindowPos(hwnd as *mut c_void, null_mut(), x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER);
    }
}

unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam as *mut Vec<String>);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() {
        let mut class = [0u16; 64];
        let n = GetClassNameW(hwnd, class.as_mut_ptr(), 64) as usize;
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut r);
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        out.push(format!(
            "{} zichtbaar={} rect=({},{},{},{}) exstyle={:#x} layered={}",
            String::from_utf16_lossy(&class[..n]),
            IsWindowVisible(hwnd) != 0,
            r.left,
            r.top,
            r.right,
            r.bottom,
            ex,
            ex & WS_EX_LAYERED != 0
        ));
    }
    1
}

/// Alle bovenste vensters van dit proces, van voor naar achter (z-volgorde).
pub fn describe_process_windows() -> Vec<String> {
    let mut v: Vec<String> = vec![];
    // SAFETY: de callback schrijft alleen in `v`, dat tijdens de aanroep leeft.
    unsafe {
        EnumWindows(Some(collect_window), &mut v as *mut _ as LPARAM);
    }
    v
}
