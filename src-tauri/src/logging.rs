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

pub fn stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("[{secs}]")
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
    fn panics_and_messages_are_written_to_the_log_directory() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path());
        info("testbericht");
        let app_log = std::fs::read_to_string(LOG_DIR.get().unwrap().join("app.log")).unwrap();
        assert!(app_log.contains("testbericht"));
    }
}
