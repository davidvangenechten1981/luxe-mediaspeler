//! Series: afleveringen uit de bibliotheek gegroepeerd per serie, met gezien-status en de volgende aflevering.

use crate::state::AppState;
use serde::Serialize;
use std::collections::BTreeMap;
use vu_core::db::{Media, MediaKind, MediaQuery};
use vu_core::error::Result;
use vu_core::series::parse_episode;

#[derive(Debug, Serialize)]
pub struct SeriesEpisode {
    pub media: Media,
    pub season: u32,
    pub episode: u32,
    pub watched: bool,
}

#[derive(Debug, Serialize)]
pub struct Series {
    pub show: String,
    pub episodes: Vec<SeriesEpisode>,
    pub watched: u32,
    /// Id van de eerste aflevering die je nog niet hebt gezien.
    pub next_media_id: Option<i64>,
    /// Ontbrekende afleveringen: per seizoen alles tussen aflevering 1 en de hoogste die je hebt.
    pub missing: Vec<MissingEpisode>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MissingEpisode {
    pub season: u32,
    pub episode: u32,
}

fn find_missing(episodes: &[SeriesEpisode]) -> Vec<MissingEpisode> {
    let mut by_season: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for e in episodes {
        by_season.entry(e.season).or_default().push(e.episode);
    }
    let mut out = vec![];
    for (season, eps) in by_season {
        let max = eps.iter().copied().max().unwrap_or(0).min(500);
        out.extend((1..=max).filter(|n| !eps.contains(n)).map(|episode| MissingEpisode { season, episode }));
    }
    out
}

pub fn list(st: &AppState) -> Result<Vec<Series>> {
    let db = st.lock_db()?;
    let done = db.completed_locations()?;
    let videos = db.query_media(&MediaQuery { kind: Some(MediaKind::Video), ..Default::default() })?;
    let mut shows: BTreeMap<String, (String, Vec<SeriesEpisode>)> = BTreeMap::new();
    for m in videos {
        let Some(ep) = parse_episode(&m.path) else { continue };
        let watched = done.contains(&m.path);
        let entry = shows.entry(ep.show.to_lowercase()).or_insert_with(|| (ep.show.clone(), Vec::new()));
        entry.1.push(SeriesEpisode { media: m, season: ep.season, episode: ep.episode, watched });
    }
    Ok(shows
        .into_values()
        .map(|(show, mut episodes)| {
            episodes.sort_by_key(|e| (e.season, e.episode));
            let watched = episodes.iter().filter(|e| e.watched).count() as u32;
            let next_media_id = episodes.iter().find(|e| !e.watched).map(|e| e.media.id);
            let missing = find_missing(&episodes);
            Series { show, episodes, watched, next_media_id, missing }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::testutil::app;
    use vu_core::db::NewMedia;

    fn video(path: &str) -> NewMedia {
        NewMedia {
            path: path.into(),
            kind: MediaKind::Video,
            title: None,
            artist: None,
            album: None,
            genre: None,
            year: None,
            track: None,
            duration_ms: 1_000,
            width: None,
            height: None,
            codec: None,
            container: None,
            bitrate: None,
            size: 1,
            mtime: 1,
        }
    }

    #[test]
    fn groups_episodes_per_show_in_order_and_finds_the_next_unwatched() {
        let app = app();
        {
            let mut db = app.state.lock_db().unwrap();
            for p in ["/s/Show.S01E02.mkv", "/s/Show.S01E01.mkv", "/s/Show.S02E01.mkv", "/s/Film.2020.mkv", "/s/Andere.S01E01.mkv"] {
                db.upsert_media(&video(p)).unwrap();
            }
            db.mark_completed("/s/Show.S01E01.mkv", true).unwrap();
        }
        let all = list(&app.state).unwrap();
        assert_eq!(all.iter().map(|s| s.show.as_str()).collect::<Vec<_>>(), ["Andere", "Show"]); // film telt niet mee
        let show = &all[1];
        let order: Vec<_> = show.episodes.iter().map(|e| (e.season, e.episode)).collect();
        assert_eq!(order, [(1, 1), (1, 2), (2, 1)]);
        assert_eq!(show.watched, 1);
        let next = show.episodes.iter().find(|e| e.media.id == show.next_media_id.unwrap()).unwrap();
        assert_eq!((next.season, next.episode), (1, 2));
    }

    #[test]
    fn reports_gaps_per_season_but_not_before_the_first_or_after_the_last_known_episode() {
        let app = app();
        {
            let mut db = app.state.lock_db().unwrap();
            for p in ["/s/Gat.S01E01.mkv", "/s/Gat.S01E04.mkv", "/s/Gat.S02E02.mkv", "/s/Vol.S01E01.mkv", "/s/Vol.S01E02.mkv"] {
                db.upsert_media(&video(p)).unwrap();
            }
        }
        let all = list(&app.state).unwrap();
        let gat = all.iter().find(|s| s.show == "Gat").unwrap();
        let gaps: Vec<_> = gat.missing.iter().map(|m| (m.season, m.episode)).collect();
        assert_eq!(gaps, [(1, 2), (1, 3), (2, 1)]);
        assert!(all.iter().find(|s| s.show == "Vol").unwrap().missing.is_empty());
    }

    #[test]
    fn a_fully_watched_show_has_no_next_episode() {
        let app = app();
        {
            let mut db = app.state.lock_db().unwrap();
            db.upsert_media(&video("/s/Klaar.S01E01.mkv")).unwrap();
            db.mark_completed("/s/Klaar.S01E01.mkv", true).unwrap();
        }
        assert_eq!(list(&app.state).unwrap()[0].next_media_id, None);
    }
}
