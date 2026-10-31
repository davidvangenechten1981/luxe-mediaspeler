//! Herkent serie-afleveringen aan de bestandsnaam (bijv. "Show.Name.S02E05.1080p.mkv" of "Show 2x05").
//! Puur tekstwerk: geen internet, geen bestanden lezen.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    pub show: String,
    pub season: u32,
    pub episode: u32,
}

/// Haalt serie, seizoen en aflevering uit een bestandsnaam of pad. `None` als het geen aflevering lijkt.
pub fn parse_episode(path: &str) -> Option<Episode> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    let chars: Vec<char> = stem.chars().collect();
    for i in 0..chars.len() {
        if let Some((season, episode, _end)) = marker_at(&chars, i) {
            let show = clean_show(&chars[..i].iter().collect::<String>());
            if show.is_empty() {
                return None;
            }
            return Some(Episode { show, season, episode });
        }
    }
    None
}

/// Probeert op positie `i` een "S01E02"- of "1x02"-markering te lezen.
fn marker_at(c: &[char], i: usize) -> Option<(u32, u32, usize)> {
    let boundary = i == 0 || !c[i - 1].is_alphanumeric();
    if !boundary {
        return None;
    }
    let (season, j) = if matches!(c[i], 'S' | 's') { number(c, i + 1)? } else { number(c, i)? };
    let sep = *c.get(j)?;
    if matches!(c[i], 'S' | 's') {
        if !matches!(sep, 'E' | 'e') {
            return None;
        }
    } else if !matches!(sep, 'x' | 'X') {
        return None;
    }
    let (episode, end) = number(c, j + 1)?;
    // In de korte "1x02"-vorm zijn maximaal twee cijfers normaal; "1x264" is een codecnaam.
    if !matches!(c[i], 'S' | 's') && end - (j + 1) > 2 {
        return None;
    }
    // Na de aflevering mag geen letter/cijfer meer komen ("1x264" is een codec, geen aflevering).
    if c.get(end).is_some_and(|ch| ch.is_alphanumeric()) {
        return None;
    }
    (episode > 0).then_some((season, episode, end))
}

fn number(c: &[char], start: usize) -> Option<(u32, usize)> {
    let digits: String = c[start.min(c.len())..].iter().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() > 3 {
        return None;
    }
    Some((digits.parse().ok()?, start + digits.len()))
}

fn clean_show(raw: &str) -> String {
    let spaced: String = raw.chars().map(|c| if matches!(c, '.' | '_') { ' ' } else { c }).collect();
    let trimmed = spaced.trim().trim_end_matches(['-', ' ']).trim();
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(show: &str, s: u32, e: u32) -> Option<Episode> {
        Some(Episode { show: show.into(), season: s, episode: e })
    }

    #[test]
    fn recognises_the_common_s01e02_form() {
        assert_eq!(parse_episode("Show.Name.S02E05.1080p.WEB.mkv"), ep("Show Name", 2, 5));
        assert_eq!(parse_episode("D:\\Series\\Mijn Serie - s01e10 - Titel.mp4"), ep("Mijn Serie", 1, 10));
    }

    #[test]
    fn recognises_the_1x02_form() {
        assert_eq!(parse_episode("Show Name 2x05.avi"), ep("Show Name", 2, 5));
    }

    #[test]
    fn ignores_codecs_resolutions_and_plain_movies() {
        assert_eq!(parse_episode("Film.2019.1080p.x264.mkv"), None);
        assert_eq!(parse_episode("Film 1x264.mkv"), None);
        assert_eq!(parse_episode("Vakantie.mp4"), None);
        assert_eq!(parse_episode("S01E02.mkv"), None); // zonder serienaam niet betrouwbaar
    }

    #[test]
    fn episode_zero_and_overlong_numbers_are_not_episodes() {
        assert_eq!(parse_episode("Show S01E00.mkv"), None);
        assert_eq!(parse_episode("Show S0001E01.mkv"), None);
    }
}
