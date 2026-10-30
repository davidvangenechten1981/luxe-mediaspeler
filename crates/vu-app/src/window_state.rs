//! Venstergrootte en -plek onthouden tussen sessies. De regels (wat is nog zichtbaar?) staan hier,
//! zodat ze zonder Tauri te testen zijn; het lezen en toepassen op het venster gebeurt in `src-tauri`.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MIN_WIDTH: u32 = 900;
pub const MIN_HEIGHT: u32 = 560;
/// Zoveel pixels van de titelbalk moeten zichtbaar blijven, anders pakken we het venster niet meer.
const VISIBLE_MARGIN: i32 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// Een scherm in fysieke pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monitor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn load(path: &Path) -> Option<WindowState> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(path: &Path, state: &WindowState) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(state).map_err(std::io::Error::other)?)?;
    std::fs::rename(tmp, path)
}

/// Past een opgeslagen toestand aan op de schermen die nu bestaan: de grootte past op het grootste scherm
/// en de titelbalk is zichtbaar. Geeft `None` als er geen schermen bekend zijn (dan laten we het venster ongemoeid).
pub fn fit(state: &WindowState, monitors: &[Monitor]) -> Option<WindowState> {
    let biggest = monitors.iter().max_by_key(|m| u64::from(m.width) * u64::from(m.height))?;
    let width = state.width.clamp(MIN_WIDTH.min(biggest.width), biggest.width);
    let height = state.height.clamp(MIN_HEIGHT.min(biggest.height), biggest.height);
    let mut out = WindowState { width, height, ..*state };

    let title_bar_visible = |m: &Monitor| {
        let right = out.x + out.width as i32;
        let overlap_x = right.min(m.x + m.width as i32) - out.x.max(m.x);
        out.y >= m.y - 10 && out.y < m.y + m.height as i32 - VISIBLE_MARGIN / 2 && overlap_x >= VISIBLE_MARGIN
    };
    if !monitors.iter().any(title_bar_visible) {
        // Het scherm waar het venster stond is weg of de plek ligt buiten beeld: centreer op het grootste scherm.
        out.x = biggest.x + (biggest.width as i32 - width as i32) / 2;
        out.y = biggest.y + (biggest.height as i32 - height as i32) / 2;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: Monitor = Monitor { x: 0, y: 0, width: 1920, height: 1080 };
    const SIDE: Monitor = Monitor { x: 1920, y: 0, width: 1280, height: 1024 };
    fn ws(x: i32, y: i32, w: u32, h: u32) -> WindowState {
        WindowState { x, y, width: w, height: h, maximized: false }
    }

    #[test]
    fn a_window_that_is_still_visible_stays_where_it_was() {
        assert_eq!(fit(&ws(300, 200, 1280, 760), &[MAIN]), Some(ws(300, 200, 1280, 760)));
        assert_eq!(fit(&ws(2100, 100, 800, 600), &[MAIN, SIDE]), Some(ws(2100, 100, 900, 600)));
        // te smal -> minimum
    }

    #[test]
    fn a_window_on_a_disconnected_monitor_is_centred_on_the_biggest_one() {
        let out = fit(&ws(2100, 100, 1000, 700), &[MAIN]).unwrap();
        assert_eq!((out.x, out.y), ((1920 - 1000) / 2, (1080 - 700) / 2));
    }

    #[test]
    fn a_window_hanging_off_the_screen_is_pulled_back() {
        let out = fit(&ws(-1500, 50, 1200, 700), &[MAIN]).unwrap();
        assert!(out.x >= 0, "{out:?}");
        let out = fit(&ws(300, 5000, 1200, 700), &[MAIN]).unwrap();
        assert!(out.y < 1080, "{out:?}");
    }

    #[test]
    fn a_window_bigger_than_the_screen_is_shrunk() {
        let out = fit(&ws(0, 0, 5000, 4000), &[MAIN]).unwrap();
        assert_eq!((out.width, out.height), (1920, 1080));
    }

    #[test]
    fn the_maximized_flag_is_kept_and_no_monitors_means_no_change() {
        let mut s = ws(10, 10, 1280, 760);
        s.maximized = true;
        assert!(fit(&s, &[MAIN]).unwrap().maximized);
        assert_eq!(fit(&s, &[]), None);
    }

    #[test]
    fn save_and_load_roundtrip_and_garbage_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("window.json");
        assert_eq!(load(&p), None);
        let s = ws(5, 6, 1000, 700);
        save(&p, &s).unwrap();
        assert_eq!(load(&p), Some(s));
        std::fs::write(&p, "geen json").unwrap();
        assert_eq!(load(&p), None);
    }
}
