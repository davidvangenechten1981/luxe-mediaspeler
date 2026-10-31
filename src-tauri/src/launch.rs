//! Bestanden en URL's die via de opdrachtregel worden meegegeven (Verkenner, "Openen met", tweede instantie).

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct LaunchRequest {
    pub files: Vec<String>,
    /// `--enqueue`: toevoegen aan de wachtrij in plaats van direct afspelen.
    pub enqueue: bool,
}

/// Haalt bestanden uit de argumenten. Onbekende vlaggen worden genegeerd; paden worden niet gevalideerd
/// (dat doet de backend bij het openen, met dezelfde regels als overal).
pub fn parse(args: &[String]) -> LaunchRequest {
    let mut req = LaunchRequest::default();
    for a in args {
        match a.as_str() {
            "--enqueue" => req.enqueue = true,
            s if s.starts_with("--") => {}
            s if s.trim().is_empty() => {}
            s => req.files.push(s.to_string()),
        }
    }
    req
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn files_urls_and_flags_are_separated() {
        let r = parse(&args(&["C:\\Video's\\film één.mkv", "https://example.com/a.m3u8", "--enqueue", "--onbekend", ""]));
        assert_eq!(r.files, vec!["C:\\Video's\\film één.mkv", "https://example.com/a.m3u8"]);
        assert!(r.enqueue);
        assert_eq!(parse(&[]), LaunchRequest::default());
    }
}
