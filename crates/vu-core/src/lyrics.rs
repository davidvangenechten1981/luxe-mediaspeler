//! Songteksten uit `.lrc`-bestanden (tijdgebonden) of gewone tekst.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    /// `None` bij tekst zonder tijden.
    pub time_ms: Option<u64>,
    pub text: String,
}

/// Leest LRC-tekst: regels als `[01:23.45] tekst`, ook met meerdere tijden per regel. Metatags (`[ar:..]`) worden overgeslagen.
/// Bevat de tekst geen tijden, dan komen de regels ongetimed terug. De uitvoer is gesorteerd op tijd.
pub fn parse_lrc(input: &str) -> Vec<LyricLine> {
    let input = input.trim_start_matches('\u{feff}');
    let mut timed = Vec::new();
    let mut plain = Vec::new();
    for raw in input.lines() {
        let line = raw.trim_end();
        let mut rest = line;
        let mut times = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(close) = stripped.find(']') else { break };
            match parse_stamp(&stripped[..close]) {
                Some(t) => times.push(t),
                None => {
                    if times.is_empty() {
                        rest = ""; // metatag zoals [ar:Artiest]
                    }
                    break;
                }
            }
            rest = &stripped[close + 1..];
        }
        if times.is_empty() {
            if !line.is_empty() && !line.starts_with('[') {
                plain.push(LyricLine { time_ms: None, text: line.trim().to_string() });
            }
            continue;
        }
        let text = rest.trim().to_string();
        timed.extend(times.into_iter().map(|t| LyricLine { time_ms: Some(t), text: text.clone() }));
    }
    if timed.is_empty() {
        return plain;
    }
    timed.sort_by_key(|l| l.time_ms);
    timed
}

fn parse_stamp(s: &str) -> Option<u64> {
    let (min, rest) = s.split_once(':')?;
    let (sec, frac) = rest.split_once(['.', ':']).unwrap_or((rest, ""));
    let min: u64 = min.parse().ok()?;
    let sec: u64 = sec.parse().ok()?;
    if sec >= 60 || !frac.chars().all(|c| c.is_ascii_digit()) || frac.len() > 3 {
        return None;
    }
    let ms = match frac.len() {
        0 => 0,
        1 => frac.parse::<u64>().ok()? * 100,
        2 => frac.parse::<u64>().ok()? * 10,
        _ => frac.parse::<u64>().ok()?,
    };
    Some((min * 60 + sec) * 1000 + ms)
}

/// Index van de regel die op `position_ms` getoond hoort te worden (laatste regel met tijd <= positie).
pub fn current_line(lines: &[LyricLine], position_ms: u64) -> Option<usize> {
    lines.iter().rposition(|l| l.time_ms.is_some_and(|t| t <= position_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timed_lines_and_skips_metadata() {
        let l = parse_lrc("\u{feff}[ar:Band]\n[ti:Lied]\n[00:12.50]Eerste\n[01:02.5]Tweede\n");
        assert_eq!(
            l,
            vec![LyricLine { time_ms: Some(12_500), text: "Eerste".into() }, LyricLine { time_ms: Some(62_500), text: "Tweede".into() },]
        );
    }

    #[test]
    fn repeated_timestamps_expand_and_sort() {
        let l = parse_lrc("[00:30.00][00:10.00]Refrein\n[00:20.00]Couplet");
        let times: Vec<_> = l.iter().map(|x| x.time_ms.unwrap()).collect();
        assert_eq!(times, [10_000, 20_000, 30_000]);
        assert_eq!(l[0].text, "Refrein");
    }

    #[test]
    fn plain_text_stays_untimed_and_garbage_stamps_are_rejected() {
        let l = parse_lrc("Regel een\n\nRegel twee");
        assert!(l.iter().all(|x| x.time_ms.is_none()));
        assert_eq!(l.len(), 2);
        assert!(parse_lrc("[99:99.99]x").iter().all(|x| x.time_ms.is_none()));
    }

    #[test]
    fn current_line_follows_the_position() {
        let l = parse_lrc("[00:05.00]a\n[00:10.00]b");
        assert_eq!(current_line(&l, 0), None);
        assert_eq!(current_line(&l, 5_000), Some(0));
        assert_eq!(current_line(&l, 9_999), Some(0));
        assert_eq!(current_line(&l, 60_000), Some(1));
    }
}
