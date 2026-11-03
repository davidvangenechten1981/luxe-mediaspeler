//! IPTV-afspeellijsten (uitgebreide M3U): zenders met `#EXTINF`-attributen, groepen, kanaalnummers, logo's,
//! catch-up en per-stream headers. De attributen zijn conventies en geen standaard: de parser kent aliassen
//! en negeert wat hij niet snapt. Er wordt niets opgehaald; dit is zuivere tekstverwerking.

use serde::{Deserialize, Serialize};

/// Catch-up-gegevens uit de lijst (modus, bronsjabloon, aantal dagen).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catchup {
    pub mode: String,
    pub source: Option<String>,
    pub days: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub url: String,
    pub tvg_id: Option<String>,
    pub tvg_name: Option<String>,
    pub logo: Option<String>,
    pub group: Option<String>,
    pub number: Option<u32>,
    pub radio: bool,
    pub catchup: Option<Catchup>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedPlaylist {
    pub channels: Vec<Channel>,
    /// Gids-links uit de kop (`url-tvg` / `x-tvg-url`, kommagescheiden).
    pub epg_urls: Vec<String>,
    /// Aantal `#EXTINF`-blokken zonder bruikbare stream-URL.
    pub skipped: usize,
}

/// Zet bytes om naar tekst: UTF-8 (met of zonder BOM), anders elk byte als Latin-1 teken (verliesvrij).
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// Splitst `sleutel="waarde" sleutel2=waarde2` in paren. Sleutels worden kleine letters.
fn parse_attributes(s: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = s.chars().collect();
    let (mut i, mut out) = (0, vec![]);
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < chars.len() && (chars[i].is_alphanumeric() || matches!(chars[i], '-' | '_' | '.')) {
            i += 1;
        }
        if i == key_start || i >= chars.len() || chars[i] != '=' {
            i += 1; // geen sleutel=waarde: sla dit teken over
            continue;
        }
        let key: String = chars[key_start..i].iter().collect::<String>().to_ascii_lowercase();
        i += 1;
        let value: String = if i < chars.len() && chars[i] == '"' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            let v = chars[start..i].iter().collect();
            i += 1;
            v
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
            chars[start..i].iter().collect()
        };
        out.push((key, value));
    }
    out
}

/// Splitst de rest van een `#EXTINF`-regel (na de dubbele punt) in (attributen, naam) bij de eerste komma buiten aanhalingstekens.
fn split_extinf(rest: &str) -> (&str, &str) {
    let mut in_quotes = false;
    for (i, c) in rest.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => return (&rest[..i], rest[i + 1..].trim()),
            _ => {}
        }
    }
    (rest, "")
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Splitst `url|User-Agent=x&Referer=y` (Kodi-stijl) in de schone URL en de bekende headers. Andere headers vervallen.
fn split_url_headers(line: &str) -> (String, Option<String>, Option<String>) {
    let Some((url, headers)) = line.split_once('|') else { return (line.trim().to_string(), None, None) };
    let (mut ua, mut referrer) = (None, None);
    for pair in headers.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k.trim().to_ascii_lowercase().as_str() {
                "user-agent" => ua = non_empty(v),
                "referer" | "referrer" => referrer = non_empty(v),
                _ => {}
            }
        }
    }
    (url.trim().to_string(), ua, referrer)
}

pub fn parse(text: &str) -> ParsedPlaylist {
    let mut out = ParsedPlaylist::default();
    let mut pending: Option<Channel> = None;
    let mut group_override: Option<String> = None;

    for raw in text.lines() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXTM3U") {
            for (k, v) in parse_attributes(rest) {
                if matches!(k.as_str(), "url-tvg" | "x-tvg-url") {
                    out.epg_urls.extend(v.split(',').filter_map(non_empty));
                }
            }
        } else if let Some(rest) = line.strip_prefix("#EXTINF:") {
            if pending.is_some() {
                out.skipped += 1; // vorige blok zonder URL
            }
            let (attrs_part, name) = split_extinf(rest);
            // de duur (eerste token) hoort niet bij de attributen
            let attrs_part = attrs_part.split_once(char::is_whitespace).map_or("", |(_, a)| a);
            let mut ch = Channel { name: name.to_string(), ..Default::default() };
            let (mut catchup_mode, mut catchup_source, mut catchup_days) = (None, None, None);
            for (k, v) in parse_attributes(attrs_part) {
                match k.as_str() {
                    "tvg-id" => ch.tvg_id = non_empty(&v),
                    "tvg-name" => ch.tvg_name = non_empty(&v),
                    "tvg-logo" | "logo" => ch.logo = ch.logo.take().or_else(|| non_empty(&v)),
                    "group-title" => ch.group = non_empty(v.split(';').next().unwrap_or("")),
                    "tvg-chno" | "ch-number" | "channel-number" => ch.number = v.trim().parse().ok(),
                    "radio" => ch.radio = v.trim().eq_ignore_ascii_case("true") || v.trim() == "1",
                    "catchup" | "catchup-type" => catchup_mode = non_empty(&v),
                    "catchup-source" => catchup_source = non_empty(&v),
                    "catchup-days" | "tvg-rec" => catchup_days = v.trim().parse().ok(),
                    "http-user-agent" | "user-agent" => ch.user_agent = non_empty(&v),
                    "http-referrer" | "http-referer" => ch.referrer = non_empty(&v),
                    _ => {}
                }
            }
            if let Some(mode) = catchup_mode.or_else(|| catchup_source.as_ref().map(|_| "default".to_string())) {
                ch.catchup = Some(Catchup { mode: mode.to_ascii_lowercase(), source: catchup_source, days: catchup_days });
            }
            group_override = None;
            pending = Some(ch);
        } else if let Some(rest) = line.strip_prefix("#EXTGRP:") {
            group_override = non_empty(rest);
        } else if let Some(rest) = line.strip_prefix("#EXTVLCOPT:") {
            if let (Some(ch), Some((k, v))) = (pending.as_mut(), rest.split_once('=')) {
                match k.trim().to_ascii_lowercase().as_str() {
                    "http-user-agent" => ch.user_agent = non_empty(v),
                    "http-referrer" | "http-referer" => ch.referrer = non_empty(v),
                    _ => {}
                }
            }
        } else if line.starts_with('#') {
            // #KODIPROP, #EXTALB, opmerkingen: negeren
        } else if let Some(mut ch) = pending.take() {
            let (url, ua, referrer) = split_url_headers(line);
            if url.contains("://") {
                ch.url = url;
                ch.user_agent = ch.user_agent.or(ua);
                ch.referrer = ch.referrer.or(referrer);
                ch.group = ch.group.or_else(|| group_override.take());
                if ch.name.is_empty() {
                    ch.name = ch.tvg_name.clone().unwrap_or_else(|| ch.url.clone());
                }
                out.channels.push(ch);
            } else {
                out.skipped += 1;
            }
        }
        // een URL-regel zonder #EXTINF ervoor is geen zender (kale M3U's gebruiken we niet voor IPTV)
    }
    if pending.is_some() {
        out.skipped += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\u{feff}#EXTM3U url-tvg=\"https://gids.example/a.xml, https://gids.example/b.xml.gz\"\r\n\
#EXTINF:-1 tvg-id=\"een.nl\" tvg-name=\"Een HD\" tvg-logo=\"https://l.example/een.png\" tvg-chno=\"1\" group-title=\"Nieuws; Algemeen\" catchup=\"shift\" catchup-days=\"7\",Een, de eerste\r\n\
http://prov.example/live/jan/pw/1.ts\r\n\
#EXTINF:-1 radio=\"true\" group-title=\"Radio\",Klassiek FM\r\n\
#EXTVLCOPT:http-user-agent=MijnSpeler/1.0\r\n\
#EXTVLCOPT:http-referrer=https://ref.example/\r\n\
https://radio.example/klassiek.aac\r\n\
#EXTINF:-1,Zonder URL\r\n\
#EXTINF:-1 tvg-id=\"drie\",Drie\r\n\
#EXTGRP:Sport\r\n\
rtsp://cam.example/3|User-Agent=Foo&Referer=https://r.example/&X-Auth=geheim\r\n";

    #[test]
    fn a_full_extended_list_is_parsed() {
        let p = parse(LIST);
        assert_eq!(p.epg_urls, vec!["https://gids.example/a.xml", "https://gids.example/b.xml.gz"]);
        assert_eq!(p.channels.len(), 3);
        assert_eq!(p.skipped, 1, "het blok zonder URL telt als overgeslagen");

        let een = &p.channels[0];
        assert_eq!(een.name, "Een, de eerste", "komma's in de naam blijven staan");
        assert_eq!((een.tvg_id.as_deref(), een.tvg_name.as_deref()), (Some("een.nl"), Some("Een HD")));
        assert_eq!(een.logo.as_deref(), Some("https://l.example/een.png"));
        assert_eq!(een.number, Some(1));
        assert_eq!(een.group.as_deref(), Some("Nieuws"), "bij meerdere groepen geldt de eerste");
        assert_eq!(een.catchup, Some(Catchup { mode: "shift".into(), source: None, days: Some(7) }));
        assert_eq!(een.url, "http://prov.example/live/jan/pw/1.ts");

        let radio = &p.channels[1];
        assert!(radio.radio);
        assert_eq!(radio.user_agent.as_deref(), Some("MijnSpeler/1.0"));
        assert_eq!(radio.referrer.as_deref(), Some("https://ref.example/"));

        let drie = &p.channels[2];
        assert_eq!(drie.group.as_deref(), Some("Sport"), "#EXTGRP vult een ontbrekende groep aan");
        assert_eq!(drie.url, "rtsp://cam.example/3", "de header-suffix verdwijnt uit de URL");
        assert_eq!(drie.user_agent.as_deref(), Some("Foo"));
        assert_eq!(drie.referrer.as_deref(), Some("https://r.example/"));
    }

    #[test]
    fn unknown_header_suffix_values_are_dropped_not_kept() {
        let p = parse("#EXTINF:-1,X\nhttp://h.example/x|X-Auth=geheim&Cookie=abc\n");
        let ch = &p.channels[0];
        assert_eq!(ch.url, "http://h.example/x");
        let dump = format!("{ch:?}");
        assert!(!dump.contains("geheim") && !dump.contains("abc"));
    }

    #[test]
    fn name_falls_back_to_tvg_name_then_url() {
        let p = parse("#EXTINF:-1 tvg-name=\"Alleen tvg\",\nhttp://h.example/a\n#EXTINF:-1,\nhttp://h.example/b\n");
        assert_eq!(p.channels[0].name, "Alleen tvg");
        assert_eq!(p.channels[1].name, "http://h.example/b");
    }

    #[test]
    fn catchup_source_without_mode_means_default_and_aliases_work() {
        let p = parse("#EXTINF:-1 ch-number=\"12\" catchup-source=\"http://h/{utc}\" tvg-rec=\"3\",X\nhttp://h/x\n");
        let ch = &p.channels[0];
        assert_eq!(ch.number, Some(12));
        assert_eq!(ch.catchup, Some(Catchup { mode: "default".into(), source: Some("http://h/{utc}".into()), days: Some(3) }));
    }

    #[test]
    fn junk_and_lists_without_channels_are_harmless() {
        assert_eq!(parse("").channels.len(), 0);
        assert_eq!(parse("geen m3u, gewoon tekst\n").channels.len(), 0);
        let p = parse("#EXTM3U\nhttp://zonder-extinf.example/x\n");
        assert_eq!(p.channels.len(), 0);
    }

    #[test]
    fn bytes_are_decoded_as_utf8_with_a_latin1_fallback() {
        assert_eq!(decode_text("\u{feff}café".as_bytes()), "café");
        assert_eq!(decode_text(b"caf\xe9"), "café");
    }
}
