//! IPTV: eigen bronnen (M3U-link, M3U-bestand, Xtream), zenderlijst, programmagids, afspelen, terugkijken,
//! herinneringen en opnemen. Inloggegevens staan alleen in de `SecretStore`; de interface krijgt nooit een stream-URL.

pub mod net;
#[cfg(test)]
mod tests;

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::Mutex;
use vu_core::db::{
    IptvChannel, IptvChannelQuery, IptvGroup, IptvProgramme, IptvRecording, IptvReminder, IptvSource, NewChannel, NewRecording, NowNext,
};
use vu_core::error::{CoreError, Result};
use vu_core::iptv::{catchup, epg, mask, playlist, record, xtream};
use vu_core::player::queue::QueueItem;
use vu_core::validate::{validate_path_shape, validate_stream_url};

const MAX_LIST_FILE_BYTES: u64 = 200 * 1024 * 1024;
const EPG_PAST_SECS: i64 = 86_400;
const EPG_FUTURE_SECS: i64 = 14 * 86_400;
/// Minimale vrije schijfruimte om een opname te starten.
const MIN_FREE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Zo ver van tevoren komt een herinnering.
const REMINDER_LEAD_SECS: i64 = 120;

fn now() -> i64 {
    vu_core::db::now()
}

// ---------- geheimen ----------

fn url_key(id: i64) -> String {
    format!("source-{id}-url")
}
fn epg_key(id: i64) -> String {
    format!("source-{id}-epg")
}
fn xtream_key(id: i64) -> String {
    format!("source-{id}-xtream")
}

#[derive(Serialize, Deserialize)]
struct XtreamSecret {
    server: String,
    username: String,
    password: String,
}

fn xtream_account(st: &AppState, id: i64) -> Result<xtream::Account> {
    let raw = st.secrets.get(&xtream_key(id))?.ok_or_else(|| CoreError::Invalid("De inloggegevens van deze bron ontbreken".into()))?;
    let s: XtreamSecret =
        serde_json::from_str(&raw).map_err(|_| CoreError::Invalid("De opgeslagen inloggegevens zijn onleesbaar".into()))?;
    xtream::Account::new(&s.server, &s.username, &s.password)
}

fn allow_local(st: &AppState) -> Result<bool> {
    Ok(st.settings()?.allow_local_streams)
}

fn http_url(st: &AppState, url: &str) -> Result<String> {
    let u = validate_stream_url(url.trim(), allow_local(st)?)?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err(CoreError::Invalid("Een link moet met http:// of https:// beginnen".into()));
    }
    Ok(u.to_string())
}

fn get(st: &AppState, url: &str, user_agent: Option<&str>) -> Result<Vec<u8>> {
    net::fetch(url, user_agent, allow_local(st)?)
}

// ---------- bronnen ----------

pub fn sources(st: &AppState) -> Result<Vec<IptvSource>> {
    st.lock_db()?.list_iptv_sources()
}

fn set_epg_link(st: &AppState, id: i64, epg_url: &str) -> Result<()> {
    let url = http_url(st, epg_url)?;
    st.secrets.set(&epg_key(id), &url)?;
    let db = st.lock_db()?;
    let src = db.iptv_source(id)?.ok_or_else(|| CoreError::NotFound("bron".into()))?;
    db.update_iptv_source(id, &src.name, &mask::mask_url(&url), src.user_agent.as_deref())
}

/// Voegt een bron toe; bij een fout achteraf wordt de halve bron weer weggehaald.
fn add_source(
    st: &AppState,
    name: &str,
    kind: &str,
    location: &str,
    user_agent: Option<&str>,
    secrets: &[(String, String)],
    epg_url: Option<&str>,
) -> Result<IptvSource> {
    let id = st.lock_db()?.add_iptv_source(name, kind, location, "", user_agent)?;
    let result = (|| -> Result<()> {
        for (k, v) in secrets {
            st.secrets.set(&k.replace("{id}", &id.to_string()), v)?;
        }
        if let Some(e) = epg_url.map(str::trim).filter(|e| !e.is_empty()) {
            set_epg_link(st, id, e)?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = delete_source(st, id);
        return Err(e);
    }
    st.lock_db()?.iptv_source(id)?.ok_or_else(|| CoreError::NotFound("bron".into()))
}

pub fn add_m3u_url(st: &AppState, name: &str, url: &str, epg_url: Option<&str>, user_agent: Option<&str>) -> Result<IptvSource> {
    let url = http_url(st, url)?;
    add_source(st, name, "m3u_url", &mask::mask_url(&url), user_agent, &[("source-{id}-url".to_string(), url)], epg_url)
}

pub fn add_m3u_file(st: &AppState, name: &str, path: &str, epg_url: Option<&str>) -> Result<IptvSource> {
    let p = validate_path_shape(path)?;
    if !p.is_file() {
        return Err(CoreError::NotFound(p.to_string_lossy().into_owned()));
    }
    add_source(st, name, "m3u_file", &p.to_string_lossy(), None, &[], epg_url)
}

pub fn add_xtream(st: &AppState, name: &str, server: &str, username: &str, password: &str) -> Result<IptvSource> {
    let server = http_url(st, server)?;
    let account = xtream::Account::new(&server, username, password)?;
    let secret = serde_json::to_string(&XtreamSecret {
        server: account.server().to_string(),
        username: username.trim().to_string(),
        password: password.to_string(),
    })
    .map_err(|e| CoreError::Invalid(e.to_string()))?;
    add_source(st, name, "xtream", account.server(), None, &[("source-{id}-xtream".to_string(), secret)], None)
}

/// `epg_url`: `None` laat de gidslink ongemoeid, een lege tekst wist hem, anders wordt hij ingesteld.
pub fn update_source(st: &AppState, id: i64, name: &str, epg_url: Option<&str>, user_agent: Option<&str>) -> Result<IptvSource> {
    {
        let db = st.lock_db()?;
        let src = db.iptv_source(id)?.ok_or_else(|| CoreError::NotFound("bron".into()))?;
        db.update_iptv_source(id, name, &src.epg_location, user_agent)?;
    }
    match epg_url.map(str::trim) {
        Some("") => {
            st.secrets.delete(&epg_key(id))?;
            let db = st.lock_db()?;
            let src = db.iptv_source(id)?.ok_or_else(|| CoreError::NotFound("bron".into()))?;
            db.update_iptv_source(id, &src.name, "", src.user_agent.as_deref())?;
        }
        Some(url) => set_epg_link(st, id, url)?,
        None => {}
    }
    st.lock_db()?.iptv_source(id)?.ok_or_else(|| CoreError::NotFound("bron".into()))
}

pub fn delete_source(st: &AppState, id: i64) -> Result<()> {
    for k in [url_key(id), epg_key(id), xtream_key(id)] {
        st.secrets.delete(&k)?;
    }
    st.lock_db()?.delete_iptv_source(id)
}

// ---------- verversen ----------

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RefreshReport {
    pub source_id: i64,
    pub channels: usize,
    pub skipped: usize,
    pub guide_link_found: bool,
}

struct Loaded {
    channels: Vec<NewChannel>,
    skipped: usize,
    epg_url: Option<String>,
}

fn guess_kind(url: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or("").to_ascii_lowercase();
    if [".mkv", ".mp4", ".avi", ".mov", ".wmv", ".m4v"].iter().any(|e| path.ends_with(e)) {
        "movie"
    } else {
        "live"
    }
}

fn from_playlist(c: playlist::Channel) -> NewChannel {
    NewChannel {
        kind: guess_kind(&c.url).into(),
        catchup_mode: c.catchup.as_ref().map(|x| x.mode.clone()),
        catchup_source: c.catchup.as_ref().and_then(|x| x.source.clone()),
        catchup_days: c.catchup.as_ref().and_then(|x| x.days),
        name: c.name,
        url: c.url,
        group: c.group,
        logo: c.logo,
        tvg_id: c.tvg_id,
        number: c.number,
        radio: c.radio,
        user_agent: c.user_agent,
        referrer: c.referrer,
        remote_id: None,
    }
}

fn text_of(bytes: &[u8]) -> String {
    playlist::decode_text(bytes)
}

fn load_channels(st: &AppState, src: &IptvSource) -> Result<Loaded> {
    match src.kind.as_str() {
        "m3u_file" => {
            let path = validate_path_shape(&src.location)?;
            let size = std::fs::metadata(&path)?.len();
            if size > MAX_LIST_FILE_BYTES {
                return Err(CoreError::Invalid("Het bestand is te groot voor een afspeellijst".into()));
            }
            let parsed = playlist::parse(&text_of(&std::fs::read(&path)?));
            Ok(Loaded {
                skipped: parsed.skipped,
                epg_url: parsed.epg_urls.first().cloned(),
                channels: parsed.channels.into_iter().map(from_playlist).collect(),
            })
        }
        "m3u_url" => {
            let url = st.secrets.get(&url_key(src.id))?.ok_or_else(|| CoreError::Invalid("De link van deze bron ontbreekt".into()))?;
            let parsed = playlist::parse(&text_of(&get(st, &url, src.user_agent.as_deref())?));
            Ok(Loaded {
                skipped: parsed.skipped,
                epg_url: parsed.epg_urls.first().cloned(),
                channels: parsed.channels.into_iter().map(from_playlist).collect(),
            })
        }
        "xtream" => load_xtream(st, src),
        other => Err(CoreError::Invalid(format!("Onbekend brontype: {other}"))),
    }
}

fn load_xtream(st: &AppState, src: &IptvSource) -> Result<Loaded> {
    let acct = xtream_account(st, src.id)?;
    let ua = src.user_agent.as_deref();
    let call = |action: Option<&str>| -> Result<String> { Ok(text_of(&get(st, &acct.api_url(action, &[]), ua)?)) };
    let info = xtream::parse_account_info(&call(None)?)?;
    if !info.status.eq_ignore_ascii_case("active") {
        return Err(CoreError::Invalid(format!("Het account is niet actief ({})", info.status)));
    }
    let mut channels = vec![];
    // live
    let cats = xtream::parse_categories(&call(Some("get_live_categories"))?).unwrap_or_default();
    for s in xtream::parse_live_streams(&call(Some("get_live_streams"))?)? {
        channels.push(NewChannel {
            kind: "live".into(),
            url: acct.live_url(&s.stream_id, "ts"),
            group: s.category_id.as_ref().and_then(|c| cats.get(c)).cloned(),
            logo: s.logo,
            tvg_id: s.epg_channel_id,
            number: s.number,
            catchup_mode: (s.archive_days > 0).then(|| "xc".to_string()),
            catchup_days: (s.archive_days > 0).then_some(s.archive_days),
            remote_id: Some(s.stream_id),
            name: s.name,
            ..Default::default()
        });
    }
    // films en series zijn optioneel: sommige accounts hebben ze niet
    if let Ok(vod_cats) = call(Some("get_vod_categories")).and_then(|j| xtream::parse_categories(&j)) {
        if let Ok(vods) = call(Some("get_vod_streams")).and_then(|j| xtream::parse_vod_streams(&j)) {
            for v in vods {
                channels.push(NewChannel {
                    kind: "movie".into(),
                    url: acct.movie_url(&v.stream_id, &v.extension),
                    group: v.category_id.as_ref().and_then(|c| vod_cats.get(c)).cloned(),
                    logo: v.poster,
                    remote_id: Some(format!("movie:{}", v.stream_id)),
                    name: v.name,
                    ..Default::default()
                });
            }
        }
    }
    if let Ok(series_cats) = call(Some("get_series_categories")).and_then(|j| xtream::parse_categories(&j)) {
        if let Ok(list) = call(Some("get_series")).and_then(|j| xtream::parse_series_list(&j)) {
            for s in list {
                channels.push(NewChannel {
                    kind: "series".into(),
                    url: format!("series:{}", s.series_id),
                    group: s.category_id.as_ref().and_then(|c| series_cats.get(c)).cloned(),
                    logo: s.cover,
                    remote_id: Some(format!("series:{}", s.series_id)),
                    name: s.name,
                    ..Default::default()
                });
            }
        }
    }
    Ok(Loaded { channels, skipped: 0, epg_url: Some(acct.xmltv_url()) })
}

pub fn refresh_source(st: &AppState, id: i64) -> Result<RefreshReport> {
    let src = st.lock_db()?.iptv_source(id)?.ok_or_else(|| CoreError::NotFound("bron".into()))?;
    match load_channels(st, &src) {
        Ok(loaded) => {
            let guide_link_found = loaded.epg_url.is_some();
            let n = st.lock_db()?.replace_iptv_channels(id, &loaded.channels)?;
            // een gidslink uit de lijst zelf wordt alleen gebruikt als er nog geen is ingesteld
            if let (Some(link), true) = (loaded.epg_url.as_deref().filter(|_| src.kind != "xtream"), src.epg_location.is_empty()) {
                let _ = set_epg_link(st, id, link);
            }
            st.lock_db()?.set_iptv_source_status(id, Some(now()), None)?;
            let _ = link_epg(st);
            Ok(RefreshReport { source_id: id, channels: n, skipped: loaded.skipped, guide_link_found })
        }
        Err(e) => {
            let msg = mask::mask_text(&e.to_string());
            st.lock_db()?.set_iptv_source_status(id, None, Some(&msg))?;
            Err(CoreError::Invalid(msg))
        }
    }
}

// ---------- gids ----------

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct GuideReport {
    pub programmes: usize,
    pub matched: usize,
    pub errors: Vec<String>,
}

fn epg_url_of(st: &AppState, src: &IptvSource) -> Result<Option<String>> {
    if src.kind == "xtream" {
        return Ok(Some(xtream_account(st, src.id)?.xmltv_url()));
    }
    st.secrets.get(&epg_key(src.id))
}

pub fn refresh_guide(st: &AppState) -> Result<GuideReport> {
    let sources = sources(st)?;
    let (mut channels, mut programmes, mut errors): (Vec<epg::EpgChannel>, Vec<epg::Programme>, Vec<String>) = (vec![], vec![], vec![]);
    let mut seen = HashSet::new();
    let mut tried = 0;
    let t = now();
    for src in &sources {
        let Some(url) = epg_url_of(st, src).unwrap_or(None) else { continue };
        if !seen.insert(url.clone()) {
            continue;
        }
        tried += 1;
        let result = get(st, &url, src.user_agent.as_deref())
            .and_then(|b| epg::gunzip_if_needed(&b))
            .and_then(|b| epg::parse_xmltv(&b, Some((t - EPG_PAST_SECS, t + EPG_FUTURE_SECS))));
        match result {
            Ok(g) => {
                channels.extend(g.channels);
                programmes.extend(g.programmes);
            }
            Err(e) => errors.push(format!("{}: {}", src.name, mask::mask_text(&e.to_string()))),
        }
    }
    if tried == 0 {
        return Err(CoreError::Invalid("Er is nog geen programmagids ingesteld. Voeg een gidslink toe bij een bron.".into()));
    }
    if programmes.is_empty() && channels.is_empty() {
        return Err(CoreError::Invalid(errors.join("; ")));
    }
    let mut ids = HashSet::new();
    let rows: Vec<(String, String, Option<String>)> =
        channels.iter().filter(|c| ids.insert(c.id.clone())).map(|c| (c.id.clone(), c.names.join(" | "), c.icon.clone())).collect();
    let n = st.lock_db()?.replace_iptv_guide(&rows, &programmes)?;
    let matched = link_epg(st)?;
    Ok(GuideReport { programmes: n, matched, errors })
}

/// Koppelt gidszenders automatisch aan zenders; geeft het aantal gekoppelde zenders terug.
pub fn link_epg(st: &AppState) -> Result<usize> {
    let db = st.lock_db()?;
    let epg_channels: Vec<epg::EpgChannel> = db
        .iptv_epg_channels()?
        .into_iter()
        .map(|(id, names)| epg::EpgChannel { names: names.split(" | ").map(str::to_string).collect(), id, icon: None })
        .collect();
    if epg_channels.is_empty() {
        return Ok(0);
    }
    let channels = db.list_iptv_channels(&IptvChannelQuery {
        kind: Some("live".into()),
        include_hidden: true,
        limit: Some(20_000),
        ..Default::default()
    })?;
    let refs: Vec<epg::ChannelRef<'_>> = channels.iter().map(|c| epg::ChannelRef { tvg_id: c.tvg_id.as_deref(), name: &c.name }).collect();
    let found = epg::match_channels(&refs, &epg_channels);
    let links: Vec<(i64, Option<String>)> = channels.iter().map(|c| c.id).zip(found).collect();
    let matched = links.iter().filter(|(_, e)| e.is_some()).count();
    db.set_iptv_epg_auto(&links)?;
    Ok(matched)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ChannelNowNext {
    pub channel_id: i64,
    #[serde(flatten)]
    pub programmes: NowNext,
}

pub fn now_next(st: &AppState, ids: &[i64]) -> Result<Vec<ChannelNowNext>> {
    let db = st.lock_db()?;
    let mut pairs: Vec<(i64, String)> = vec![];
    for id in ids.iter().take(1000) {
        if let Some(e) = db.iptv_channel(*id)?.and_then(|c| c.epg_id) {
            pairs.push((*id, e));
        }
    }
    let epg_ids: Vec<String> = pairs.iter().map(|(_, e)| e.clone()).collect::<HashSet<_>>().into_iter().collect();
    let found = db.iptv_now_next(&epg_ids, now())?;
    Ok(pairs
        .into_iter()
        .filter_map(|(channel_id, e)| found.get(&e).map(|n| ChannelNowNext { channel_id, programmes: n.clone() }))
        .collect())
}

pub fn guide(st: &AppState, channel_id: i64, from: i64, to: i64) -> Result<Vec<IptvProgramme>> {
    let db = st.lock_db()?;
    let ch = db.iptv_channel(channel_id)?.ok_or_else(|| CoreError::NotFound("zender".into()))?;
    match ch.epg_id {
        Some(e) => db.iptv_guide(&e, from, to.min(from + 7 * 86_400)),
        None => Ok(vec![]),
    }
}

// ---------- zenders ----------

pub fn channels(st: &AppState, q: &IptvChannelQuery) -> Result<Vec<IptvChannel>> {
    st.lock_db()?.list_iptv_channels(q)
}

pub fn groups(st: &AppState, source_id: Option<i64>, kind: &str) -> Result<Vec<IptvGroup>> {
    st.lock_db()?.iptv_groups(source_id, kind)
}

pub fn recent(st: &AppState) -> Result<Vec<IptvChannel>> {
    st.lock_db()?.recent_iptv_channels(20)
}

// ---------- afspelen ----------

fn header_options(user_agent: Option<&str>, referrer: Option<&str>) -> Vec<String> {
    let ok = |s: &&str| !s.trim().is_empty() && !s.chars().any(char::is_control);
    let mut out = vec![];
    if let Some(ua) = user_agent.filter(ok) {
        out.push(format!(":http-user-agent={}", ua.trim()));
    }
    if let Some(r) = referrer.filter(ok) {
        out.push(format!(":http-referrer={}", r.trim()));
    }
    out
}

fn start_stream(st: &AppState, token: String, title: String, url: &str, user_agent: Option<&str>, referrer: Option<&str>) -> Result<()> {
    validate_stream_url(url, allow_local(st)?)?;
    let player = st.require_player()?;
    let item =
        QueueItem { location: token, title: Some(title), stream: Some(url.to_string()), options: header_options(user_agent, referrer) };
    player.with(|s| s.play_items(vec![item], 0))??;
    Ok(())
}

fn channel_or_err(st: &AppState, id: i64) -> Result<(IptvChannel, Option<String>)> {
    let db = st.lock_db()?;
    let ch = db.iptv_channel(id)?.ok_or_else(|| CoreError::NotFound("zender".into()))?;
    let source_ua = db.iptv_source(ch.source_id)?.and_then(|s| s.user_agent);
    Ok((ch, source_ua))
}

pub fn play_channel(st: &AppState, id: i64) -> Result<IptvChannel> {
    let (ch, source_ua) = channel_or_err(st, id)?;
    if ch.kind == "series" {
        return Err(CoreError::Invalid("Kies eerst een aflevering van deze serie".into()));
    }
    let ua = ch.user_agent.clone().or(source_ua);
    start_stream(st, format!("iptv://channel/{id}"), ch.name.clone(), &ch.url, ua.as_deref(), ch.referrer.as_deref())?;
    st.lock_db()?.touch_iptv_watched(id, now())?;
    Ok(ch)
}

/// Naar de volgende (`step` = 1) of vorige (`-1`) livezender; aan het eind begint de lijst opnieuw.
pub fn zap(st: &AppState, from: i64, step: i32, favorites_only: bool) -> Result<IptvChannel> {
    let list = st.lock_db()?.list_iptv_channels(&IptvChannelQuery {
        kind: Some("live".into()),
        favorites_only,
        limit: Some(20_000),
        ..Default::default()
    })?;
    if list.is_empty() {
        return Err(CoreError::Invalid("Er zijn geen zenders om naartoe te zappen".into()));
    }
    let len = list.len() as i64;
    let next = match list.iter().position(|c| c.id == from) {
        Some(i) => (i as i64 + i64::from(step)).rem_euclid(len),
        None if step < 0 => len - 1,
        None => 0,
    } as usize;
    play_channel(st, list[next].id)
}

/// Speelt de eerste zichtbare livezender met dit kanaalnummer.
pub fn play_number(st: &AppState, number: u32) -> Result<IptvChannel> {
    let list =
        st.lock_db()?.list_iptv_channels(&IptvChannelQuery { kind: Some("live".into()), limit: Some(20_000), ..Default::default() })?;
    let ch = list.iter().find(|c| c.number == Some(number)).ok_or_else(|| CoreError::NotFound(format!("zender {number}")))?;
    play_channel(st, ch.id)
}

/// Terugkijken van een programma (start en einde in UTC-seconden).
pub fn play_catchup(st: &AppState, id: i64, start: i64, stop: i64) -> Result<IptvChannel> {
    let (ch, source_ua) = channel_or_err(st, id)?;
    let source = st.lock_db()?.iptv_source(ch.source_id)?;
    let mode = ch
        .catchup_mode
        .clone()
        .or_else(|| source.as_ref().filter(|s| s.kind == "xtream" && ch.catchup_days.unwrap_or(0) > 0).map(|_| "xc".to_string()))
        .ok_or_else(|| CoreError::Invalid("Deze zender biedt geen terugkijken".into()))?;
    let t = now();
    if let Some(days) = ch.catchup_days.filter(|d| *d > 0) {
        if start < t - i64::from(days) * 86_400 {
            return Err(CoreError::Invalid(format!("Dit programma ligt buiten het archief van {days} dagen")));
        }
    }
    let url = catchup::build_url(&catchup::CatchupRequest {
        stream_url: &ch.url,
        mode: &mode,
        source: ch.catchup_source.as_deref(),
        start,
        duration_secs: stop - start,
        now: t,
    })?;
    let ua = ch.user_agent.clone().or(source_ua);
    start_stream(st, format!("iptv://catchup/{id}/{start}"), ch.name.clone(), &url, ua.as_deref(), ch.referrer.as_deref())?;
    Ok(ch)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct EpisodeInfo {
    pub id: String,
    pub season: u32,
    pub episode: u32,
    pub title: String,
    pub extension: String,
}

fn series_id_of(ch: &IptvChannel) -> Result<&str> {
    if ch.kind != "series" {
        return Err(CoreError::Invalid("Dit is geen serie".into()));
    }
    ch.url.strip_prefix("series:").ok_or_else(|| CoreError::Invalid("Onbekende serie".into()))
}

pub fn series_episodes(st: &AppState, channel_id: i64) -> Result<Vec<EpisodeInfo>> {
    let (ch, ua) = channel_or_err(st, channel_id)?;
    let series_id = series_id_of(&ch)?.to_string();
    let acct = xtream_account(st, ch.source_id)?;
    let json = text_of(&get(st, &acct.api_url(Some("get_series_info"), &[("series_id", &series_id)]), ua.as_deref())?);
    Ok(xtream::parse_series_info(&json)?
        .into_iter()
        .map(|e| EpisodeInfo { id: e.id, season: e.season, episode: e.episode, title: e.title, extension: e.extension })
        .collect())
}

pub fn play_episode(st: &AppState, channel_id: i64, episode_id: &str, extension: &str) -> Result<()> {
    let (ch, ua) = channel_or_err(st, channel_id)?;
    series_id_of(&ch)?;
    let ext_ok = !extension.is_empty() && extension.len() <= 5 && extension.chars().all(|c| c.is_ascii_alphanumeric());
    if !ext_ok || episode_id.is_empty() || !episode_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(CoreError::Invalid("Ongeldige aflevering".into()));
    }
    let url = xtream_account(st, ch.source_id)?.series_url(episode_id, extension);
    start_stream(st, format!("iptv://episode/{channel_id}/{episode_id}"), ch.name, &url, ua.as_deref(), None)
}

// ---------- herinneringen ----------

pub fn reminder_add(st: &AppState, channel_id: i64, title: &str, start: i64) -> Result<IptvReminder> {
    let db = st.lock_db()?;
    let ch = db.iptv_channel(channel_id)?.ok_or_else(|| CoreError::NotFound("zender".into()))?;
    let id = db.add_iptv_reminder(Some(channel_id), &ch.name, title, start)?;
    db.list_iptv_reminders()?.into_iter().find(|r| r.id == id).ok_or_else(|| CoreError::NotFound("herinnering".into()))
}

pub fn reminders(st: &AppState) -> Result<Vec<IptvReminder>> {
    st.lock_db()?.list_iptv_reminders()
}

pub fn reminder_delete(st: &AppState, id: i64) -> Result<()> {
    st.lock_db()?.delete_iptv_reminder(id)
}

// ---------- opnemen ----------

struct Recorder {
    child: Child,
    part: PathBuf,
    target: PathBuf,
}

/// Lopende opnames (proces per opname).
#[derive(Default)]
pub struct Runtime {
    recorders: Mutex<HashMap<i64, Recorder>>,
}

pub fn recordings(st: &AppState) -> Result<Vec<IptvRecording>> {
    st.lock_db()?.list_iptv_recordings()
}

pub fn recording_add(st: &AppState, channel_id: i64, title: &str, start: i64, stop: i64) -> Result<IptvRecording> {
    let (ch, source_ua) = channel_or_err(st, channel_id)?;
    if ch.kind != "live" {
        return Err(CoreError::Invalid("Alleen livezenders kunnen worden opgenomen".into()));
    }
    if stop <= now() {
        return Err(CoreError::Invalid("Het einde van deze opname ligt in het verleden".into()));
    }
    let ua = ch.user_agent.clone().or(source_ua);
    let db = st.lock_db()?;
    let id = db.add_iptv_recording(&NewRecording {
        channel_name: &ch.name,
        url: &ch.url,
        user_agent: ua.as_deref(),
        referrer: ch.referrer.as_deref(),
        title,
        start,
        stop,
    })?;
    db.iptv_recording(id)?.ok_or_else(|| CoreError::NotFound("opname".into()))
}

/// Begint nu met opnemen voor `minutes` minuten.
pub fn record_now(st: &AppState, channel_id: i64, minutes: u32) -> Result<IptvRecording> {
    let minutes = i64::from(minutes.clamp(1, 12 * 60));
    let t = now();
    let title =
        now_next(st, &[channel_id])?.into_iter().next().and_then(|n| n.programmes.now).map_or_else(|| "Opname".to_string(), |p| p.title);
    let rec = recording_add(st, channel_id, &title, t, t + minutes * 60)?;
    tick(st)?;
    st.lock_db()?.iptv_recording(rec.id)?.ok_or_else(|| CoreError::NotFound("opname".into()))
}

fn finalize(part: &Path, target: &Path) -> bool {
    match std::fs::metadata(part) {
        Ok(m) if m.len() > 0 => std::fs::rename(part, target).is_ok(),
        _ => false,
    }
}

fn start_recording(st: &AppState, rec: &IptvRecording) -> Result<()> {
    let dir = st.settings()?.iptv_record_dir.ok_or_else(|| CoreError::Invalid("Kies eerst een map voor opnames (Instellingen)".into()))?;
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir)?;
    if record::free_bytes(&dir).is_some_and(|free| free < MIN_FREE_BYTES) {
        return Err(CoreError::Invalid("Er is te weinig vrije schijfruimte voor een opname (minder dan 2 GB)".into()));
    }
    let ffmpeg = st.require_ffmpeg()?.to_path_buf();
    let target = record::unique_path(&dir, &record::file_name(&rec.channel_name, &rec.title, rec.start));
    let part = PathBuf::from(format!("{}.part", target.display()));
    let remaining = (rec.stop - now()).max(1) as u64;
    let args = record::build_args(&rec.url, rec.user_agent.as_deref(), rec.referrer.as_deref(), &part, remaining);
    let child = vu_core::proc::background(ffmpeg)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::Ffmpeg(format!("De opname kon niet starten: {e}")))?;
    st.iptv
        .recorders
        .lock()
        .map_err(|_| CoreError::Invalid("Opnames zijn vergrendeld".into()))?
        .insert(rec.id, Recorder { child, part, target: target.clone() });
    st.lock_db()?.set_iptv_recording_status(rec.id, "recording", Some(&target.to_string_lossy()), None)
}

/// Rondt een gestopt proces af en zet de status. Geeft `true` als er iets veranderd is.
fn reap(st: &AppState) -> Result<bool> {
    let mut finished: Vec<(i64, Recorder, std::process::ExitStatus)> = vec![];
    {
        let mut map = st.iptv.recorders.lock().map_err(|_| CoreError::Invalid("Opnames zijn vergrendeld".into()))?;
        let ids: Vec<i64> = map.keys().copied().collect();
        for id in ids {
            if let Some(status) = map.get_mut(&id).and_then(|r| r.child.try_wait().ok().flatten()) {
                if let Some(r) = map.remove(&id) {
                    finished.push((id, r, status));
                }
            }
        }
    }
    let changed = !finished.is_empty();
    for (id, mut r, status) in finished {
        let mut stderr = String::new();
        if let Some(mut e) = r.child.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut e, &mut stderr);
        }
        let detail: String = mask::mask_text(stderr.trim()).chars().rev().take(300).collect::<Vec<_>>().into_iter().rev().collect();
        let kept = finalize(&r.part, &r.target);
        let db = st.lock_db()?;
        match (status.success(), kept) {
            (true, true) => db.set_iptv_recording_status(id, "done", Some(&r.target.to_string_lossy()), None)?,
            (false, true) => db.set_iptv_recording_status(
                id,
                "done",
                Some(&r.target.to_string_lossy()),
                Some(&format!("Onderbroken, het opgenomen deel is bewaard. {detail}")),
            )?,
            _ => {
                db.set_iptv_recording_status(id, "failed", None, Some(if detail.is_empty() { "Er is niets opgenomen" } else { &detail }))?
            }
        }
    }
    Ok(changed)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TickResult {
    pub reminders: Vec<IptvReminder>,
    pub recordings_changed: bool,
}

/// Wordt door de interface regelmatig (ongeveer elke 30 s) aangeroepen: start geplande opnames, rondt klare af
/// en geeft herinneringen die nu gemeld moeten worden.
pub fn tick(st: &AppState) -> Result<TickResult> {
    let mut changed = reap(st)?;
    let t = now();
    let all = st.lock_db()?.list_iptv_recordings()?;
    for rec in all.iter().filter(|r| r.status == "planned") {
        if rec.stop <= t {
            st.lock_db()?.set_iptv_recording_status(rec.id, "failed", None, Some("Gemist: de app was niet actief op de geplande tijd"))?;
            changed = true;
        } else if rec.start <= t + 2 {
            changed = true;
            if let Err(e) = start_recording(st, rec) {
                st.lock_db()?.set_iptv_recording_status(rec.id, "failed", None, Some(&mask::mask_text(&e.to_string())))?;
            }
        }
    }
    let reminders = st.lock_db()?.take_due_iptv_reminders(t, REMINDER_LEAD_SECS)?;
    Ok(TickResult { reminders, recordings_changed: changed })
}

pub fn recording_cancel(st: &AppState, id: i64) -> Result<()> {
    let running = st.iptv.recorders.lock().map_err(|_| CoreError::Invalid("Opnames zijn vergrendeld".into()))?.remove(&id);
    let db = st.lock_db()?;
    match running {
        Some(mut r) => {
            let _ = r.child.kill();
            let _ = r.child.wait();
            let kept = finalize(&r.part, &r.target);
            db.set_iptv_recording_status(
                id,
                "cancelled",
                kept.then(|| r.target.to_string_lossy().into_owned()).as_deref(),
                Some(if kept { "Gestopt, het opgenomen deel is bewaard" } else { "Gestopt" }),
            )
        }
        None => db.set_iptv_recording_status(id, "cancelled", None, None),
    }
}

/// Haalt een opname uit de lijst. Het opgenomen bestand blijft staan.
pub fn recording_delete(st: &AppState, id: i64) -> Result<()> {
    if st.iptv.recorders.lock().map_err(|_| CoreError::Invalid("Opnames zijn vergrendeld".into()))?.contains_key(&id) {
        return Err(CoreError::Invalid("Stop de opname eerst".into()));
    }
    st.lock_db()?.delete_iptv_recording(id)
}

/// Bij afsluiten: lopende opnames stoppen en wat er is bewaren.
pub fn shutdown(st: &AppState) {
    let running: Vec<(i64, Recorder)> = match st.iptv.recorders.lock() {
        Ok(mut m) => m.drain().collect(),
        Err(_) => return,
    };
    for (id, mut r) in running {
        let _ = r.child.kill();
        let _ = r.child.wait();
        let kept = finalize(&r.part, &r.target);
        if let Ok(db) = st.lock_db() {
            let _ = db.set_iptv_recording_status(
                id,
                "cancelled",
                kept.then(|| r.target.to_string_lossy().into_owned()).as_deref(),
                Some("De app is afgesloten tijdens de opname"),
            );
        }
    }
}

// ---------- handmatige gidskoppeling ----------

pub fn set_epg_manual(st: &AppState, channel_id: i64, epg_id: Option<&str>) -> Result<()> {
    st.lock_db()?.set_iptv_epg_manual(channel_id, epg_id)
}

pub fn epg_channels(st: &AppState) -> Result<Vec<EpgChannelInfo>> {
    Ok(st.lock_db()?.iptv_epg_channels()?.into_iter().map(|(id, name)| EpgChannelInfo { id, name: name.replace(" | ", " / ") }).collect())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct EpgChannelInfo {
    pub id: String,
    pub name: String,
}

pub fn search_programmes(st: &AppState, term: &str) -> Result<Vec<IptvProgramme>> {
    if term.trim().chars().count() < 2 {
        return Ok(vec![]);
    }
    st.lock_db()?.iptv_search_programmes(term, now(), 200)
}
