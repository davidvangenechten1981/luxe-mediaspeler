//! Ophalen van afspeellijsten, gidsen en Xtream-antwoorden via `curl.exe` (zit in Windows 10/11).
//! De URL (mogelijk met inloggegevens) gaat via de configuratie op stdin en staat dus niet in de opdrachtregel.
//! Alleen http en https; omleidingen blijven daartoe beperkt; de grootte is begrensd.

use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use vu_core::error::{CoreError, Result};
use vu_core::iptv::mask::mask_text;
use vu_core::validate::validate_stream_url;

/// Grootste bestand dat we ophalen (voldoende voor flinke gidsen en lijsten).
pub const MAX_DOWNLOAD_BYTES: u64 = 300 * 1024 * 1024;

fn curl_exe() -> PathBuf {
    #[cfg(windows)]
    {
        let root = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows"));
        root.join("System32").join("curl.exe")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("curl")
    }
}

/// Zet een waarde veilig tussen aanhalingstekens voor een curl-configuratieregel.
fn curl_quote(value: &str) -> Result<String> {
    if value.chars().any(char::is_control) {
        return Err(CoreError::Invalid("Ongeldige tekens in het adres of de browsernaam".into()));
    }
    Ok(format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")))
}

/// Haalt `url` op en geeft de bytes terug. `allow_local` komt uit de instelling "lokale streams toestaan".
pub fn fetch(url: &str, user_agent: Option<&str>, allow_local: bool) -> Result<Vec<u8>> {
    let checked = validate_stream_url(url, allow_local)?;
    if !matches!(checked.scheme(), "http" | "https") {
        return Err(CoreError::Invalid("Alleen http- en https-adressen kunnen worden opgehaald".into()));
    }
    let mut config = format!("url = {}\n", curl_quote(checked.as_str())?);
    if let Some(ua) = user_agent.map(str::trim).filter(|u| !u.is_empty()) {
        config.push_str(&format!("user-agent = {}\n", curl_quote(ua)?));
    }
    let mut child = vu_core::proc::quiet(curl_exe())
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-redirs",
            "5",
            "--proto",
            "=http,https",
            "--proto-redir",
            "=http,https",
            "--connect-timeout",
            "15",
            "--max-time",
            "180",
            "--max-filesize",
            &MAX_DOWNLOAD_BYTES.to_string(),
            "--config",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::Invalid(format!("curl kon niet worden gestart: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(config.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| CoreError::Invalid(format!("Ophalen mislukt: {e}")))?;
    if out.status.success() {
        return Ok(out.stdout);
    }
    let detail = mask_text(String::from_utf8_lossy(&out.stderr).trim());
    let detail: String = detail.chars().take(300).collect();
    Err(CoreError::Invalid(format!(
        "Ophalen mislukt: {}",
        if detail.is_empty() { "geen antwoord van de server".to_string() } else { detail }
    )))
}

#[cfg(test)]
pub mod testserver {
    //! Kleine HTTP-server voor tests: antwoordt op een pad (zonder query) met vaste bytes.
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    pub struct Server {
        pub port: u16,
        pub hits: Arc<Mutex<Vec<String>>>,
    }

    impl Server {
        pub fn url(&self, path: &str) -> String {
            format!("http://127.0.0.1:{}{path}", self.port)
        }
    }

    /// `routes`: pad -> (content-type, bytes). Alles anders geeft 404. `hits` bewaart elke aangevraagde pad+query.
    pub fn serve(routes: HashMap<String, (String, Vec<u8>)>) -> Server {
        serve_with(move |target| routes.get(target.split('?').next().unwrap_or("/")).cloned())
    }

    /// Zoals `serve`, maar het antwoord hangt af van het volledige verzoek (pad en query).
    pub fn serve_with(handler: impl Fn(&str) -> Option<(String, Vec<u8>)> + Send + Sync + 'static) -> Server {
        let handler = Arc::new(handler);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(Mutex::new(vec![]));
        let hits2 = hits.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (handler, hits) = (handler.clone(), hits2.clone());
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() {
                        return;
                    }
                    loop {
                        let mut h = String::new();
                        if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                            break;
                        }
                    }
                    let target = line.split_whitespace().nth(1).unwrap_or("/").to_string();
                    hits.lock().unwrap().push(target.clone());
                    let mut out = stream;
                    match handler(&target) {
                        Some((ct, body)) => {
                            let _ = write!(
                                out,
                                "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            let _ = out.write_all(&body);
                        }
                        None => {
                            let _ = write!(out, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        }
                    }
                });
            }
        });
        Server { port, hits }
    }
}

#[cfg(test)]
mod tests {
    use super::testserver::serve;
    use super::*;
    use std::collections::HashMap;

    fn routes(path: &str, body: &[u8]) -> HashMap<String, (String, Vec<u8>)> {
        HashMap::from([(path.to_string(), ("text/plain".to_string(), body.to_vec()))])
    }

    #[test]
    fn a_page_is_fetched_with_the_given_user_agent_and_missing_pages_fail() {
        let s = serve(routes("/lijst.m3u", b"#EXTM3U\n"));
        assert_eq!(fetch(&s.url("/lijst.m3u"), Some("Agent/1"), true).unwrap(), b"#EXTM3U\n");
        let err = fetch(&s.url("/bestaat-niet"), None, true).unwrap_err().to_string();
        assert!(err.contains("Ophalen mislukt") && err.contains("404"), "{err}");
    }

    #[test]
    fn local_addresses_and_other_schemes_are_refused_without_permission() {
        let s = serve(routes("/x", b"x"));
        assert!(fetch(&s.url("/x"), None, false).is_err(), "lokaal adres zonder toestemming");
        assert!(fetch("ftp://example.com/x", None, true).is_err());
        assert!(fetch("rtsp://203.0.113.5/x", None, true).is_err(), "rtsp is geen ophaalbaar adres");
        assert!(fetch("file:///C:/geheim.txt", None, true).is_err());
    }

    #[test]
    fn user_agents_with_control_characters_are_refused() {
        let s = serve(routes("/x", b"x"));
        assert!(fetch(&s.url("/x"), Some("a\nurl = \"file:///etc/passwd\""), true).is_err(), "geen extra configuratieregels");
    }

    #[test]
    fn errors_never_contain_the_password_of_the_url() {
        // poort 1: niemand luistert; curl noemt de URL in de foutmelding
        let err = fetch("http://jan:geheim99@127.0.0.1:1/live/jan/geheim99/1.ts", None, true).unwrap_err().to_string();
        assert!(!err.contains("geheim99"), "{err}");
    }
}
