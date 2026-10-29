//! M3U/M3U8: lezen en schrijven, inclusief relatieve paden.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3uEntry {
    pub location: String,
    pub title: Option<String>,
    pub duration_s: Option<i64>,
}

fn is_url(s: &str) -> bool {
    match s.split_once("://") {
        Some((scheme, _)) => !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'),
        None => false,
    }
}

fn is_windows_absolute(s: &str) -> bool {
    let b = s.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')) || s.starts_with("\\\\")
}

/// Leest afspeellijsttekst. Relatieve paden worden opgelost ten opzichte van `base_dir`.
pub fn parse(content: &str, base_dir: &Path) -> Vec<M3uEntry> {
    let content = content.trim_start_matches('\u{feff}');
    let mut out = vec![];
    let mut pending: Option<(Option<i64>, Option<String>)> = None;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            let (dur, title) = info.split_once(',').unwrap_or((info, ""));
            let dur = dur.split_whitespace().next().and_then(|d| d.parse::<i64>().ok()).filter(|d| *d >= 0);
            let title = Some(title.trim().to_string()).filter(|t| !t.is_empty());
            pending = Some((dur, title));
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let location = if is_url(line) || is_windows_absolute(line) || Path::new(line).is_absolute() {
            line.to_string()
        } else {
            let rel = line.replace('\\', "/");
            base_dir.join(rel).to_string_lossy().into_owned()
        };
        let (duration_s, title) = pending.take().unwrap_or((None, None));
        out.push(M3uEntry { location, title, duration_s });
    }
    out
}

/// Schrijft een afspeellijst. Met `relative_to` worden bestanden onder die map relatief opgeslagen.
pub fn export(entries: &[M3uEntry], relative_to: Option<&Path>) -> String {
    let mut s = String::from("#EXTM3U\n");
    for e in entries {
        if e.title.is_some() || e.duration_s.is_some() {
            s.push_str(&format!(
                "#EXTINF:{},{}\n",
                e.duration_s.unwrap_or(-1),
                e.title.as_deref().unwrap_or("").replace(['\r', '\n'], " ")
            ));
        }
        let loc = match relative_to {
            Some(base) if !is_url(&e.location) => Path::new(&e.location)
                .strip_prefix(base)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| e.location.clone()),
            _ => e.location.clone(),
        };
        s.push_str(&loc);
        s.push('\n');
    }
    s
}

/// Leest een bestand: UTF-8, met terugval op Latin-1 voor oude .m3u-bestanden.
pub fn read_file(path: &Path) -> Result<Vec<M3uEntry>> {
    let bytes = std::fs::read(path)?;
    let text = match String::from_utf8(bytes.clone()) {
        Ok(t) => t,
        Err(_) => bytes.iter().map(|b| *b as char).collect(),
    };
    let base = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("/"));
    if !text.trim_start_matches('\u{feff}').lines().any(|l| !l.trim().is_empty()) {
        return Err(CoreError::Invalid("De afspeellijst is leeg".into()));
    }
    Ok(parse(&text, &base))
}

/// Welke lokale items bestaan niet (meer)? URL's tellen als aanwezig. Geeft de indexen terug.
pub fn missing_indexes(entries: &[M3uEntry]) -> Vec<usize> {
    entries.iter().enumerate().filter(|(_, e)| !is_url(&e.location) && !Path::new(&e.location).exists()).map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extinf_urls_comments_bom_and_crlf() {
        let text = "\u{feff}#EXTM3U\r\n# gewone opmerking\r\n#EXTINF:123,Mijn lied\r\n/muziek/a.mp3\r\n\r\n#EXTINF:-1,Radio\r\nhttps://radio.example.com/stream\r\n/muziek/zonder-titel.mp3\r\n";
        let e = parse(text, Path::new("/base"));
        assert_eq!(e.len(), 3);
        assert_eq!((e[0].location.as_str(), e[0].title.as_deref(), e[0].duration_s), ("/muziek/a.mp3", Some("Mijn lied"), Some(123)));
        assert_eq!((e[1].location.as_str(), e[1].duration_s), ("https://radio.example.com/stream", None)); // -1 = onbekend
        assert_eq!((e[2].title.clone(), e[2].duration_s), (None, None)); // EXTINF geldt maar voor één regel
    }

    #[test]
    fn resolves_relative_paths_including_windows_separators() {
        let e = parse("sub\\a.mp3\n../b.mp3\nC:\\Muziek\\c.mp3\n", Path::new("/lijsten"));
        assert_eq!(e[0].location, "/lijsten/sub/a.mp3");
        assert_eq!(e[1].location, "/lijsten/../b.mp3"); // pad blijft zichtbaar; validatie gebeurt bij openen
        assert_eq!(e[2].location, "C:\\Muziek\\c.mp3");
    }

    #[test]
    fn export_then_parse_roundtrips_with_relative_paths() {
        let entries = vec![
            M3uEntry { location: "/muziek/rock/a.mp3".into(), title: Some("A".into()), duration_s: Some(200) },
            M3uEntry { location: "/elders/b.mp3".into(), title: None, duration_s: None },
            M3uEntry { location: "http://x.example/s".into(), title: Some("Stream\nmet newline".into()), duration_s: None },
        ];
        let text = export(&entries, Some(Path::new("/muziek")));
        assert!(text.contains("\nrock/a.mp3\n"));
        assert!(text.contains("\n/elders/b.mp3\n")); // buiten de basismap blijft absoluut
        assert!(!text.contains("Stream\nmet")); // newline in titel kan de structuur niet breken
        let back = parse(&text, Path::new("/muziek"));
        assert_eq!(back.len(), 3);
        assert_eq!(back[0].location, "/muziek/rock/a.mp3");
        assert_eq!(back[0].duration_s, Some(200));
    }

    #[test]
    fn reads_utf8_and_latin1_files_and_rejects_empty() {
        let dir = tempfile::tempdir().unwrap();
        let utf8 = dir.path().join("a.m3u8");
        std::fs::write(&utf8, "#EXTM3U\n#EXTINF:1,Café\nlied.mp3\n").unwrap();
        assert_eq!(read_file(&utf8).unwrap()[0].title.as_deref(), Some("Café"));
        let latin1 = dir.path().join("b.m3u");
        std::fs::write(&latin1, b"#EXTM3U\n#EXTINF:1,Caf\xe9\nlied.mp3\n").unwrap();
        assert_eq!(read_file(&latin1).unwrap()[0].title.as_deref(), Some("Café"));
        let empty = dir.path().join("c.m3u");
        std::fs::write(&empty, "\n\n").unwrap();
        assert!(read_file(&empty).is_err());
    }

    #[test]
    fn detects_missing_local_files_but_not_urls() {
        let dir = tempfile::tempdir().unwrap();
        let bestaat = dir.path().join("a.mp3");
        std::fs::write(&bestaat, b"x").unwrap();
        let entries = vec![
            M3uEntry { location: bestaat.to_string_lossy().into(), title: None, duration_s: None },
            M3uEntry { location: dir.path().join("weg.mp3").to_string_lossy().into(), title: None, duration_s: None },
            M3uEntry { location: "https://x.example/s".into(), title: None, duration_s: None },
        ];
        assert_eq!(missing_indexes(&entries), vec![1]);
    }
}
