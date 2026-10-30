//! Hulpprogramma's (ffmpeg, ffprobe) starten zonder dat Windows er een zwart consolevenster voor opent.

use std::ffi::OsStr;
use std::process::Command;

/// Windows-vlag: start het proces zonder consolevenster.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Maakt een `Command` dat op Windows geen consolevenster toont. Op andere systemen is het een gewone `Command`.
pub fn quiet<S: AsRef<OsStr>>(program: S) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Windows-vlag: lagere prioriteit, zodat de pc bruikbaar blijft tijdens een zware conversie.
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

/// Zoals `quiet`, maar met lagere prioriteit voor langdurig werk op de achtergrond (conversies).
pub fn background<S: AsRef<OsStr>>(program: S) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quiet_command_still_runs_and_returns_output() {
        #[cfg(windows)]
        let out = quiet("cmd").args(["/C", "echo", "hallo"]).output().unwrap();
        #[cfg(not(windows))]
        let out = quiet("echo").arg("hallo").output().unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("hallo"));
    }

    #[test]
    fn a_background_command_still_runs_and_returns_output() {
        #[cfg(windows)]
        let out = background("cmd").args(["/C", "echo", "hallo"]).output().unwrap();
        #[cfg(not(windows))]
        let out = background("echo").arg("hallo").output().unwrap();
        assert!(out.status.success());
    }

    /// Elk programma dat de app zelf start moet via `quiet`; een gewone `Command::new` laat op Windows kort een zwart venster zien.
    #[test]
    fn production_code_never_spawns_a_console_window() {
        let files = [
            ("vu-app/services/video.rs", include_str!("../../vu-app/src/services/video.rs")),
            ("vu-app/services/cover.rs", include_str!("../../vu-app/src/services/cover.rs")),
            ("vu-app/tools.rs", include_str!("../../vu-app/src/tools.rs")),
            ("vu-core/convert/args.rs", include_str!("convert/args.rs")),
            ("vu-core/convert/runner.rs", include_str!("convert/runner.rs")),
            ("vu-core/probe.rs", include_str!("probe.rs")),
            ("vu-core/scanner.rs", include_str!("scanner.rs")),
        ];
        for (name, text) in files {
            let production = text.split("#[cfg(test)]").next().unwrap();
            assert!(!production.contains("Command::new("), "{name} start een programma zonder `quiet`");
        }
        for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src-tauri/src")).unwrap().flatten() {
            if entry.path().extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(entry.path()).unwrap();
                let production = text.split("#[cfg(test)]").next().unwrap();
                assert!(!production.contains("Command::new("), "{:?} start een programma zonder `quiet`", entry.path());
            }
        }
    }
}
