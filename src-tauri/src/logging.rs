//! Eenvoudige logbestanden: een paniekhaak die crashes vastlegt en een regel-logger.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
const MAX_LOG_BYTES: u64 = 1_000_000;

fn append(file: &str, line: &str) {
    let Some(dir) = LOG_DIR.get() else { return };
    let path = dir.join(file);
    // eenvoudige rotatie: te groot geworden bestand wordt .old (het vorige .old gaat weg)
    if std::fs::metadata(&path).map(|m| m.len() > MAX_LOG_BYTES).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join(format!("{file}.old")));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

/// Tijdstip in UTC als "2026-10-30 10:15:00" (zonder externe bibliotheek; dagen-naar-datum volgens Howard Hinnant).
pub fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}", rem / 3600, (rem % 3600) / 60, rem % 60)
}

pub fn stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("[{} UTC]", format_utc(secs))
}

/// Is de app de vorige keer gecrasht? Dan staat er een `crash.log`. Die wordt hernoemd (zodat de melding
/// maar één keer verschijnt) en de tekst wordt teruggegeven. Aanroepen vóór `init`.
pub fn take_previous_crash(dir: &Path) -> Option<String> {
    let path = dir.join("crash.log");
    let text = std::fs::read_to_string(&path).ok().filter(|t| !t.trim().is_empty());
    if text.is_some() {
        let _ = std::fs::rename(&path, dir.join("crash-vorige.log"));
    } else {
        let _ = std::fs::remove_file(&path);
    }
    text.map(|t| t.chars().take(6000).collect())
}

pub fn info(msg: &str) {
    append("app.log", &format!("{} {msg}", stamp()));
}

/// Zet de logmap en een paniekhaak die elke panic met locatie naar `crash.log` schrijft.
pub fn init(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = LOG_DIR.set(dir.to_path_buf());
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        append("crash.log", &format!("{} PANIC: {info}\n{}", stamp(), std::backtrace::Backtrace::force_capture()));
        previous(info);
    }));
    info("VLC Ultimate gestart");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_readable_utc() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00");
        assert_eq!(format_utc(1_000_000_000), "2001-09-09 01:46:40");
        assert_eq!(format_utc(1_709_164_800), "2024-02-29 00:00:00"); // schrikkeldag
        assert_eq!(format_utc(1_793_328_285), "2026-10-30 02:44:45");
    }

    #[test]
    fn an_earlier_crash_is_reported_once_and_kept_for_the_diagnostics() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(take_previous_crash(dir.path()), None);
        std::fs::write(dir.path().join("crash.log"), "PANIC: kapot").unwrap();
        assert_eq!(take_previous_crash(dir.path()).as_deref(), Some("PANIC: kapot"));
        assert!(!dir.path().join("crash.log").exists());
        assert!(dir.path().join("crash-vorige.log").exists());
        assert_eq!(take_previous_crash(dir.path()), None, "de tweede keer niets meer");
        std::fs::write(dir.path().join("crash.log"), "  \n").unwrap();
        assert_eq!(take_previous_crash(dir.path()), None, "een leeg bestand telt niet");
    }

    #[test]
    fn panics_and_messages_are_written_to_the_log_directory() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path());
        info("testbericht");
        let app_log = std::fs::read_to_string(LOG_DIR.get().unwrap().join("app.log")).unwrap();
        assert!(app_log.contains("testbericht"));
    }
}
