//! Leest de belangrijkste velden uit een `.nfo`-bestand naast een film of aflevering (Kodi-stijl XML).
//! Bewust een kleine tekstparser: alleen enkelvoudige tags, geen externe verwijzingen.

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Nfo {
    pub title: Option<String>,
    pub year: Option<u32>,
    pub plot: Option<String>,
}

pub fn parse_nfo(xml: &str) -> Option<Nfo> {
    let xml = xml.trim_start_matches('\u{feff}');
    let nfo = Nfo { title: tag(xml, "title"), year: tag(xml, "year").and_then(|y| y.trim().parse().ok()), plot: tag(xml, "plot") };
    (nfo != Nfo::default()).then_some(nfo)
}

fn tag(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&format!("</{name}>"))? + start;
    let text = decode(xml[start..end].trim());
    (!text.is_empty()).then_some(text)
}

fn decode(s: &str) -> String {
    let s = s.strip_prefix("<![CDATA[").and_then(|r| r.strip_suffix("]]>")).unwrap_or(s);
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_title_year_and_plot() {
        let n =
            parse_nfo("<?xml version=\"1.0\"?><movie><title>De Film</title><year>1999</year><plot>Een &amp; twee.</plot></movie>").unwrap();
        assert_eq!(n, Nfo { title: Some("De Film".into()), year: Some(1999), plot: Some("Een & twee.".into()) });
    }

    #[test]
    fn handles_cdata_and_missing_fields() {
        let n = parse_nfo("<movie><title><![CDATA[Rock & Roll]]></title></movie>").unwrap();
        assert_eq!(n.title.as_deref(), Some("Rock & Roll"));
        assert_eq!((n.year, n.plot), (None, None));
    }

    #[test]
    fn unrelated_text_gives_nothing() {
        assert!(parse_nfo("gewoon een tekstje").is_none());
        assert!(parse_nfo("<movie><year>abc</year></movie>").is_none());
    }
}
