//! Database-onderdeel voor IPTV: bronnen, zenders, programmagids, opnames en herinneringen (schema 9).
//!
//! Geheimen: van een bron met een link (`m3u_url`) of Xtream-gegevens staat het geheim in de `SecretStore`
//! (Credential Manager); in de database staat alleen een gemaskeerde weergavetekst. De stream-URL's van zenders
//! staan wel in de database (de speler heeft ze nodig), maar gaan nooit naar de interface (`#[serde(skip)]`).

use super::Database;
use crate::error::{CoreError, Result};
use crate::iptv::epg::{normalize_name, Programme};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IptvSource {
    pub id: i64,
    pub name: String,
    /// `m3u_url`, `m3u_file` of `xtream`.
    pub kind: String,
    /// Weergavetekst: pad, servernaam of gemaskeerde link (nooit een geheim).
    pub location: String,
    pub epg_location: String,
    pub user_agent: Option<String>,
    pub last_refresh: Option<i64>,
    pub last_error: Option<String>,
    pub channel_count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IptvChannel {
    pub id: i64,
    pub source_id: i64,
    pub number: Option<u32>,
    pub name: String,
    /// Alleen voor de backend: kan inloggegevens bevatten.
    #[serde(skip)]
    pub url: String,
    /// `live`, `movie`, `series` (lijst van afleveringen, niet direct afspeelbaar) of `episode`.
    pub kind: String,
    pub group: Option<String>,
    pub logo: Option<String>,
    pub tvg_id: Option<String>,
    pub radio: bool,
    pub catchup_mode: Option<String>,
    #[serde(skip)]
    pub catchup_source: Option<String>,
    pub catchup_days: Option<u32>,
    #[serde(skip)]
    pub user_agent: Option<String>,
    #[serde(skip)]
    pub referrer: Option<String>,
    /// Gidszender die voor deze zender geldt (handmatige koppeling wint van de automatische).
    pub epg_id: Option<String>,
    pub epg_manual: bool,
    pub favorite: bool,
    pub hidden: bool,
    pub last_watched: Option<i64>,
}

/// Een zender zoals de bron hem aanlevert, vóór opslag.
#[derive(Debug, Clone, Default)]
pub struct NewChannel {
    pub name: String,
    pub url: String,
    pub kind: String,
    pub group: Option<String>,
    pub logo: Option<String>,
    pub tvg_id: Option<String>,
    pub number: Option<u32>,
    pub radio: bool,
    pub catchup_mode: Option<String>,
    pub catchup_source: Option<String>,
    pub catchup_days: Option<u32>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    /// Id bij de aanbieder (Xtream): maakt de sleutel stabiel ook als de naam verandert.
    pub remote_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct IptvChannelQuery {
    pub source_id: Option<i64>,
    pub kind: Option<String>,
    pub group: Option<String>,
    pub search: Option<String>,
    pub favorites_only: bool,
    pub include_hidden: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IptvGroup {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IptvProgramme {
    pub epg_id: String,
    pub start: i64,
    pub stop: i64,
    pub title: String,
    pub desc: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct NowNext {
    pub now: Option<IptvProgramme>,
    pub next: Option<IptvProgramme>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IptvRecording {
    pub id: i64,
    pub channel_name: String,
    #[serde(skip)]
    pub url: String,
    #[serde(skip)]
    pub user_agent: Option<String>,
    #[serde(skip)]
    pub referrer: Option<String>,
    pub title: String,
    pub start: i64,
    pub stop: i64,
    /// `planned`, `recording`, `done`, `failed` of `cancelled`.
    pub status: String,
    pub path: Option<String>,
    pub message: Option<String>,
}

/// Gegevens voor een nieuwe opname.
#[derive(Debug, Clone, Copy)]
pub struct NewRecording<'a> {
    pub channel_name: &'a str,
    pub url: &'a str,
    pub user_agent: Option<&'a str>,
    pub referrer: Option<&'a str>,
    pub title: &'a str,
    pub start: i64,
    pub stop: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IptvReminder {
    pub id: i64,
    pub channel_id: Option<i64>,
    pub channel_name: String,
    pub title: String,
    pub start: i64,
    pub notified: bool,
}

const CHANNEL_COLUMNS: &str =
    "id, source_id, number, name, url, kind, group_name, logo, tvg_id, radio, catchup_mode, catchup_source, catchup_days, \
     user_agent, referrer, COALESCE(epg_manual, epg_id), epg_manual IS NOT NULL, favorite, hidden, last_watched";

fn channel_from_row(r: &Row<'_>) -> rusqlite::Result<IptvChannel> {
    Ok(IptvChannel {
        id: r.get(0)?,
        source_id: r.get(1)?,
        number: r.get::<_, Option<i64>>(2)?.map(|n| n as u32),
        name: r.get(3)?,
        url: r.get(4)?,
        kind: r.get(5)?,
        group: r.get(6)?,
        logo: r.get(7)?,
        tvg_id: r.get(8)?,
        radio: r.get(9)?,
        catchup_mode: r.get(10)?,
        catchup_source: r.get(11)?,
        catchup_days: r.get::<_, Option<i64>>(12)?.map(|n| n as u32),
        user_agent: r.get(13)?,
        referrer: r.get(14)?,
        epg_id: r.get(15)?,
        epg_manual: r.get(16)?,
        favorite: r.get(17)?,
        hidden: r.get(18)?,
        last_watched: r.get(19)?,
    })
}

fn source_from_row(r: &Row<'_>) -> rusqlite::Result<IptvSource> {
    Ok(IptvSource {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        location: r.get(3)?,
        epg_location: r.get(4)?,
        user_agent: r.get(5)?,
        last_refresh: r.get(6)?,
        last_error: r.get(7)?,
        channel_count: r.get(8)?,
    })
}

const SOURCE_SELECT: &str = "SELECT s.id, s.name, s.kind, s.location, s.epg_location, s.user_agent, s.last_refresh, s.last_error, \
     (SELECT COUNT(*) FROM iptv_channels c WHERE c.source_id = s.id) FROM iptv_sources s";

fn programme_from_row(r: &Row<'_>) -> rusqlite::Result<IptvProgramme> {
    Ok(IptvProgramme { epg_id: r.get(0)?, start: r.get(1)?, stop: r.get(2)?, title: r.get(3)?, desc: r.get(4)? })
}

fn recording_from_row(r: &Row<'_>) -> rusqlite::Result<IptvRecording> {
    Ok(IptvRecording {
        id: r.get(0)?,
        channel_name: r.get(1)?,
        url: r.get(2)?,
        user_agent: r.get(3)?,
        referrer: r.get(4)?,
        title: r.get(5)?,
        start: r.get(6)?,
        stop: r.get(7)?,
        status: r.get(8)?,
        path: r.get(9)?,
        message: r.get(10)?,
    })
}

const RECORDING_COLUMNS: &str = "id, channel_name, url, user_agent, referrer, title, start, stop, status, path, message";

/// Stabiele sleutel per zender, zodat favorieten een verversing overleven. Dubbelen krijgen `#2`, `#3`, ...
fn assign_keys(channels: &[NewChannel]) -> Vec<String> {
    let mut seen: HashMap<String, u32> = HashMap::new();
    channels
        .iter()
        .map(|c| {
            let base = if let Some(r) = c.remote_id.as_deref().filter(|r| !r.is_empty()) {
                format!("x:{r}")
            } else if let Some(t) = c.tvg_id.as_deref().filter(|t| !t.trim().is_empty()) {
                format!("t:{}", t.trim().to_lowercase())
            } else {
                format!("n:{}", normalize_name(&c.name))
            };
            let n = seen.entry(base.clone()).or_insert(0);
            *n += 1;
            if *n == 1 {
                base
            } else {
                format!("{base}#{n}")
            }
        })
        .collect()
}

fn clean(s: &str, what: &str, max: usize) -> Result<String> {
    let t = s.trim();
    if t.is_empty() || t.chars().count() > max || t.chars().any(char::is_control) {
        return Err(CoreError::Invalid(format!("Ongeldige waarde voor {what}")));
    }
    Ok(t.to_string())
}

impl Database {
    // ---------- bronnen ----------

    pub fn add_iptv_source(&self, name: &str, kind: &str, location: &str, epg_location: &str, user_agent: Option<&str>) -> Result<i64> {
        let name = clean(name, "de naam", 80)?;
        if !matches!(kind, "m3u_url" | "m3u_file" | "xtream") {
            return Err(CoreError::Invalid("Onbekend brontype".into()));
        }
        self.conn.execute(
            "INSERT INTO iptv_sources (name, kind, location, epg_location, user_agent) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, kind, location, epg_location, user_agent.map(str::trim).filter(|u| !u.is_empty())],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_iptv_sources(&self) -> Result<Vec<IptvSource>> {
        let mut st = self.conn.prepare(&format!("{SOURCE_SELECT} ORDER BY s.id"))?;
        let rows = st.query_map([], source_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn iptv_source(&self, id: i64) -> Result<Option<IptvSource>> {
        Ok(self.conn.query_row(&format!("{SOURCE_SELECT} WHERE s.id = ?1"), [id], source_from_row).optional()?)
    }

    pub fn update_iptv_source(&self, id: i64, name: &str, epg_location: &str, user_agent: Option<&str>) -> Result<()> {
        let name = clean(name, "de naam", 80)?;
        let n = self.conn.execute(
            "UPDATE iptv_sources SET name = ?2, epg_location = ?3, user_agent = ?4 WHERE id = ?1",
            params![id, name, epg_location, user_agent.map(str::trim).filter(|u| !u.is_empty())],
        )?;
        if n == 0 {
            return Err(CoreError::NotFound("bron".into()));
        }
        Ok(())
    }

    pub fn set_iptv_source_status(&self, id: i64, refreshed_at: Option<i64>, error: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE iptv_sources SET last_refresh = COALESCE(?2, last_refresh), last_error = ?3 WHERE id = ?1",
            params![id, refreshed_at, error],
        )?;
        Ok(())
    }

    pub fn delete_iptv_source(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM iptv_sources WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- zenders ----------

    /// Vervangt alle zenders van een bron. Favoriet, verborgen, handmatige gidskoppeling en laatst bekeken
    /// blijven behouden voor zenders met dezelfde sleutel. Geeft het aantal opgeslagen zenders terug.
    pub fn replace_iptv_channels(&self, source_id: i64, channels: &[NewChannel]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let keep: HashMap<String, (bool, bool, Option<String>, Option<i64>)> = {
            let mut st = tx.prepare("SELECT key, favorite, hidden, epg_manual, last_watched FROM iptv_channels WHERE source_id = ?1")?;
            let rows = st.query_map([source_id], |r| Ok((r.get::<_, String>(0)?, (r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        tx.execute("DELETE FROM iptv_channels WHERE source_id = ?1", [source_id])?;
        let keys = assign_keys(channels);
        {
            let mut ins = tx.prepare(
                "INSERT INTO iptv_channels (source_id, key, position, number, name, url, kind, group_name, logo, tvg_id, radio, catchup_mode, \
                 catchup_source, catchup_days, user_agent, referrer, favorite, hidden, epg_manual, last_watched) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            )?;
            for (i, (c, key)) in channels.iter().zip(&keys).enumerate() {
                let (fav, hid, manual, watched) = keep.get(key).cloned().unwrap_or((false, false, None, None));
                let kind = if c.kind.is_empty() { "live" } else { c.kind.as_str() };
                ins.execute(params![
                    source_id,
                    key,
                    i as i64,
                    c.number,
                    c.name,
                    c.url,
                    kind,
                    c.group,
                    c.logo,
                    c.tvg_id,
                    c.radio,
                    c.catchup_mode,
                    c.catchup_source,
                    c.catchup_days,
                    c.user_agent,
                    c.referrer,
                    fav,
                    hid,
                    manual,
                    watched
                ])?;
            }
        }
        tx.commit()?;
        Ok(channels.len())
    }

    pub fn list_iptv_channels(&self, q: &IptvChannelQuery) -> Result<Vec<IptvChannel>> {
        let mut sql = format!("SELECT {CHANNEL_COLUMNS} FROM iptv_channels WHERE 1 = 1");
        let mut args: Vec<rusqlite::types::Value> = vec![];
        if let Some(s) = q.source_id {
            sql.push_str(" AND source_id = ?");
            args.push(s.into());
        }
        match q.kind.as_deref() {
            Some(k) if !k.is_empty() => {
                sql.push_str(" AND kind = ?");
                args.push(k.to_string().into());
            }
            _ => {}
        }
        if let Some(g) = q.group.as_deref().filter(|g| !g.is_empty()) {
            sql.push_str(" AND group_name = ?");
            args.push(g.to_string().into());
        }
        if let Some(s) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            sql.push_str(" AND (name LIKE ? ESCAPE '\\' OR group_name LIKE ? ESCAPE '\\' OR CAST(number AS TEXT) = ?)");
            let like = format!("%{}%", s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
            args.extend([like.clone().into(), like.into(), s.to_string().into()]);
        }
        if q.favorites_only {
            sql.push_str(" AND favorite = 1");
        }
        if !q.include_hidden {
            sql.push_str(" AND hidden = 0");
        }
        sql.push_str(" ORDER BY source_id, number IS NULL, number, position LIMIT ? OFFSET ?");
        args.push(i64::from(q.limit.unwrap_or(5000).min(20_000)).into());
        args.push(i64::from(q.offset.unwrap_or(0)).into());
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(rusqlite::params_from_iter(args), channel_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn iptv_channel(&self, id: i64) -> Result<Option<IptvChannel>> {
        Ok(self.conn.query_row(&format!("SELECT {CHANNEL_COLUMNS} FROM iptv_channels WHERE id = ?1"), [id], channel_from_row).optional()?)
    }

    /// Groepen met aantallen, voor de filterlijst.
    pub fn iptv_groups(&self, source_id: Option<i64>, kind: &str) -> Result<Vec<IptvGroup>> {
        let mut st = self.conn.prepare(
            "SELECT group_name, COUNT(*) FROM iptv_channels WHERE group_name IS NOT NULL AND hidden = 0 AND kind = ?2 AND (?1 IS NULL OR source_id = ?1) \
             GROUP BY group_name ORDER BY group_name COLLATE NOCASE",
        )?;
        let rows = st.query_map(params![source_id, kind], |r| Ok(IptvGroup { name: r.get(0)?, count: r.get(1)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn iptv_channel_flag(&self, id: i64, column: &str, value: bool) -> Result<()> {
        let n = self.conn.execute(&format!("UPDATE iptv_channels SET {column} = ?2 WHERE id = ?1"), params![id, value])?;
        if n == 0 {
            return Err(CoreError::NotFound("zender".into()));
        }
        Ok(())
    }

    pub fn set_iptv_favorite(&self, id: i64, value: bool) -> Result<()> {
        self.iptv_channel_flag(id, "favorite", value)
    }

    pub fn set_iptv_hidden(&self, id: i64, value: bool) -> Result<()> {
        self.iptv_channel_flag(id, "hidden", value)
    }

    pub fn touch_iptv_watched(&self, id: i64, at: i64) -> Result<()> {
        self.conn.execute("UPDATE iptv_channels SET last_watched = ?2 WHERE id = ?1", params![id, at])?;
        Ok(())
    }

    /// Handmatige gidskoppeling; `None` haalt hem weg.
    pub fn set_iptv_epg_manual(&self, id: i64, epg_id: Option<&str>) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE iptv_channels SET epg_manual = ?2 WHERE id = ?1",
            params![id, epg_id.map(str::trim).filter(|e| !e.is_empty())],
        )?;
        if n == 0 {
            return Err(CoreError::NotFound("zender".into()));
        }
        Ok(())
    }

    /// Zet de automatische gidskoppelingen (kanaal-id, gids-id) in één keer.
    pub fn set_iptv_epg_auto(&self, links: &[(i64, Option<String>)]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut st = tx.prepare("UPDATE iptv_channels SET epg_id = ?2 WHERE id = ?1")?;
            for (id, epg) in links {
                st.execute(params![id, epg])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// De laatst bekeken zenders, nieuwste eerst.
    pub fn recent_iptv_channels(&self, limit: u32) -> Result<Vec<IptvChannel>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {CHANNEL_COLUMNS} FROM iptv_channels WHERE last_watched IS NOT NULL AND hidden = 0 ORDER BY last_watched DESC LIMIT ?1"
        ))?;
        let rows = st.query_map([limit], channel_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---------- gids ----------

    /// Vervangt de hele gids. Gidszenders (voor handmatig koppelen) en programma's gaan in één transactie.
    pub fn replace_iptv_guide(&self, epg_channels: &[(String, String, Option<String>)], programmes: &[Programme]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM iptv_programmes", [])?;
        tx.execute("DELETE FROM iptv_epg_channels", [])?;
        {
            let mut ch = tx.prepare("INSERT OR REPLACE INTO iptv_epg_channels (epg_id, name, icon) VALUES (?1, ?2, ?3)")?;
            for (id, name, icon) in epg_channels {
                ch.execute(params![id, name, icon])?;
            }
            let mut pr =
                tx.prepare("INSERT OR REPLACE INTO iptv_programmes (epg_id, start, stop, title, desc) VALUES (?1, ?2, ?3, ?4, ?5)")?;
            for p in programmes {
                pr.execute(params![p.channel, p.start, p.stop, p.title, p.desc])?;
            }
        }
        tx.commit()?;
        Ok(programmes.len())
    }

    /// Gidszenders (id, naam) om handmatig aan te koppelen.
    pub fn iptv_epg_channels(&self) -> Result<Vec<(String, String)>> {
        let mut st = self.conn.prepare("SELECT epg_id, COALESCE(name, epg_id) FROM iptv_epg_channels ORDER BY 2 COLLATE NOCASE")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn iptv_programme_count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM iptv_programmes", [], |r| r.get(0))?)
    }

    /// "Nu" en "straks" voor een aantal gidszenders.
    pub fn iptv_now_next(&self, epg_ids: &[String], now: i64) -> Result<HashMap<String, NowNext>> {
        let mut out: HashMap<String, NowNext> = HashMap::new();
        let mut now_st = self.conn.prepare("SELECT epg_id, start, stop, title, desc FROM iptv_programmes WHERE epg_id = ?1 AND start <= ?2 AND stop > ?2 ORDER BY start DESC LIMIT 1")?;
        let mut next_st = self.conn.prepare(
            "SELECT epg_id, start, stop, title, desc FROM iptv_programmes WHERE epg_id = ?1 AND start >= ?2 ORDER BY start LIMIT 1",
        )?;
        for id in epg_ids {
            let current = now_st.query_row(params![id, now], programme_from_row).optional()?;
            let after = current.as_ref().map_or(now, |c| c.stop);
            let next = next_st.query_row(params![id, after], programme_from_row).optional()?;
            if current.is_some() || next.is_some() {
                out.insert(id.clone(), NowNext { now: current, next });
            }
        }
        Ok(out)
    }

    /// Alle programma's van één gidszender in een tijdvak (voor de tijdlijn).
    pub fn iptv_guide(&self, epg_id: &str, from: i64, to: i64) -> Result<Vec<IptvProgramme>> {
        let mut st = self.conn.prepare(
            "SELECT epg_id, start, stop, title, desc FROM iptv_programmes WHERE epg_id = ?1 AND stop > ?2 AND start < ?3 ORDER BY start",
        )?;
        let rows = st.query_map(params![epg_id, from, to], programme_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Programma's waarvan de titel de zoekterm bevat, vanaf nu.
    pub fn iptv_search_programmes(&self, term: &str, from: i64, limit: u32) -> Result<Vec<IptvProgramme>> {
        let like = format!("%{}%", term.trim().replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        let mut st = self.conn.prepare(
            "SELECT epg_id, start, stop, title, desc FROM iptv_programmes WHERE title LIKE ?1 ESCAPE '\\' AND stop > ?2 ORDER BY start LIMIT ?3",
        )?;
        let rows = st.query_map(params![like, from, limit.min(500)], programme_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---------- opnames ----------

    pub fn add_iptv_recording(&self, r: &NewRecording<'_>) -> Result<i64> {
        if r.stop <= r.start {
            return Err(CoreError::Invalid("Het einde moet na het begin liggen".into()));
        }
        let title = clean(r.title, "de titel", 200)?;
        self.conn.execute(
            "INSERT INTO iptv_recordings (channel_name, url, user_agent, referrer, title, start, stop) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![r.channel_name, r.url, r.user_agent, r.referrer, title, r.start, r.stop],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_iptv_recordings(&self) -> Result<Vec<IptvRecording>> {
        let mut st = self.conn.prepare(&format!("SELECT {RECORDING_COLUMNS} FROM iptv_recordings ORDER BY start DESC, id DESC"))?;
        let rows = st.query_map([], recording_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn iptv_recording(&self, id: i64) -> Result<Option<IptvRecording>> {
        Ok(self
            .conn
            .query_row(&format!("SELECT {RECORDING_COLUMNS} FROM iptv_recordings WHERE id = ?1"), [id], recording_from_row)
            .optional()?)
    }

    pub fn set_iptv_recording_status(&self, id: i64, status: &str, path: Option<&str>, message: Option<&str>) -> Result<()> {
        if !matches!(status, "planned" | "recording" | "done" | "failed" | "cancelled") {
            return Err(CoreError::Invalid("Onbekende opnamestatus".into()));
        }
        self.conn.execute(
            "UPDATE iptv_recordings SET status = ?2, path = COALESCE(?3, path), message = ?4 WHERE id = ?1",
            params![id, status, path, message],
        )?;
        Ok(())
    }

    pub fn delete_iptv_recording(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM iptv_recordings WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- herinneringen ----------

    pub fn add_iptv_reminder(&self, channel_id: Option<i64>, channel_name: &str, title: &str, start: i64) -> Result<i64> {
        let title = clean(title, "de titel", 200)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO iptv_reminders (channel_id, channel_name, title, start) VALUES (?1, ?2, ?3, ?4)",
            params![channel_id, channel_name, title, start],
        )?;
        Ok(self.conn.query_row(
            "SELECT id FROM iptv_reminders WHERE channel_name = ?1 AND start = ?2",
            params![channel_name, start],
            |r| r.get(0),
        )?)
    }

    fn reminder_from_row(r: &Row<'_>) -> rusqlite::Result<IptvReminder> {
        Ok(IptvReminder {
            id: r.get(0)?,
            channel_id: r.get(1)?,
            channel_name: r.get(2)?,
            title: r.get(3)?,
            start: r.get(4)?,
            notified: r.get(5)?,
        })
    }

    pub fn list_iptv_reminders(&self) -> Result<Vec<IptvReminder>> {
        let mut st = self.conn.prepare("SELECT id, channel_id, channel_name, title, start, notified FROM iptv_reminders ORDER BY start")?;
        let rows = st.query_map([], Self::reminder_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Herinneringen die nu gemeld moeten worden (begin binnen `lead_secs`, nog niet gemeld, niet al lang voorbij). Markeert ze als gemeld.
    pub fn take_due_iptv_reminders(&self, now: i64, lead_secs: i64) -> Result<Vec<IptvReminder>> {
        let tx = self.conn.unchecked_transaction()?;
        let due: Vec<IptvReminder> = {
            let mut st = tx.prepare(
                "SELECT id, channel_id, channel_name, title, start, notified FROM iptv_reminders WHERE notified = 0 AND start - ?2 <= ?1 AND start + 3600 > ?1 ORDER BY start",
            )?;
            let rows = st.query_map(params![now, lead_secs], Self::reminder_from_row)?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for d in &due {
            tx.execute("UPDATE iptv_reminders SET notified = 1 WHERE id = ?1", [d.id])?;
        }
        tx.commit()?;
        Ok(due)
    }

    pub fn delete_iptv_reminder(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM iptv_reminders WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(name: &str, group: Option<&str>, number: Option<u32>) -> NewChannel {
        NewChannel {
            name: name.into(),
            url: format!("http://prov.example/live/jan/pw/{}.ts", name.replace(' ', "")),
            kind: "live".into(),
            group: group.map(Into::into),
            number,
            ..Default::default()
        }
    }

    fn db_with_source() -> (Database, i64) {
        let db = Database::open_in_memory().unwrap();
        let id = db.add_iptv_source("Thuis", "m3u_file", "C:\\lijsten\\tv.m3u", "", None).unwrap();
        (db, id)
    }

    #[test]
    fn sources_are_added_listed_updated_and_deleted_with_their_channels() {
        let (db, id) = db_with_source();
        db.replace_iptv_channels(id, &[ch("Een", None, None), ch("Twee", None, None)]).unwrap();
        let s = db.list_iptv_sources().unwrap();
        assert_eq!((s.len(), s[0].name.as_str(), s[0].channel_count), (1, "Thuis", 2));
        db.update_iptv_source(id, "Woonkamer", "", Some(" Agent/1 ")).unwrap();
        db.set_iptv_source_status(id, Some(1000), Some("fout")).unwrap();
        let s = db.iptv_source(id).unwrap().unwrap();
        assert_eq!(
            (s.name.as_str(), s.user_agent.as_deref(), s.last_refresh, s.last_error.as_deref()),
            ("Woonkamer", Some("Agent/1"), Some(1000), Some("fout"))
        );
        db.set_iptv_source_status(id, None, None).unwrap();
        let s = db.iptv_source(id).unwrap().unwrap();
        assert_eq!((s.last_refresh, s.last_error), (Some(1000), None), "zonder nieuwe tijd blijft de oude staan");
        db.delete_iptv_source(id).unwrap();
        assert!(db.list_iptv_sources().unwrap().is_empty());
        assert!(db.list_iptv_channels(&IptvChannelQuery::default()).unwrap().is_empty(), "zenders verdwijnen mee");
        assert!(db.add_iptv_source("", "m3u_file", "x", "", None).is_err());
        assert!(db.add_iptv_source("x", "onbekend", "x", "", None).is_err());
    }

    #[test]
    fn a_refresh_keeps_favorites_hidden_and_manual_epg_by_key_even_with_duplicate_names() {
        let (db, id) = db_with_source();
        db.replace_iptv_channels(
            id,
            &[ch("Een HD", Some("Nieuws"), Some(1)), ch("Een HD", Some("Nieuws"), Some(2)), ch("Twee", None, None)],
        )
        .unwrap();
        let all = db.list_iptv_channels(&IptvChannelQuery::default()).unwrap();
        db.set_iptv_favorite(all[1].id, true).unwrap(); // de tweede "Een HD"
        db.set_iptv_hidden(all[2].id, true).unwrap();
        db.set_iptv_epg_manual(all[0].id, Some("een.nl")).unwrap();
        db.touch_iptv_watched(all[0].id, 500).unwrap();

        // verversen: andere volgorde, extra zender
        db.replace_iptv_channels(
            id,
            &[ch("Drie", None, None), ch("Een HD", Some("Nieuws"), Some(1)), ch("Een HD", Some("Nieuws"), Some(2)), ch("Twee", None, None)],
        )
        .unwrap();
        let after = db.list_iptv_channels(&IptvChannelQuery { include_hidden: true, ..Default::default() }).unwrap();
        let by = |n: &str, num: Option<u32>| after.iter().find(|c| c.name == n && c.number == num).unwrap();
        assert!(!by("Een HD", Some(1)).favorite && by("Een HD", Some(2)).favorite, "de juiste duplicaat blijft favoriet");
        assert!(by("Twee", None).hidden && !by("Drie", None).hidden);
        assert_eq!(
            (by("Een HD", Some(1)).epg_id.as_deref(), by("Een HD", Some(1)).epg_manual, by("Een HD", Some(1)).last_watched),
            (Some("een.nl"), true, Some(500))
        );
    }

    #[test]
    fn channels_are_filtered_searched_sorted_and_paged() {
        let (db, id) = db_with_source();
        let mut radio = ch("Klassiek FM", Some("Radio"), None);
        radio.radio = true;
        db.replace_iptv_channels(
            id,
            &[
                ch("Sport 1", Some("Sport"), Some(20)),
                ch("Nieuws 24", Some("Nieuws"), Some(10)),
                radio,
                ch("Film_100%", Some("Film"), None),
            ],
        )
        .unwrap();
        let names = |q: IptvChannelQuery| db.list_iptv_channels(&q).unwrap().into_iter().map(|c| c.name).collect::<Vec<_>>();
        assert_eq!(
            names(IptvChannelQuery::default()),
            vec!["Nieuws 24", "Sport 1", "Klassiek FM", "Film_100%"],
            "eerst op nummer, rest op volgorde"
        );
        assert_eq!(names(IptvChannelQuery { group: Some("Sport".into()), ..Default::default() }), vec!["Sport 1"]);
        assert_eq!(names(IptvChannelQuery { search: Some("nieuws".into()), ..Default::default() }), vec!["Nieuws 24"]);
        assert_eq!(names(IptvChannelQuery { search: Some("20".into()), ..Default::default() }), vec!["Sport 1"], "zoeken op kanaalnummer");
        assert_eq!(
            names(IptvChannelQuery { search: Some("_100%".into()), ..Default::default() }),
            vec!["Film_100%"],
            "LIKE-tekens zijn gewone tekens"
        );
        assert_eq!(names(IptvChannelQuery { limit: Some(2), offset: Some(1), ..Default::default() }), vec!["Sport 1", "Klassiek FM"]);
        let groups = db.iptv_groups(None, "live").unwrap();
        assert_eq!(groups.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), vec!["Film", "Nieuws", "Radio", "Sport"]);
    }

    #[test]
    fn urls_and_templates_never_serialize_to_the_interface() {
        let (db, id) = db_with_source();
        let mut c = ch("Een", None, None);
        c.catchup_source = Some("http://prov.example/geheim/${start}".into());
        c.user_agent = Some("Geheim/1".into());
        db.replace_iptv_channels(id, &[c]).unwrap();
        let got = db.list_iptv_channels(&IptvChannelQuery::default()).unwrap();
        assert!(got[0].url.contains("jan/pw"), "de backend heeft de URL wel");
        let json = serde_json::to_string(&got).unwrap();
        for secret in ["jan", "pw", "geheim", "Geheim"] {
            assert!(!json.contains(secret), "{secret} lekt naar de interface: {json}");
        }
    }

    #[test]
    fn guide_now_next_range_and_search_work() {
        let (db, _) = db_with_source();
        let p = |c: &str, s: i64, e: i64, t: &str| Programme { channel: c.into(), start: s, stop: e, title: t.into(), desc: None };
        let n = db
            .replace_iptv_guide(
                &[("een.nl".into(), "Een".into(), None)],
                &[
                    p("een.nl", 0, 100, "Voorbij"),
                    p("een.nl", 100, 200, "Nu bezig"),
                    p("een.nl", 200, 300, "Straks"),
                    p("een.nl", 500, 600, "Later 100%"),
                    p("twee.nl", 100, 200, "Elders"),
                ],
            )
            .unwrap();
        assert_eq!((n, db.iptv_programme_count().unwrap()), (5, 5));
        let nn = db.iptv_now_next(&["een.nl".into(), "onbekend".into(), "twee.nl".into()], 150).unwrap();
        assert_eq!(nn["een.nl"].now.as_ref().unwrap().title, "Nu bezig");
        assert_eq!(nn["een.nl"].next.as_ref().unwrap().title, "Straks");
        assert!(nn["twee.nl"].next.is_none() && !nn.contains_key("onbekend"));
        let gap = db.iptv_now_next(&["een.nl".into()], 350).unwrap();
        assert!(gap["een.nl"].now.is_none());
        assert_eq!(gap["een.nl"].next.as_ref().unwrap().title, "Later 100%", "in een gat: alleen 'straks'");
        assert_eq!(
            db.iptv_guide("een.nl", 150, 250).unwrap().iter().map(|p| p.title.as_str()).collect::<Vec<_>>(),
            vec!["Nu bezig", "Straks"]
        );
        assert_eq!(db.iptv_search_programmes("100%", 0, 10).unwrap().len(), 1);
        assert_eq!(db.iptv_search_programmes("nu", 0, 10).unwrap()[0].title, "Nu bezig");
        assert_eq!(db.iptv_epg_channels().unwrap(), vec![("een.nl".to_string(), "Een".to_string())]);
        // een nieuwe gids vervangt de oude volledig
        db.replace_iptv_guide(&[], &[p("een.nl", 0, 10, "Nieuw")]).unwrap();
        assert_eq!(db.iptv_programme_count().unwrap(), 1);
    }

    #[test]
    fn automatic_epg_links_are_set_but_a_manual_link_wins() {
        let (db, id) = db_with_source();
        db.replace_iptv_channels(id, &[ch("Een", None, None), ch("Twee", None, None)]).unwrap();
        let all = db.list_iptv_channels(&IptvChannelQuery::default()).unwrap();
        db.set_iptv_epg_auto(&[(all[0].id, Some("auto.een".into())), (all[1].id, None)]).unwrap();
        db.set_iptv_epg_manual(all[0].id, Some("hand.een")).unwrap();
        let c = db.iptv_channel(all[0].id).unwrap().unwrap();
        assert_eq!((c.epg_id.as_deref(), c.epg_manual), (Some("hand.een"), true));
        db.set_iptv_epg_manual(all[0].id, None).unwrap();
        assert_eq!(db.iptv_channel(all[0].id).unwrap().unwrap().epg_id.as_deref(), Some("auto.een"));
        assert!(db.set_iptv_epg_manual(9999, Some("x")).is_err());
        assert!(db.set_iptv_favorite(9999, true).is_err());
    }

    #[test]
    fn recordings_have_a_status_lifecycle_and_hide_their_url() {
        let (db, _) = db_with_source();
        assert!(db
            .add_iptv_recording(&NewRecording {
                channel_name: "Een",
                url: "http://p/live/jan/pw/1.ts",
                user_agent: None,
                referrer: None,
                title: "Titel",
                start: 200,
                stop: 100
            })
            .is_err());
        let id = db
            .add_iptv_recording(&NewRecording {
                channel_name: "Een",
                url: "http://p/live/jan/pw/1.ts",
                user_agent: Some("UA"),
                referrer: None,
                title: "Nieuws",
                start: 100,
                stop: 200,
            })
            .unwrap();
        db.set_iptv_recording_status(id, "recording", Some("D:\\opnames\\x.ts.part"), None).unwrap();
        db.set_iptv_recording_status(id, "done", None, None).unwrap();
        let r = db.iptv_recording(id).unwrap().unwrap();
        assert_eq!((r.status.as_str(), r.path.as_deref()), ("done", Some("D:\\opnames\\x.ts.part")), "het pad blijft staan");
        assert!(db.set_iptv_recording_status(id, "raar", None, None).is_err());
        assert!(!serde_json::to_string(&db.list_iptv_recordings().unwrap()).unwrap().contains("jan"));
        db.delete_iptv_recording(id).unwrap();
        assert!(db.list_iptv_recordings().unwrap().is_empty());
    }

    #[test]
    fn reminders_are_unique_due_once_and_expire() {
        let (db, _) = db_with_source();
        let a = db.add_iptv_reminder(None, "Een", "Nieuws", 1000).unwrap();
        assert_eq!(db.add_iptv_reminder(None, "Een", "Nieuws", 1000).unwrap(), a, "dubbel toevoegen geeft dezelfde herinnering");
        db.add_iptv_reminder(None, "Twee", "Film", 5000).unwrap();
        db.add_iptv_reminder(None, "Drie", "Oud", -5000).unwrap();
        assert!(db.take_due_iptv_reminders(900, 60).unwrap().is_empty(), "nog te vroeg");
        let due = db.take_due_iptv_reminders(950, 60).unwrap();
        assert_eq!(due.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), vec!["Nieuws"]);
        assert!(db.take_due_iptv_reminders(960, 60).unwrap().is_empty(), "maar één keer melden");
        assert_eq!(db.list_iptv_reminders().unwrap().len(), 3, "een veel te late herinnering wordt niet gemeld maar blijft staan");
        db.delete_iptv_reminder(a).unwrap();
        assert_eq!(db.list_iptv_reminders().unwrap().len(), 2);
    }
}
