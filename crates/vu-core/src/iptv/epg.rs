//! Programmagids (EPG): XMLTV lezen en gidszenders aan afspeellijstzenders koppelen.
//! Alleen verwerking van bytes; het ophalen gebeurt elders.

use crate::error::{CoreError, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Read;

/// Bovengrens voor een (uitgepakte) gids, tegen enorme of kwaadaardige bestanden.
pub const MAX_XML_BYTES: u64 = 300 * 1024 * 1024;
/// Bovengrens voor het aantal programma's dat we bewaren.
pub const MAX_PROGRAMMES: usize = 1_500_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EpgChannel {
    pub id: String,
    pub names: Vec<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Programme {
    pub channel: String,
    /// Seconden sinds 1970 (UTC).
    pub start: i64,
    pub stop: i64,
    pub title: String,
    pub desc: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Guide {
    pub channels: Vec<EpgChannel>,
    pub programmes: Vec<Programme>,
}

/// Pakt gzip uit als de bytes daarmee beginnen; anders worden ze ongewijzigd teruggegeven.
pub fn gunzip_if_needed(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(MAX_XML_BYTES + 1)
            .read_to_end(&mut out)
            .map_err(|e| CoreError::Invalid(format!("De gids kon niet worden uitgepakt: {e}")))?;
        if out.len() as u64 > MAX_XML_BYTES {
            return Err(CoreError::Invalid("De gids is uitgepakt te groot".into()));
        }
        Ok(out)
    } else {
        Ok(bytes.to_vec())
    }
}

/// Dagen sinds 1970-01-01 voor een datum in de Gregoriaanse kalender (Howard Hinnant).
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// "20261103101500 +0100" (seconden en zone mogen ontbreken; zonder zone geldt UTC) naar seconden sinds 1970 in UTC.
pub fn parse_xmltv_time(s: &str) -> Option<i64> {
    let s = s.trim();
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !(12..=14).contains(&digits.len()) {
        return None;
    }
    let num = |a: usize, b: usize| -> Option<i64> { digits.get(a..b).and_then(|x| x.parse().ok()) };
    let (y, mo, d, h, mi) = (num(0, 4)?, num(4, 6)?, num(6, 8)?, num(8, 10)?, num(10, 12)?);
    let sec = if digits.len() >= 14 { num(12, 14)? } else { 0 };
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    let mut utc = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + sec;
    let zone = s[digits.len()..].trim();
    if let Some(sign) = zone.chars().next().filter(|c| matches!(c, '+' | '-')) {
        let z = &zone[1..];
        if z.len() == 4 && z.chars().all(|c| c.is_ascii_digit()) {
            let off = z[..2].parse::<i64>().ok()? * 3600 + z[2..].parse::<i64>().ok()? * 60;
            utc -= if sign == '+' { off } else { -off };
        }
    }
    Some(utc)
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    None,
    DisplayName,
    Title,
    Desc,
}

fn attr(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok())
        .map(|v| v.into_owned())
}

/// Leest XMLTV. `window` = (van, tot) in UTC-seconden: programma's buiten dat venster worden weggelaten.
/// Een afgebroken bestand levert op wat al gelezen was; alleen als er niets bruikbaars is volgt een fout.
pub fn parse_xmltv(bytes: &[u8], window: Option<(i64, i64)>) -> Result<Guide> {
    let mut reader = Reader::from_reader(bytes);
    let mut guide = Guide::default();
    let (mut channel, mut prog): (Option<EpgChannel>, Option<Programme>) = (None, None);
    let (mut target, mut text) = (Target::None, String::new());
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                "channel" => channel = attr(&e, "id").filter(|i| !i.is_empty()).map(|id| EpgChannel { id, ..Default::default() }),
                "display-name" if channel.is_some() => (target, text) = (Target::DisplayName, String::new()),
                "programme" => {
                    prog = match (
                        attr(&e, "channel"),
                        attr(&e, "start").and_then(|s| parse_xmltv_time(&s)),
                        attr(&e, "stop").and_then(|s| parse_xmltv_time(&s)),
                    ) {
                        (Some(channel), Some(start), Some(stop)) if stop > start => {
                            Some(Programme { channel, start, stop, title: String::new(), desc: None })
                        }
                        _ => None,
                    }
                }
                "title" if prog.is_some() => (target, text) = (Target::Title, String::new()),
                "desc" if prog.is_some() => (target, text) = (Target::Desc, String::new()),
                "icon" => {
                    if let (Some(c), Some(src)) = (channel.as_mut(), attr(&e, "src")) {
                        c.icon.get_or_insert(src);
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == "icon" {
                    if let (Some(c), Some(src)) = (channel.as_mut(), attr(&e, "src")) {
                        c.icon.get_or_insert(src);
                    }
                }
            }
            Ok(Event::Text(t)) if target != Target::None => text.push_str(&t.xml10_content()),
            Ok(Event::CData(c)) if target != Target::None => text.push_str(&c.xml10_content()),
            Ok(Event::GeneralRef(r)) if target != Target::None => {
                if r.is_char_ref() {
                    if let Ok(Some(ch)) = r.resolve_char_ref() {
                        text.push(ch);
                    }
                } else {
                    match r.as_ref() {
                        "amp" => text.push('&'),
                        "lt" => text.push('<'),
                        "gt" => text.push('>'),
                        "quot" => text.push('"'),
                        "apos" => text.push('\''),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                "display-name" => {
                    if let (Some(c), true) = (channel.as_mut(), target == Target::DisplayName) {
                        let name = text.trim();
                        if !name.is_empty() {
                            c.names.push(name.to_string());
                        }
                    }
                    target = Target::None;
                }
                "title" => {
                    if let (Some(p), true) = (prog.as_mut(), target == Target::Title) {
                        if p.title.is_empty() {
                            p.title = text.trim().to_string();
                        }
                    }
                    target = Target::None;
                }
                "desc" => {
                    if let (Some(p), true) = (prog.as_mut(), target == Target::Desc) {
                        if p.desc.is_none() && !text.trim().is_empty() {
                            p.desc = Some(text.trim().to_string());
                        }
                    }
                    target = Target::None;
                }
                "channel" => {
                    if let Some(c) = channel.take() {
                        guide.channels.push(c);
                    }
                }
                "programme" => {
                    if let Some(p) = prog.take() {
                        let in_window = window.is_none_or(|(from, to)| p.stop > from && p.start < to);
                        if in_window && !p.title.is_empty() && guide.programmes.len() < MAX_PROGRAMMES {
                            guide.programmes.push(p);
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => {
                if guide.channels.is_empty() && guide.programmes.is_empty() {
                    return Err(CoreError::Invalid(format!("Dit is geen geldige XMLTV-gids: {e}")));
                }
                break; // afgebroken bestand: bewaar wat er is
            }
            _ => {}
        }
        buf.clear();
    }
    if guide.channels.is_empty() && guide.programmes.is_empty() {
        return Err(CoreError::Invalid("De gids bevat geen zenders of programma's".into()));
    }
    Ok(guide)
}

/// Maakt een gids-id vergelijkbaar: kleine letters en zonder `@SD`/`@HD`-achtervoegsel.
pub fn normalize_id(id: &str) -> String {
    id.trim().split('@').next().unwrap_or("").trim().to_lowercase()
}

/// Maakt een zendernaam vergelijkbaar: kleine letters, alleen letters en cijfers, zonder HD/SD/4K-woorden.
pub fn normalize_name(name: &str) -> String {
    const NOISE: &[&str] = &["hd", "fhd", "uhd", "sd", "4k", "hevc", "h265", "raw"];
    name.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty() && !NOISE.contains(t)).collect::<Vec<_>>().concat()
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelRef<'a> {
    pub tvg_id: Option<&'a str>,
    pub name: &'a str,
}

/// Zoekt voor elke zender de bijbehorende gidszender (id uit het XMLTV-bestand), in deze volgorde:
/// 1. exact `tvg-id`, 2. `tvg-id` zonder achtervoegsel en hoofdletters, 3. genormaliseerde naam.
pub fn match_channels(channels: &[ChannelRef<'_>], epg: &[EpgChannel]) -> Vec<Option<String>> {
    let exact: HashMap<&str, &str> = epg.iter().map(|c| (c.id.as_str(), c.id.as_str())).collect();
    let mut by_norm_id: HashMap<String, &str> = HashMap::new();
    let mut by_name: HashMap<String, &str> = HashMap::new();
    for c in epg {
        by_norm_id.entry(normalize_id(&c.id)).or_insert(&c.id);
        for n in &c.names {
            let key = normalize_name(n);
            if !key.is_empty() {
                by_name.entry(key).or_insert(&c.id);
            }
        }
    }
    channels
        .iter()
        .map(|ch| {
            let by_id = ch
                .tvg_id
                .filter(|i| !i.trim().is_empty())
                .and_then(|id| exact.get(id.trim()).copied().or_else(|| by_norm_id.get(&normalize_id(id)).copied()));
            by_id
                .or_else(|| {
                    let key = normalize_name(ch.name);
                    (!key.is_empty()).then(|| by_name.get(&key).copied()).flatten()
                })
                .map(str::to_string)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<tv generator-info-name="test">
  <channel id="een.nl@SD">
    <display-name>Een HD</display-name>
    <display-name lang="nl">Een</display-name>
    <icon src="https://l.example/een.png"/>
  </channel>
  <channel id="twee.nl"><display-name>Twee</display-name></channel>
  <programme start="20261103101500 +0100" stop="20261103110000 +0100" channel="een.nl@SD">
    <title lang="nl">Nieuws &amp; Weer</title>
    <desc><![CDATA[Het laatste <b>nieuws</b>]]></desc>
    <category>Nieuws</category>
  </programme>
  <programme start="20261103110000 +0100" stop="20261103120000 +0100" channel="een.nl@SD"><title>Film &#233;</title></programme>
  <programme start="kapot" stop="20261103120000 +0100" channel="twee.nl"><title>Foutief</title></programme>
  <programme start="20261103110000" stop="20261103100000" channel="twee.nl"><title>Stop voor start</title></programme>
</tv>"#;

    #[test]
    fn times_with_zone_seconds_and_without() {
        assert_eq!(parse_xmltv_time("20261103101500 +0100"), Some(1_793_697_300));
        assert_eq!(parse_xmltv_time("20261103091500"), Some(1_793_697_300), "zonder zone geldt UTC");
        assert_eq!(parse_xmltv_time("202611030915 +0000"), Some(1_793_697_300), "seconden mogen ontbreken");
        assert_eq!(parse_xmltv_time("20261103041500 -0500"), Some(1_793_697_300), "negatieve zone");
        assert_eq!(parse_xmltv_time("20261103000000 +0000"), Some(1_793_664_000));
        for bad in ["", "kapot", "2026", "20261303101500", "20261103251500"] {
            assert_eq!(parse_xmltv_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_guide_is_parsed_with_entities_cdata_and_bad_programmes_dropped() {
        let g = parse_xmltv(XML.as_bytes(), None).unwrap();
        assert_eq!(g.channels.len(), 2);
        assert_eq!(g.channels[0].id, "een.nl@SD");
        assert_eq!(g.channels[0].names, vec!["Een HD", "Een"]);
        assert_eq!(g.channels[0].icon.as_deref(), Some("https://l.example/een.png"));
        assert_eq!(g.programmes.len(), 2, "de twee foute programma's vallen weg");
        let p = &g.programmes[0];
        assert_eq!(p.title, "Nieuws & Weer");
        assert_eq!(p.desc.as_deref(), Some("Het laatste <b>nieuws</b>"));
        assert_eq!((p.start, p.stop), (1_793_697_300, 1_793_700_000));
        assert_eq!(g.programmes[1].title, "Film é");
    }

    #[test]
    fn the_time_window_filters_programmes() {
        let g = parse_xmltv(XML.as_bytes(), Some((1_793_700_000, 1_793_707_200))).unwrap();
        assert_eq!(g.programmes.len(), 1);
        assert_eq!(g.programmes[0].title, "Film é");
    }

    #[test]
    fn gzip_is_unpacked_and_garbage_is_an_error() {
        use std::io::Write;
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(XML.as_bytes()).unwrap();
        let gz = enc.finish().unwrap();
        let g = parse_xmltv(&gunzip_if_needed(&gz).unwrap(), None).unwrap();
        assert_eq!(g.programmes.len(), 2);
        assert!(parse_xmltv(b"dit is geen xml", None).is_err());
        assert!(parse_xmltv(b"<tv></tv>", None).is_err(), "een lege gids is een fout");
        assert!(gunzip_if_needed(&[0x1f, 0x8b, 0, 1, 2, 3]).is_err(), "kapotte gzip");
    }

    #[test]
    fn a_truncated_guide_keeps_what_was_read() {
        let cut = &XML[..XML.find("<programme start=\"20261103110000 +0100\"").unwrap() + 10];
        let g = parse_xmltv(cut.as_bytes(), None).unwrap();
        assert_eq!(g.programmes.len(), 1);
    }

    #[test]
    fn normalizers_ignore_suffixes_case_and_quality_words() {
        assert_eq!(normalize_id("AB1.fr@SD"), "ab1.fr");
        assert_eq!(normalize_id(" een.nl "), "een.nl");
        assert_eq!(normalize_name("NPO 1 HD"), "npo1");
        assert_eq!(normalize_name("Één (FHD)"), "één");
        assert_eq!(normalize_name("HD"), "");
    }

    #[test]
    fn channels_are_matched_by_id_then_normalized_id_then_name() {
        let g = parse_xmltv(XML.as_bytes(), None).unwrap();
        let list = [
            ChannelRef { tvg_id: Some("een.nl@SD"), name: "iets anders" }, // exact
            ChannelRef { tvg_id: Some("EEN.NL"), name: "x" },              // genormaliseerd
            ChannelRef { tvg_id: None, name: "Twee HD" },                  // naam
            ChannelRef { tvg_id: Some("onbekend"), name: "Onbekend" },     // geen match
            ChannelRef { tvg_id: Some(""), name: "" },                     // leeg
        ];
        let m = match_channels(&list, &g.channels);
        assert_eq!(m, vec![Some("een.nl@SD".into()), Some("een.nl@SD".into()), Some("twee.nl".into()), None, None]);
    }
}
