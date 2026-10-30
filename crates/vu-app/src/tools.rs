//! Zoekt de externe programma's en bibliotheken: FFmpeg, ffprobe en LibVLC.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Default, Serialize)]
pub struct Tools {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    /// Map met `libvlc.dll` en `plugins/`. `None` = systeembibliotheek (alleen buiten Windows).
    pub vlc_dir: Option<PathBuf>,
}

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn works(path: &Path) -> bool {
    Command::new(path).arg("-version").stdin(Stdio::null()).output().map(|o| o.status.success()).unwrap_or(false)
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let exe = exe_name(name);
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(&exe)).find(|p| p.is_file()))
}

impl Tools {
    /// Zoekvolgorde: omgevingsvariabele, meegeleverde map (`resource_dir`), daarna PATH.
    pub fn discover(resource_dir: Option<&Path>) -> Tools {
        let pick = |env: &str, rel: &str, name: &str| -> Option<PathBuf> {
            let mut candidates: Vec<PathBuf> = vec![];
            if let Some(v) = std::env::var_os(env) {
                candidates.push(PathBuf::from(v));
            }
            if let Some(r) = resource_dir {
                candidates.push(r.join(rel).join(exe_name(name)));
                candidates.push(r.join(exe_name(name)));
            }
            if let Some(p) = find_in_path(name) {
                candidates.push(p);
            }
            candidates.into_iter().find(|p| p.is_file() && works(p))
        };
        let vlc_dir = std::env::var_os("VU_VLC_DIR")
            .map(PathBuf::from)
            .into_iter()
            .chain(resource_dir.map(|r| r.join("vlc")))
            .find(|d| d.join(if cfg!(windows) { "libvlc.dll" } else { "libvlc.so.5" }).is_file());
        Tools { ffmpeg: pick("VU_FFMPEG", "ffmpeg", "ffmpeg"), ffprobe: pick("VU_FFPROBE", "ffmpeg", "ffprobe"), vlc_dir }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolVersions {
    pub ffmpeg: Option<String>,
    pub ffprobe: Option<String>,
}

/// Eerste regel van `-version`, voor het diagnosescherm.
pub fn version_line(path: &Path) -> Option<String> {
    let out = Command::new(path).arg("-version").stdin(Stdio::null()).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().next().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_ffmpeg_and_ffprobe_on_path_when_present() {
        let t = Tools::discover(None);
        if find_in_path("ffmpeg").is_some() {
            assert!(t.ffmpeg.is_some() && t.ffprobe.is_some());
            assert!(version_line(t.ffmpeg.as_ref().unwrap()).unwrap().starts_with("ffmpeg version"));
        }
    }

    #[test]
    fn resource_dir_without_binaries_finds_nothing_extra() {
        let dir = tempfile::tempdir().unwrap();
        let t = Tools::discover(Some(dir.path()));
        // VU_VLC_DIR mag de CI wel zetten; er mag alleen niets uit de lege resourcemap komen
        assert!(t.vlc_dir.as_deref().is_none_or(|d| !d.starts_with(dir.path())));
    }

    #[test]
    fn a_broken_binary_is_not_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join(exe_name("ffmpeg"));
        std::fs::write(&fake, b"geen programma").unwrap();
        assert!(!works(&fake));
    }
}
