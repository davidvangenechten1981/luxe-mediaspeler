//! Validatie van paden, URL's en bestandsnamen. Alles wat van de interface
//! komt, gaat hier eerst doorheen (padmanipulatie, SSRF, commando-injectie).

use crate::error::{CoreError, Result};
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};

/// Geldig lokaal mediabestand: absoluut, zonder `..`, bestaand en een bestand.
pub fn validate_media_path(input: &str) -> Result<PathBuf> {
    let p = validate_path_shape(input)?;
    if !p.exists() {
        return Err(CoreError::NotFound(p.display().to_string()));
    }
    if !p.is_file() {
        return Err(CoreError::Invalid("Het pad is geen bestand".into()));
    }
    Ok(p)
}

/// Geldige map: absoluut, zonder `..`, bestaand en een map.
pub fn validate_folder_path(input: &str) -> Result<PathBuf> {
    let p = validate_path_shape(input)?;
    if !p.is_dir() {
        return Err(CoreError::NotFound(p.display().to_string()));
    }
    Ok(p)
}

/// Pad dat nog niet hoeft te bestaan (bv. uitvoermap of -bestand).
pub fn validate_path_shape(input: &str) -> Result<PathBuf> {
    if input.trim().is_empty() {
        return Err(CoreError::Invalid("Leeg pad".into()));
    }
    if input.contains('\0') {
        return Err(CoreError::Invalid("Pad bevat een ongeldig teken".into()));
    }
    let p = PathBuf::from(input);
    if !p.is_absolute() {
        return Err(CoreError::Invalid("Alleen absolute paden zijn toegestaan".into()));
    }
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(CoreError::Invalid("Paden met '..' zijn niet toegestaan".into()));
    }
    Ok(p)
}

/// Schoon een bestandsnaam op: geen mapscheiding, geen stuurtekens, geen gereserveerde Windows-namen.
pub fn sanitize_file_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    out = out.trim().trim_end_matches(['.', ' ']).to_string();
    if out.is_empty() {
        out = "naamloos".into();
    }
    const RESERVED: [&str; 8] = ["CON", "PRN", "AUX", "NUL", "COM1", "LPT1", "COM2", "LPT2"];
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        out = format!("_{out}");
    }
    out.chars().take(150).collect()
}

const STREAM_SCHEMES: [&str; 6] = ["http", "https", "rtsp", "rtp", "udp", "rtmp"];

/// Controleert een netwerk-URL. Lokale en interne adressen worden geweigerd,
/// tenzij `allow_local` expliciet aan staat.
pub fn validate_stream_url(input: &str, allow_local: bool) -> Result<url::Url> {
    let u = url::Url::parse(input.trim()).map_err(|_| CoreError::Invalid("Ongeldige URL".into()))?;
    if !STREAM_SCHEMES.contains(&u.scheme()) {
        return Err(CoreError::Invalid(format!("Protocol '{}' wordt niet ondersteund", u.scheme())));
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Err(CoreError::Invalid("URL's met inloggegevens zijn niet toegestaan".into()));
    }
    let host = u.host_str().ok_or_else(|| CoreError::Invalid("URL zonder host".into()))?;
    if !allow_local && is_local_host(host) {
        return Err(CoreError::Invalid("Lokale en interne netwerkadressen zijn geblokkeerd".into()));
    }
    Ok(u)
}

fn is_local_host(host: &str) -> bool {
    let h = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if h == "localhost" || h.ends_with(".localhost") || h.ends_with(".local") || h.ends_with(".internal") {
        return true;
    }
    match h.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified() || ip.is_broadcast()
        }
        Ok(IpAddr::V6(ip)) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || (ip.segments()[0] & 0xfe00) == 0xfc00 // unique local
                || (ip.segments()[0] & 0xffc0) == 0xfe80 // link local
        }
        Err(_) => false,
    }
}

/// Is dit pad een toegestaan uitvoerpad voor `input` (niet hetzelfde bestand overschrijven)?
pub fn ensure_not_same_file(input: &Path, output: &Path) -> Result<()> {
    if input == output {
        return Err(CoreError::Invalid("Uitvoer mag niet hetzelfde zijn als de invoer".into()));
    }
    if let (Ok(a), Ok(b)) = (input.canonicalize(), output.canonicalize()) {
        if a == b {
            return Err(CoreError::Invalid("Uitvoer mag niet hetzelfde zijn als de invoer".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_relative_and_traversal_paths() {
        assert!(validate_path_shape("").is_err());
        assert!(validate_path_shape("   ").is_err());
        assert!(validate_path_shape("relatief/pad.mp4").is_err());
        assert!(validate_path_shape("/tmp/../etc/passwd").is_err());
        assert!(validate_path_shape("/tmp/a\0b").is_err());
    }

    #[test]
    fn accepts_existing_file_and_rejects_directory_or_missing() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a b é.mkv");
        std::fs::write(&f, b"x").unwrap();
        assert!(validate_media_path(f.to_str().unwrap()).is_ok());
        assert!(validate_media_path(dir.path().to_str().unwrap()).is_err());
        assert!(matches!(
            validate_media_path(dir.path().join("nee.mkv").to_str().unwrap()),
            Err(CoreError::NotFound(_))
        ));
    }

    #[test]
    fn sanitizes_file_names() {
        assert_eq!(sanitize_file_name("a/b\\c:d*e?.mp4"), "a_b_c_d_e_.mp4");
        assert_eq!(sanitize_file_name("   "), "naamloos");
        assert_eq!(sanitize_file_name("CON.txt"), "_CON.txt");
        assert_eq!(sanitize_file_name("film."), "film");
        assert_eq!(sanitize_file_name(&"x".repeat(400)).len(), 150);
    }

    #[test]
    fn stream_urls_allow_public_and_block_internal_hosts() {
        assert!(validate_stream_url("https://example.com/live.m3u8", false).is_ok());
        assert!(validate_stream_url("rtsp://203.0.113.5:554/stream", false).is_ok());
        for bad in [
            "http://localhost/x",
            "http://127.0.0.1/x",
            "http://192.168.1.10/x",
            "http://10.0.0.1/x",
            "http://169.254.169.254/latest/meta-data",
            "http://[::1]/x",
            "http://printer.local/x",
        ] {
            assert!(validate_stream_url(bad, false).is_err(), "{bad} moest geblokkeerd zijn");
        }
        assert!(validate_stream_url("http://192.168.1.10/x", true).is_ok());
    }

    #[test]
    fn stream_urls_reject_bad_schemes_and_credentials() {
        assert!(validate_stream_url("file:///etc/passwd", true).is_err());
        assert!(validate_stream_url("javascript:alert(1)", true).is_err());
        assert!(validate_stream_url("https://user:pw@example.com/", false).is_err());
        assert!(validate_stream_url("geen url", false).is_err());
    }

    #[test]
    fn refuses_overwriting_the_input_file() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.mkv");
        std::fs::write(&f, b"x").unwrap();
        assert!(ensure_not_same_file(&f, &f).is_err());
        assert!(ensure_not_same_file(&f, &dir.path().join("b.mkv")).is_ok());
    }
}
