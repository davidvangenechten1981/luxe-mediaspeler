//! Xtream Codes: URL's bouwen en de JSON-antwoorden lezen (live, films, series). Zonder netwerk;
//! het ophalen gebeurt elders. Het wachtwoord zit alleen in `Account` en in de gebouwde URL's, en nooit in `Debug`.

use crate::error::{CoreError, Result};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

#[derive(Clone)]
pub struct Account {
    origin: Url,
    username: String,
    password: String,
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account").field("server", &self.origin.as_str()).field("username", &"***").field("password", &"***").finish()
    }
}

impl Account {
    /// `server` is bijvoorbeeld `http://prov.example:8080` (een pad erachter wordt genegeerd).
    pub fn new(server: &str, username: &str, password: &str) -> Result<Account> {
        let url = Url::parse(server.trim()).map_err(|_| CoreError::Invalid("Ongeldig serveradres".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(CoreError::Invalid("Een Xtream-server moet met http:// of https:// beginnen".into()));
        }
        if username.trim().is_empty() || password.is_empty() {
            return Err(CoreError::Invalid("Gebruikersnaam en wachtwoord zijn verplicht".into()));
        }
        let mut origin = url;
        origin.set_path("");
        origin.set_query(None);
        origin.set_fragment(None);
        let _ = origin.set_username("");
        let _ = origin.set_password(None);
        Ok(Account { origin, username: username.trim().to_string(), password: password.to_string() })
    }

    pub fn server(&self) -> &str {
        self.origin.as_str().trim_end_matches('/')
    }

    /// `player_api.php?username=..&password=..[&action=..]`
    pub fn api_url(&self, action: Option<&str>, extra: &[(&str, &str)]) -> String {
        let mut u = self.origin.clone();
        u.set_path("player_api.php");
        {
            let mut q = u.query_pairs_mut();
            q.append_pair("username", &self.username).append_pair("password", &self.password);
            if let Some(a) = action {
                q.append_pair("action", a);
            }
            for (k, v) in extra {
                q.append_pair(k, v);
            }
        }
        u.to_string()
    }

    fn path_url(&self, kind: &str, file: &str) -> String {
        let mut u = self.origin.clone();
        if let Ok(mut p) = u.path_segments_mut() {
            p.clear().extend([kind, self.username.as_str(), self.password.as_str(), file]);
        }
        u.to_string()
    }

    pub fn live_url(&self, stream_id: &str, ext: &str) -> String {
        self.path_url("live", &format!("{stream_id}.{ext}"))
    }
    pub fn movie_url(&self, stream_id: &str, ext: &str) -> String {
        self.path_url("movie", &format!("{stream_id}.{ext}"))
    }
    pub fn series_url(&self, episode_id: &str, ext: &str) -> String {
        self.path_url("series", &format!("{episode_id}.{ext}"))
    }
    pub fn xmltv_url(&self) -> String {
        let mut u = self.origin.clone();
        u.set_path("xmltv.php");
        u.query_pairs_mut().append_pair("username", &self.username).append_pair("password", &self.password);
        u.to_string()
    }
}

/// Tekst of getal uit een JSON-veld als tekst (Xtream-aanbieders mengen beide).
fn text(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn number(v: &Value, key: &str) -> Option<i64> {
    match v.get(key)? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn array(json: &str) -> Result<Vec<Value>> {
    match serde_json::from_str::<Value>(json) {
        Ok(Value::Array(a)) => Ok(a),
        Ok(Value::Null) => Ok(vec![]),
        Ok(_) => Err(CoreError::Invalid("Onverwacht antwoord van de Xtream-server (geen lijst)".into())),
        Err(e) => Err(CoreError::Invalid(format!("Onleesbaar antwoord van de Xtream-server: {e}"))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInfo {
    pub status: String,
    pub expires: Option<i64>,
    pub max_connections: Option<u32>,
    pub active_connections: Option<u32>,
}

pub fn parse_account_info(json: &str) -> Result<AccountInfo> {
    let v: Value = serde_json::from_str(json).map_err(|e| CoreError::Invalid(format!("Onleesbaar antwoord van de Xtream-server: {e}")))?;
    let info = v.get("user_info").ok_or_else(|| CoreError::Invalid("De server gaf geen accountgegevens (verkeerde inlog?)".into()))?;
    if number(info, "auth") == Some(0) {
        return Err(CoreError::Invalid("Inloggen is mislukt: controleer gebruikersnaam en wachtwoord".into()));
    }
    Ok(AccountInfo {
        status: text(info, "status").unwrap_or_else(|| "onbekend".into()),
        expires: number(info, "exp_date"),
        max_connections: number(info, "max_connections").map(|n| n as u32),
        active_connections: number(info, "active_cons").map(|n| n as u32),
    })
}

/// category_id naar categorienaam.
pub fn parse_categories(json: &str) -> Result<HashMap<String, String>> {
    Ok(array(json)?.iter().filter_map(|c| Some((text(c, "category_id")?, text(c, "category_name")?))).collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStream {
    pub stream_id: String,
    pub name: String,
    pub number: Option<u32>,
    pub category_id: Option<String>,
    pub logo: Option<String>,
    pub epg_channel_id: Option<String>,
    pub archive_days: u32,
}

pub fn parse_live_streams(json: &str) -> Result<Vec<LiveStream>> {
    Ok(array(json)?
        .iter()
        .filter_map(|s| {
            Some(LiveStream {
                stream_id: text(s, "stream_id")?,
                name: text(s, "name")?,
                number: number(s, "num").map(|n| n as u32),
                category_id: text(s, "category_id"),
                logo: text(s, "stream_icon"),
                epg_channel_id: text(s, "epg_channel_id"),
                archive_days: if number(s, "tv_archive") == Some(1) {
                    number(s, "tv_archive_duration").unwrap_or(0).max(0) as u32
                } else {
                    0
                },
            })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VodStream {
    pub stream_id: String,
    pub name: String,
    pub category_id: Option<String>,
    pub poster: Option<String>,
    pub extension: String,
}

pub fn parse_vod_streams(json: &str) -> Result<Vec<VodStream>> {
    Ok(array(json)?
        .iter()
        .filter_map(|s| {
            Some(VodStream {
                stream_id: text(s, "stream_id")?,
                name: text(s, "name")?,
                category_id: text(s, "category_id"),
                poster: text(s, "stream_icon"),
                extension: text(s, "container_extension").unwrap_or_else(|| "mp4".into()),
            })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    pub series_id: String,
    pub name: String,
    pub category_id: Option<String>,
    pub cover: Option<String>,
}

pub fn parse_series_list(json: &str) -> Result<Vec<Series>> {
    Ok(array(json)?
        .iter()
        .filter_map(|s| {
            Some(Series {
                series_id: text(s, "series_id")?,
                name: text(s, "name")?,
                category_id: text(s, "category_id"),
                cover: text(s, "cover"),
            })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    pub id: String,
    pub season: u32,
    pub episode: u32,
    pub title: String,
    pub extension: String,
}

/// `get_series_info`: `{"episodes": {"1": [ {id, episode_num, title, container_extension, season}, ... ], "2": [...]}}`.
pub fn parse_series_info(json: &str) -> Result<Vec<Episode>> {
    let v: Value = serde_json::from_str(json).map_err(|e| CoreError::Invalid(format!("Onleesbaar antwoord van de Xtream-server: {e}")))?;
    let mut out = vec![];
    let groups: Vec<(u32, &Value)> = match v.get("episodes") {
        Some(Value::Object(m)) => m.iter().map(|(k, val)| (k.trim().parse().unwrap_or(0), val)).collect(),
        Some(Value::Array(a)) => a.iter().enumerate().map(|(i, val)| (i as u32 + 1, val)).collect(),
        _ => vec![],
    };
    for (season_key, eps) in groups {
        let Value::Array(eps) = eps else { continue };
        for e in eps {
            let (Some(id), episode) = (text(e, "id"), number(e, "episode_num").unwrap_or(0).max(0) as u32) else { continue };
            out.push(Episode {
                id,
                season: number(e, "season").map_or(season_key, |s| s.max(0) as u32),
                episode,
                title: text(e, "title").unwrap_or_else(|| format!("Aflevering {episode}")),
                extension: text(e, "container_extension").unwrap_or_else(|| "mp4".into()),
            });
        }
    }
    out.sort_by_key(|e| (e.season, e.episode));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acct() -> Account {
        Account::new("http://prov.example:8080/ergens/pad?x=1", "jan", "p@ss w/oord").unwrap()
    }

    #[test]
    fn urls_are_built_and_encoded_from_the_origin() {
        let a = acct();
        assert_eq!(a.server(), "http://prov.example:8080");
        assert_eq!(
            a.api_url(Some("get_live_streams"), &[("category_id", "5")]),
            "http://prov.example:8080/player_api.php?username=jan&password=p%40ss+w%2Foord&action=get_live_streams&category_id=5"
        );
        assert_eq!(a.live_url("12", "ts"), "http://prov.example:8080/live/jan/p@ss%20w%2Foord/12.ts");
        assert_eq!(a.movie_url("7", "mkv"), "http://prov.example:8080/movie/jan/p@ss%20w%2Foord/7.mkv");
        assert_eq!(a.series_url("9", "mp4"), "http://prov.example:8080/series/jan/p@ss%20w%2Foord/9.mp4");
        assert!(a.xmltv_url().starts_with("http://prov.example:8080/xmltv.php?username=jan&password="));
    }

    #[test]
    fn built_urls_are_fully_masked_by_the_mask_module() {
        let a = acct();
        for url in [a.live_url("12", "ts"), a.api_url(None, &[]), a.xmltv_url()] {
            let masked = crate::iptv::mask::mask_url(&url);
            assert!(!masked.contains("jan") && !masked.contains("oord"), "{masked}");
        }
    }

    #[test]
    fn the_account_never_prints_its_secrets() {
        let dump = format!("{:?}", acct());
        assert!(!dump.contains("jan") && !dump.contains("oord"), "{dump}");
    }

    #[test]
    fn bad_accounts_are_refused() {
        assert!(Account::new("ftp://x", "a", "b").is_err());
        assert!(Account::new("geen url", "a", "b").is_err());
        assert!(Account::new("http://x.example", "", "b").is_err());
        assert!(Account::new("http://x.example", "a", "").is_err());
    }

    #[test]
    fn account_info_is_read_and_failed_logins_are_reported() {
        let ok = r#"{"user_info":{"auth":1,"status":"Active","exp_date":"1893456000","max_connections":"2","active_cons":"1"},"server_info":{}}"#;
        let i = parse_account_info(ok).unwrap();
        assert_eq!(
            (i.status.as_str(), i.expires, i.max_connections, i.active_connections),
            ("Active", Some(1_893_456_000), Some(2), Some(1))
        );
        assert!(parse_account_info(r#"{"user_info":{"auth":0}}"#).is_err());
        assert!(parse_account_info(r#"{"iets":"anders"}"#).is_err());
        assert!(parse_account_info("<html>").is_err());
    }

    #[test]
    fn live_streams_and_categories_tolerate_mixed_types() {
        let cats = parse_categories(
            r#"[{"category_id":"1","category_name":"Nieuws"},{"category_id":2,"category_name":"Sport"},{"category_name":"zonder id"}]"#,
        )
        .unwrap();
        assert_eq!(cats.get("1").map(String::as_str), Some("Nieuws"));
        assert_eq!(cats.get("2").map(String::as_str), Some("Sport"));
        assert_eq!(cats.len(), 2);

        let live = parse_live_streams(
            r#"[{"num":1,"name":"Een","stream_id":101,"stream_icon":"https://l/een.png","epg_channel_id":"een.nl","category_id":"1","tv_archive":1,"tv_archive_duration":"3"},
                {"num":"2","name":"Twee","stream_id":"102","category_id":null,"tv_archive":0},
                {"name":"zonder id"}]"#,
        )
        .unwrap();
        assert_eq!(live.len(), 2);
        assert_eq!((live[0].stream_id.as_str(), live[0].archive_days, live[0].epg_channel_id.as_deref()), ("101", 3, Some("een.nl")));
        assert_eq!((live[1].number, live[1].category_id.clone(), live[1].archive_days), (Some(2), None, 0));
        assert_eq!(parse_live_streams("null").unwrap().len(), 0);
        assert!(parse_live_streams(r#"{"fout":1}"#).is_err());
    }

    #[test]
    fn vod_and_series_are_read() {
        let vod = parse_vod_streams(
            r#"[{"name":"Film","stream_id":5,"container_extension":"mkv","stream_icon":"p.jpg"},{"name":"Zonder ext","stream_id":6}]"#,
        )
        .unwrap();
        assert_eq!((vod[0].extension.as_str(), vod[1].extension.as_str()), ("mkv", "mp4"));
        let series = parse_series_list(r#"[{"series_id":3,"name":"Serie","cover":"c.jpg","category_id":"9"}]"#).unwrap();
        assert_eq!((series[0].series_id.as_str(), series[0].cover.as_deref()), ("3", Some("c.jpg")));

        let info = r#"{"episodes":{"2":[{"id":"22","episode_num":1,"title":"S2E1","container_extension":"mkv","season":2}],
                                  "1":[{"id":"12","episode_num":"2","title":"","season":1},{"id":"11","episode_num":1,"title":"Begin","season":1}]}}"#;
        let eps = parse_series_info(info).unwrap();
        let order: Vec<(u32, u32, &str)> = eps.iter().map(|e| (e.season, e.episode, e.id.as_str())).collect();
        assert_eq!(order, vec![(1, 1, "11"), (1, 2, "12"), (2, 1, "22")]);
        assert_eq!(eps[1].title, "Aflevering 2");
        assert_eq!(eps[2].extension, "mkv");
    }
}
