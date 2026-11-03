//! Opnemen van een livestream met FFmpeg: argumenten (als lijst, nooit als shell-tekst) en bestandsnamen.

use crate::validate::sanitize_file_name;
use std::path::{Path, PathBuf};

fn no_control(s: &str) -> bool {
    !s.chars().any(char::is_control)
}

/// FFmpeg-argumenten voor een opname zonder hercoderen (`-c copy`) naar een MPEG-TS-bestand.
/// Voor http(s)-bronnen wordt herverbinden aangezet. Headers met stuurtekens worden genegeerd.
pub fn build_args(url: &str, user_agent: Option<&str>, referrer: Option<&str>, out: &Path, duration_secs: u64) -> Vec<String> {
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin"].iter().map(|s| s.to_string()).collect();
    if let Some(ua) = user_agent.filter(|u| !u.trim().is_empty() && no_control(u)) {
        a.extend(["-user_agent".into(), ua.trim().into()]);
    }
    if let Some(r) = referrer.filter(|r| !r.trim().is_empty() && no_control(r)) {
        a.extend(["-referer".into(), r.trim().into()]);
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        a.extend(["-reconnect", "1", "-reconnect_streamed", "1", "-reconnect_delay_max", "10"].map(String::from));
    }
    a.extend(["-i".into(), url.into()]);
    a.extend(["-map", "0:v?", "-map", "0:a?", "-c", "copy"].map(String::from));
    a.extend(["-t".into(), duration_secs.max(1).to_string()]);
    a.extend(["-f", "mpegts"].map(String::from));
    a.push(out.to_string_lossy().into_owned());
    a
}

/// Vrije schijfruimte (bytes) op de schijf van `dir`, als die te bepalen is.
pub fn free_bytes(dir: &Path) -> Option<u64> {
    fs4::available_space(dir).ok()
}

/// Bestandsnaam zonder map: `Zender - Titel - 20261103-1015.ts` (veilig voor Windows).
pub fn file_name(channel: &str, title: &str, start_utc: i64) -> String {
    let (y, mo, d, h, mi, _) = crate::iptv::catchup::utc_parts(start_utc);
    let mut name = sanitize_file_name(&format!("{channel} - {title}"));
    name = name.chars().take(120).collect();
    format!("{name} - {y:04}{mo:02}{d:02}-{h:02}{mi:02}.ts")
}

/// Kiest een pad dat nog niet bestaat (`.ts` en `.ts.part`): bij een botsing komt er ` (2)`, ` (3)`, ... achter.
pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let stem = file_name.trim_end_matches(".ts");
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{stem}.ts") } else { format!("{stem} ({n}).ts") };
        let p = dir.join(&name);
        if !p.exists() && !dir.join(format!("{name}.part")).exists() {
            return p;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_copy_streams_with_headers_and_reconnect_for_http() {
        let a =
            build_args("http://h.example/live/1.ts", Some("Agent/1"), Some("https://ref.example/"), Path::new("D:\\op\\x.ts.part"), 3600);
        let joined = a.join(" ");
        assert!(joined.contains("-user_agent Agent/1") && joined.contains("-referer https://ref.example/"));
        assert!(joined.contains("-reconnect 1") && joined.contains("-i http://h.example/live/1.ts"));
        assert!(joined.contains("-map 0:v? -map 0:a? -c copy -t 3600 -f mpegts"));
        assert_eq!(a.last().unwrap(), "D:\\op\\x.ts.part");
        // de URL is een eigen argument, nooit samengevoegd met iets anders
        assert!(a.iter().any(|x| x == "http://h.example/live/1.ts"));
    }

    #[test]
    fn rtsp_gets_no_http_options_and_bad_headers_are_dropped() {
        let a = build_args("rtsp://h.example/x", Some("slecht\nagent"), Some(" "), Path::new("o.ts.part"), 0);
        let joined = a.join(" ");
        assert!(!joined.contains("-reconnect") && !joined.contains("-user_agent") && !joined.contains("-referer"));
        assert!(joined.contains("-t 1"), "een duur van 0 wordt 1 seconde");
    }

    #[test]
    fn file_names_are_windows_safe_and_dated_in_utc() {
        assert_eq!(file_name("Een: HD", "Nieuws/Weer?", 1_793_700_900), "Een_ HD - Nieuws_Weer_ - 20261103-1015.ts");
        let long = file_name(&"k".repeat(300), "t", 0);
        assert!(long.len() < 150 && long.ends_with("19700101-0000.ts"));
    }

    #[test]
    fn unique_paths_avoid_existing_files_and_part_files() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(dir.path(), "a.ts"), dir.path().join("a.ts"));
        std::fs::write(dir.path().join("a.ts"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.ts"), dir.path().join("a (2).ts"));
        std::fs::write(dir.path().join("a (2).ts.part"), b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "a.ts"), dir.path().join("a (3).ts"));
    }
}
