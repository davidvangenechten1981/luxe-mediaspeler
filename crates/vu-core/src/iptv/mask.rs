//! Maskeert inloggegevens in stream-URL's en in vrije tekst (logs, diagnose, export).

use url::Url;

/// Queryparameters waarvan de waarde nooit in een log of export mag komen (hoofdletterongevoelig).
const SENSITIVE_KEYS: &[&str] = &[
    "username",
    "user",
    "password",
    "pass",
    "pwd",
    "token",
    "key",
    "apikey",
    "api_key",
    "auth",
    "secret",
    "access_token",
    "sig",
    "signature",
];

/// Xtream-achtige paden: `/<soort>/<gebruiker>/<wachtwoord>/<id>...`.
const XTREAM_KINDS: &[&str] = &["live", "movie", "series", "timeshift"];

const MASK: &str = "***";

/// Maskeert gebruikersnaam/wachtwoord, Xtream-paden en gevoelige queryparameters.
/// Het schema, de host, de poort en de rest van het pad blijven zichtbaar; een fragment (`#...`) valt weg.
pub fn mask_url(input: &str) -> String {
    let Ok(url) = Url::parse(input) else { return input.to_string() };
    let Some(host) = url.host_str() else { return input.to_string() };

    let mut out = format!("{}://", url.scheme());
    if url.password().is_some() {
        out.push_str("***:***@");
    } else if !url.username().is_empty() {
        out.push_str("***@");
    }
    out.push_str(host);
    if let Some(port) = url.port() {
        out.push_str(&format!(":{port}"));
    }

    let mut parts: Vec<&str> = url.path().split('/').collect(); // parts[0] is leeg (voor de eerste /)
    if parts.len() >= 5 && XTREAM_KINDS.contains(&parts[1].to_ascii_lowercase().as_str()) {
        parts[2] = MASK;
        parts[3] = MASK;
    }
    out.push_str(&parts.join("/"));

    if let Some(query) = url.query() {
        let masked: Vec<String> = query
            .split('&')
            .map(|pair| match pair.split_once('=') {
                Some((key, _)) if SENSITIVE_KEYS.contains(&key.to_ascii_lowercase().as_str()) => format!("{key}={MASK}"),
                _ => pair.to_string(),
            })
            .collect();
        out.push('?');
        out.push_str(&masked.join("&"));
    }
    out
}

/// Maskeert elke URL die in een stuk tekst staat; de rest blijft ongewijzigd.
/// Een URL eindigt bij een spatie, aanhalingsteken, `<`, `>` of `)`; leestekens aan het eind horen er niet bij.
pub fn mask_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(pos) = rest.find("://") {
        let scheme_len = rest[..pos].chars().rev().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')).count();
        let start = pos - scheme_len; // schematekens zijn ASCII: tekens == bytes
        let after = &rest[pos + 3..];
        let candidate_len = after.find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')')).unwrap_or(after.len());
        let trimmed_len = after[..candidate_len].trim_end_matches([',', '.', ';', ':', '!', '?']).len();
        let url_end = pos + 3 + trimmed_len;
        out.push_str(&rest[..start]);
        out.push_str(&mask_url(&rest[start..url_end]));
        rest = &rest[url_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn userinfo_is_masked() {
        assert_eq!(mask_url("rtsp://admin:geheim@192.0.2.5:554/stream1"), "rtsp://***:***@192.0.2.5:554/stream1");
        assert_eq!(mask_url("http://alleen@example.com/x"), "http://***@example.com/x");
    }

    #[test]
    fn xtream_paths_hide_user_and_password() {
        assert_eq!(mask_url("http://prov.example:8080/live/jan/wachtwoord1/1234.ts"), "http://prov.example:8080/live/***/***/1234.ts");
        assert_eq!(mask_url("http://prov.example/movie/jan/wachtwoord1/55.mkv"), "http://prov.example/movie/***/***/55.mkv");
        assert_eq!(mask_url("http://prov.example/series/jan/wachtwoord1/9.mp4"), "http://prov.example/series/***/***/9.mp4");
        assert_eq!(
            mask_url("http://prov.example/timeshift/jan/wachtwoord1/60/2026-11-03:10-00/77.ts"),
            "http://prov.example/timeshift/***/***/60/2026-11-03:10-00/77.ts"
        );
    }

    #[test]
    fn sensitive_query_values_are_masked_and_others_kept() {
        assert_eq!(
            mask_url("http://prov.example/get.php?username=jan&password=pw&type=m3u_plus&output=ts"),
            "http://prov.example/get.php?username=***&password=***&type=m3u_plus&output=ts"
        );
        assert_eq!(mask_url("https://x.example/l.m3u8?Token=abc123&id=7"), "https://x.example/l.m3u8?Token=***&id=7");
        assert_eq!(mask_url("https://x.example/l.m3u8?api_key=k&sig=s&a=b"), "https://x.example/l.m3u8?api_key=***&sig=***&a=b");
    }

    #[test]
    fn fragments_are_dropped() {
        assert_eq!(mask_url("https://x.example/a.m3u8#token=abc"), "https://x.example/a.m3u8");
    }

    #[test]
    fn harmless_urls_and_plain_text_stay_as_they_are() {
        assert_eq!(mask_url("https://example.com/live/stream.m3u8"), "https://example.com/live/stream.m3u8");
        assert_eq!(mask_url("geen url"), "geen url");
        assert_eq!(mask_url(""), "");
    }

    #[test]
    fn text_with_urls_is_masked_in_place() {
        let line = "afspelen mislukt voor http://prov.example/live/jan/pw/1.ts, nieuwe poging; ook (rtsp://a:b@h/x).";
        assert_eq!(
            mask_text(line),
            "afspelen mislukt voor http://prov.example/live/***/***/1.ts, nieuwe poging; ook (rtsp://***:***@h/x)."
        );
        assert_eq!(mask_text("geen urls hier"), "geen urls hier");
    }

    #[test]
    fn no_secret_survives_in_any_sample() {
        let samples = [
            ("http://u1:s3cr3t@h.example/a", "s3cr3t"),
            ("http://h.example/live/gebruiker9/hunter2/5.ts", "hunter2"),
            ("http://h.example/x?password=hunter2&username=gebruiker9", "hunter2"),
            ("https://h.example/p.m3u8?token=tok_123#frag_456", "tok_123"),
            ("https://h.example/p.m3u8?token=tok_123#frag_456", "frag_456"),
        ];
        for (url, secret) in samples {
            assert!(!mask_url(url).contains(secret), "{url} lekt {secret}");
            assert!(!mask_text(&format!("fout bij {url}!")).contains(secret), "tekst lekt {secret}");
        }
    }
}
