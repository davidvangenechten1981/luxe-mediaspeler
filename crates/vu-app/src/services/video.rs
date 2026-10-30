//! Video Studio: mediainfo, miniaturen en niet-destructieve beeldinstellingen.

use crate::state::AppState;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use vu_core::error::{CoreError, Result};
use vu_core::probe::{probe_file, MediaInfo};
use vu_core::validate::validate_media_path;
use vu_vlc::{Adjust, Rotation};

pub fn media_info(st: &AppState, path: &str) -> Result<MediaInfo> {
    let p = validate_media_path(path)?;
    probe_file(st.require_ffprobe()?, &p)
}

pub(crate) fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01b3))
}

pub(crate) fn run_with_timeout(mut cmd: Command, limit: Duration) -> Result<()> {
    let mut child =
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| CoreError::Ffmpeg(e.to_string()))?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            let mut msg = String::new();
            if let Some(mut e) = child.stderr.take() {
                let _ = std::io::Read::read_to_string(&mut e, &mut msg);
            }
            return Err(CoreError::Ffmpeg(msg.trim().to_string()));
        }
        if start.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CoreError::Ffmpeg("Het maken van de miniatuur duurde te lang".into()));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Miniatuur op `at_ms`, gecachet. Geeft het pad naar een JPEG terug.
pub fn thumbnail(st: &AppState, path: &str, at_ms: u64, width: u32) -> Result<PathBuf> {
    let p = validate_media_path(path)?;
    let ffmpeg = st.require_ffmpeg()?;
    let width = width.clamp(64, 1280);
    let meta = std::fs::metadata(&p)?;
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let key = fnv1a(format!("{}|{}|{}|{}|{}", p.display(), meta.len(), mtime, at_ms, width).as_bytes());
    let dir = st.dirs.thumbnails();
    std::fs::create_dir_all(&dir)?;
    let out = dir.join(format!("{key:016x}.jpg"));
    if out.is_file() && std::fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(out);
    }
    let tmp = out.with_extension("tmp.jpg");
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-ss", &format!("{:.3}", at_ms as f64 / 1000.0), "-i"])
        .arg(&p)
        .args(["-frames:v", "1", "-vf", &format!("scale={width}:-2"), "-q:v", "4", "-y"])
        .arg(&tmp);
    if let Err(e) = run_with_timeout(cmd, Duration::from_secs(20)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if std::fs::metadata(&tmp).map(|m| m.len() == 0).unwrap_or(true) {
        let _ = std::fs::remove_file(&tmp);
        return Err(CoreError::Ffmpeg("Op dit tijdstip is geen beeld beschikbaar".into()));
    }
    std::fs::rename(&tmp, &out)?;
    Ok(out)
}

/// `count` miniaturen gelijkmatig tussen 5% en 95% van de duur, voor de tijdlijnpreview.
pub fn thumbnail_strip(st: &AppState, path: &str, count: u32, width: u32) -> Result<Vec<(u64, PathBuf)>> {
    let info = media_info(st, path)?;
    if !info.has_video() {
        return Err(CoreError::Invalid("Dit bestand bevat geen video".into()));
    }
    let count = count.clamp(1, 60) as u64;
    (0..count)
        .map(|i| {
            let at = info.duration_ms * (5 + 90 * i / count.max(2).saturating_sub(1).max(1)) / 100;
            thumbnail(st, path, at.min(info.duration_ms.saturating_sub(100)), width).map(|p| (at, p))
        })
        .collect()
}

/// Wist de miniatuurcache; geeft het aantal verwijderde bestanden terug.
pub fn clear_thumbnail_cache(st: &AppState) -> Result<usize> {
    let dir = st.dirs.thumbnails();
    let mut n = 0;
    if dir.is_dir() {
        for e in std::fs::read_dir(dir)?.flatten() {
            if e.path().extension().is_some_and(|x| x == "jpg") && std::fs::remove_file(e.path()).is_ok() {
                n += 1;
            }
        }
    }
    Ok(n)
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RotationChoice {
    None,
    Rotate90,
    Rotate180,
    Rotate270,
    FlipHorizontal,
    FlipVertical,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustChoice {
    Contrast,
    Brightness,
    Hue,
    Saturation,
    Gamma,
}

pub fn set_aspect_ratio(st: &AppState, ratio: Option<&str>) -> Result<()> {
    st.require_player()?.with(|s| s.engine_mut().set_aspect_ratio(ratio))?
}
pub fn set_crop(st: &AppState, ratio: Option<&str>) -> Result<()> {
    st.require_player()?.with(|s| s.engine_mut().set_crop(ratio))?
}
pub fn set_adjust(st: &AppState, which: AdjustChoice, value: f32) -> Result<()> {
    let a = match which {
        AdjustChoice::Contrast => Adjust::Contrast,
        AdjustChoice::Brightness => Adjust::Brightness,
        AdjustChoice::Hue => Adjust::Hue,
        AdjustChoice::Saturation => Adjust::Saturation,
        AdjustChoice::Gamma => Adjust::Gamma,
    };
    st.require_player()?.with(|s| s.engine_mut().set_adjust(a, value))?
}
pub fn set_deinterlace(st: &AppState, mode: &str) -> Result<()> {
    st.require_player()?.with(|s| s.engine_mut().set_deinterlace(mode))?
}
pub fn set_rotation(st: &AppState, r: RotationChoice) -> Result<()> {
    let rot = match r {
        RotationChoice::None => Rotation::None,
        RotationChoice::Rotate90 => Rotation::Rotate90,
        RotationChoice::Rotate180 => Rotation::Rotate180,
        RotationChoice::Rotate270 => Rotation::Rotate270,
        RotationChoice::FlipHorizontal => Rotation::FlipHorizontal,
        RotationChoice::FlipVertical => Rotation::FlipVertical,
    };
    st.require_player()?.with(|s| s.engine_mut().set_rotation(rot))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::*;

    #[test]
    fn media_info_reports_streams_and_chapters_for_the_ui() {
        let app = app();
        if app.state.tools.ffprobe.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let info = media_info(&app.state, dir.join("film.mkv").to_str().unwrap()).unwrap();
        assert_eq!(info.first_video().unwrap().height, Some(180));
        assert_eq!(info.chapters.len(), 2);
        assert!(media_info(&app.state, "/tmp/../etc/passwd").is_err());
    }

    #[test]
    fn thumbnails_are_real_images_cached_and_cleanable() {
        let app = app();
        if app.state.tools.ffmpeg.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let film = dir.join("film.mkv").to_string_lossy().into_owned();
        let a = thumbnail(&app.state, &film, 1_000, 160).unwrap();
        let bytes = std::fs::read(&a).unwrap();
        assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF], "geen JPEG");
        let modified = std::fs::metadata(&a).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let b = thumbnail(&app.state, &film, 1_000, 160).unwrap();
        assert_eq!(a, b);
        assert_eq!(std::fs::metadata(&b).unwrap().modified().unwrap(), modified); // uit de cache
        let other = thumbnail(&app.state, &film, 2_000, 160).unwrap();
        assert_ne!(a, other);
        assert_eq!(clear_thumbnail_cache(&app.state).unwrap(), 2);
        assert!(!a.exists());
    }

    #[test]
    fn thumbnail_errors_are_clean() {
        let app = app();
        if app.state.tools.ffmpeg.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let kapot = dir.join("kapot.mkv");
        std::fs::write(&kapot, b"geen video").unwrap();
        assert!(thumbnail(&app.state, kapot.to_str().unwrap(), 0, 160).is_err());
        assert!(thumbnail(&app.state, "/bestaat/niet.mkv", 0, 160).is_err());
        assert!(std::fs::read_dir(app.state.dirs.thumbnails())
            .map(|d| d.flatten().all(|e| !e.file_name().to_string_lossy().contains("tmp")))
            .unwrap_or(true));
        // audio heeft geen strip
        assert!(thumbnail_strip(&app.state, dir.join("sub/lied.mp3").to_str().unwrap(), 4, 120).is_err());
    }

    #[test]
    fn thumbnail_strip_covers_the_video_evenly() {
        let app = app();
        if app.state.tools.ffmpeg.is_none() {
            return;
        }
        let dir = media_dir(&app);
        let strip = thumbnail_strip(&app.state, dir.join("film.mkv").to_str().unwrap(), 4, 120).unwrap();
        assert_eq!(strip.len(), 4);
        let times: Vec<u64> = strip.iter().map(|(t, _)| *t).collect();
        assert!(times.windows(2).all(|w| w[0] < w[1]), "{times:?}");
        assert!(strip.iter().all(|(_, p)| p.is_file()));
    }

    #[test]
    fn picture_settings_reach_the_player_and_bad_values_are_refused() {
        let app = app();
        if app.state.player.is_err() {
            return;
        }
        assert!(set_aspect_ratio(&app.state, Some("21:9")).is_ok());
        assert!(set_aspect_ratio(&app.state, Some("kaas")).is_err());
        assert!(set_crop(&app.state, None).is_ok());
        assert!(set_adjust(&app.state, AdjustChoice::Saturation, 1.2).is_ok());
        assert!(set_adjust(&app.state, AdjustChoice::Saturation, 9.0).is_err());
        assert!(set_deinterlace(&app.state, "yadif").is_ok() && set_deinterlace(&app.state, "x; y").is_err());
        assert!(set_rotation(&app.state, RotationChoice::None).is_ok());
    }
}
