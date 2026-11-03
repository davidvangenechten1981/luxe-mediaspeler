//! Catch-up (terugkijken): bouwt de stream-URL voor een programma uit de livestream-URL, de modus en het sjabloon.
//! Alle tijden zijn UTC-seconden. Sommige aanbieders rekenen met lokale tijd; dat is hier niet te weten.

use crate::error::{CoreError, Result};
use url::Url;

#[derive(Debug, Clone, Copy)]
pub struct CatchupRequest<'a> {
    pub stream_url: &'a str,
    pub mode: &'a str,
    pub source: Option<&'a str>,
    pub start: i64,
    pub duration_secs: i64,
    /// Huidig tijdstip (voor `{lutc}`, `${timestamp}` en `{offset}`); als parameter zodat het testbaar blijft.
    pub now: i64,
}

/// (jaar, maand, dag, uur, minuut, seconde) in UTC.
pub fn utc_parts(secs: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// Vult de gangbare plaatsaanduidingen in (Kodi/TiviMate-stijl).
fn fill(template: &str, r: &CatchupRequest<'_>) -> String {
    let end = r.start + r.duration_secs;
    let (y, mo, d, h, mi, s) = utc_parts(r.start);
    let pairs: [(&str, String); 16] = [
        ("${start}", r.start.to_string()),
        ("${end}", end.to_string()),
        ("${timestamp}", r.now.to_string()),
        ("${duration}", r.duration_secs.to_string()),
        ("${offset}", (r.now - r.start).to_string()),
        ("{utcend}", end.to_string()),
        ("{utc}", r.start.to_string()),
        ("{lutc}", r.now.to_string()),
        ("{duration}", r.duration_secs.to_string()),
        ("{offset}", (r.now - r.start).to_string()),
        ("{Y}", format!("{y:04}")),
        ("{m}", format!("{mo:02}")),
        ("{d}", format!("{d:02}")),
        ("{H}", format!("{h:02}")),
        ("{M}", format!("{mi:02}")),
        ("{S}", format!("{s:02}")),
    ];
    pairs.iter().fold(template.to_string(), |acc, (k, v)| acc.replace(k, v))
}

fn separator(url: &str) -> char {
    if url.contains('?') {
        '&'
    } else {
        '?'
    }
}

pub fn build_url(r: &CatchupRequest<'_>) -> Result<String> {
    if r.duration_secs <= 0 || r.start <= 0 {
        return Err(CoreError::Invalid("Ongeldig tijdvak voor terugkijken".into()));
    }
    if r.start >= r.now {
        return Err(CoreError::Invalid("Dit programma is nog niet begonnen".into()));
    }
    let mode = r.mode.trim().to_ascii_lowercase();
    match mode.as_str() {
        "default" | "append" => {
            let source = r
                .source
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| CoreError::Invalid("Voor deze zender ontbreekt het catch-up-sjabloon".into()))?;
            let filled = fill(source.trim(), r);
            if mode == "default" && filled.contains("://") {
                Ok(filled)
            } else {
                Ok(format!("{}{filled}", r.stream_url))
            }
        }
        "shift" | "siptv" => Ok(format!("{}{}utc={}&lutc={}", r.stream_url, separator(r.stream_url), r.start, r.now)),
        "flussonic" | "fs" | "flussonic-ts" | "flussonic-hls" => {
            let (head, last) = r.stream_url.rsplit_once('/').ok_or_else(|| CoreError::Invalid("Onbekende Flussonic-URL".into()))?;
            let ts = r.stream_url.contains("mpegts") || last.ends_with(".ts");
            let name = if ts {
                format!("archive-{}-{}.ts", r.start, r.duration_secs)
            } else {
                format!("index-{}-{}.m3u8", r.start, r.duration_secs)
            };
            let _ = last;
            Ok(format!("{head}/{name}"))
        }
        "xc" | "xtream" | "xtreamcodes" => xtream_timeshift(r),
        other => Err(CoreError::Invalid(format!("Onbekende catch-up-modus: {other}"))),
    }
}

/// `http://host/live/USER/PASS/ID.ts` wordt `http://host/timeshift/USER/PASS/MINUTEN/JJJJ-MM-DD:UU-MM/ID.ts`.
fn xtream_timeshift(r: &CatchupRequest<'_>) -> Result<String> {
    let url = Url::parse(r.stream_url).map_err(|_| CoreError::Invalid("Ongeldige stream-URL".into()))?;
    let segs: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();
    if segs.len() != 4 || !segs[0].eq_ignore_ascii_case("live") {
        return Err(CoreError::Invalid("Dit is geen Xtream-livestream-URL".into()));
    }
    let id = segs[3].rsplit_once('.').map_or(segs[3], |(i, _)| i);
    let (y, mo, d, h, mi, _) = utc_parts(r.start);
    let minutes = (r.duration_secs + 59) / 60;
    let mut out = url.clone();
    out.set_query(None);
    {
        let mut path = out.path_segments_mut().map_err(|_| CoreError::Invalid("Ongeldige stream-URL".into()))?;
        path.clear().extend([
            "timeshift".to_string(),
            segs[1].to_string(),
            segs[2].to_string(),
            minutes.to_string(),
            format!("{y:04}-{mo:02}-{d:02}:{h:02}-{mi:02}"),
            format!("{id}.ts"),
        ]);
    }
    Ok(out.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-03 10:15:00 UTC
    const START: i64 = 1_793_700_900;
    const NOW: i64 = START + 7200;

    fn req<'a>(stream_url: &'a str, mode: &'a str, source: Option<&'a str>) -> CatchupRequest<'a> {
        CatchupRequest { stream_url, mode, source, start: START, duration_secs: 3600, now: NOW }
    }

    #[test]
    fn default_mode_fills_a_full_template() {
        let r = req(
            "http://h.example/live/1.ts",
            "default",
            Some("http://h.example/arch/1.ts?from=${start}&to=${end}&d=${duration}&o=${offset}&t=${timestamp}"),
        );
        assert_eq!(build_url(&r).unwrap(), format!("http://h.example/arch/1.ts?from={START}&to={}&d=3600&o=7200&t={NOW}", START + 3600));
    }

    #[test]
    fn append_mode_adds_to_the_stream_url_and_supports_date_parts() {
        let r = req("http://h.example/live/1.ts", "append", Some("?s={Y}{m}{d}T{H}{M}{S}&u={utc}&e={utcend}&l={lutc}"));
        assert_eq!(build_url(&r).unwrap(), format!("http://h.example/live/1.ts?s=20261103T101500&u={START}&e={}&l={NOW}", START + 3600));
        // een sjabloon zonder schema in modus default wordt ook achter de stream-URL gezet
        let r = req("http://h.example/live/1.ts", "default", Some("?utc={utc}"));
        assert_eq!(build_url(&r).unwrap(), format!("http://h.example/live/1.ts?utc={START}"));
    }

    #[test]
    fn shift_mode_uses_the_right_separator() {
        let r = req("http://h.example/live/1.ts", "shift", None);
        assert_eq!(build_url(&r).unwrap(), format!("http://h.example/live/1.ts?utc={START}&lutc={NOW}"));
        let r = req("http://h.example/live/1.ts?token=x", "SIPTV", None);
        assert_eq!(build_url(&r).unwrap(), format!("http://h.example/live/1.ts?token=x&utc={START}&lutc={NOW}"));
    }

    #[test]
    fn flussonic_mode_rewrites_the_last_segment() {
        let hls = req("http://h.example/kanaal1/index.m3u8", "flussonic", None);
        assert_eq!(build_url(&hls).unwrap(), format!("http://h.example/kanaal1/index-{START}-3600.m3u8"));
        let ts = req("http://h.example/kanaal1/mpegts", "fs", None);
        assert_eq!(build_url(&ts).unwrap(), format!("http://h.example/kanaal1/archive-{START}-3600.ts"));
    }

    #[test]
    fn xtream_mode_builds_a_timeshift_url() {
        let r = req("http://prov.example:8080/live/jan/pw/1234.ts", "xc", None);
        assert_eq!(build_url(&r).unwrap(), "http://prov.example:8080/timeshift/jan/pw/60/2026-11-03:10-15/1234.ts");
        let r = CatchupRequest { duration_secs: 3000 + 10, ..req("http://prov.example/live/jan/pw/77", "xtream", None) };
        assert_eq!(build_url(&r).unwrap(), "http://prov.example/timeshift/jan/pw/51/2026-11-03:10-15/77.ts", "minuten naar boven afgerond");
        assert!(build_url(&req("http://prov.example/movie/jan/pw/1.mkv", "xc", None)).is_err());
    }

    #[test]
    fn bad_input_is_refused() {
        assert!(build_url(&req("http://h/x", "onbekend", None)).is_err());
        assert!(build_url(&req("http://h/x", "default", None)).is_err(), "sjabloon ontbreekt");
        assert!(build_url(&CatchupRequest { duration_secs: 0, ..req("http://h/x", "shift", None) }).is_err());
        assert!(build_url(&CatchupRequest { start: NOW + 1, ..req("http://h/x", "shift", None) }).is_err(), "programma in de toekomst");
    }
}
