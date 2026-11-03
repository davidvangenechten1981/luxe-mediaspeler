use super::net::testserver::{serve, serve_with, Server};
use super::*;
use crate::services::settings as app_settings;
use crate::services::testutil::{app, fixture, wait_until, TestApp};
use std::collections::HashMap;
use vu_core::iptv::catchup::utc_parts;

fn allow_local_streams(a: &TestApp) {
    let mut s = app_settings::get(&a.state).unwrap().0;
    s.allow_local_streams = true;
    app_settings::save(&a.state, &s).unwrap();
}

fn video_bytes() -> Vec<u8> {
    std::fs::read(fixture("sample.mkv")).unwrap()
}

fn m3u(video: &str) -> String {
    format!(
        "#EXTM3U\n#EXTINF:-1 tvg-id=\"een.nl\" tvg-chno=\"1\" group-title=\"Nieuws\" catchup=\"shift\" catchup-days=\"3\",Een\n{video}/a.ts\n\
         #EXTINF:-1 tvg-id=\"twee.nl\" tvg-chno=\"2\" group-title=\"Sport\",Twee\n{video}/b.ts\n\
         #EXTINF:-1 group-title=\"Sport\",Drie HD\n{video}/c.ts\n"
    )
}

fn xmltv(now: i64) -> String {
    let f = |t: i64| {
        let (y, mo, d, h, mi, s) = utc_parts(t);
        format!("{y:04}{mo:02}{d:02}{h:02}{mi:02}{s:02} +0000")
    };
    format!(
        "<tv><channel id=\"een.nl\"><display-name>Een</display-name></channel><channel id=\"drie\"><display-name>Drie</display-name></channel>\
         <programme start=\"{}\" stop=\"{}\" channel=\"een.nl\"><title>Journaal</title></programme>\
         <programme start=\"{}\" stop=\"{}\" channel=\"een.nl\"><title>Film</title></programme></tv>",
        f(now - 1800),
        f(now + 1800),
        f(now + 1800),
        f(now + 7200)
    )
}

/// Server met een afspeellijst, een gids en de testvideo op elk pad dat met /a, /b, /c of /live begint.
fn media_server() -> Server {
    let video = video_bytes();
    serve_with(move |target| {
        let path = target.split('?').next().unwrap_or("/");
        match path {
            "/lijst.m3u" => Some(("audio/x-mpegurl".into(), b"PLACEHOLDER".to_vec())),
            _ if path.ends_with(".ts") || path.starts_with("/live/") => Some(("video/x-matroska".into(), video.clone())),
            _ => None,
        }
    })
}

fn sources_json(a: &TestApp) -> String {
    serde_json::to_string(&sources(&a.state).unwrap()).unwrap()
}

fn all_channels(a: &TestApp) -> Vec<IptvChannel> {
    channels(&a.state, &IptvChannelQuery { include_hidden: true, ..Default::default() }).unwrap()
}

fn current(a: &TestApp) -> Option<String> {
    a.state.player.as_ref().ok()?.view().ok()?.location
}

// ---------- bronnen ----------

#[test]
fn an_m3u_file_is_imported_refreshed_and_keeps_favorites() {
    let a = app();
    let dir = a.dir.path();
    let file = dir.join("tv.m3u");
    std::fs::write(&file, m3u("http://prov.example/live/jan/pw")).unwrap();
    let src = add_m3u_file(&a.state, "Thuis", file.to_str().unwrap(), None).unwrap();
    assert_eq!(src.kind, "m3u_file");
    let report = refresh_source(&a.state, src.id).unwrap();
    assert_eq!((report.channels, report.skipped), (3, 0));
    let list = all_channels(&a);
    assert_eq!(list.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["Een", "Twee", "Drie HD"]);
    assert_eq!(list[0].catchup_mode.as_deref(), Some("shift"));

    app_settings_favorite(&a, list[1].id);
    // de lijst wijzigt: Twee blijft, Een verdwijnt, Vier komt erbij
    std::fs::write(&file, "#EXTM3U\n#EXTINF:-1 tvg-id=\"twee.nl\" group-title=\"Sport\",Twee\nhttp://prov.example/b\n#EXTINF:-1,Vier\nhttp://prov.example/d\n").unwrap();
    refresh_source(&a.state, src.id).unwrap();
    let list = all_channels(&a);
    assert_eq!(list.iter().map(|c| (c.name.as_str(), c.favorite)).collect::<Vec<_>>(), vec![("Twee", true), ("Vier", false)]);
    assert!(sources(&a.state).unwrap()[0].last_refresh.is_some());

    // een kapotte of verdwenen lijst geeft een fout en bewaart de zenders
    std::fs::remove_file(&file).unwrap();
    assert!(refresh_source(&a.state, src.id).is_err());
    assert_eq!(all_channels(&a).len(), 2, "de oude zenders blijven staan");
    assert!(sources(&a.state).unwrap()[0].last_error.is_some());
}

fn app_settings_favorite(a: &TestApp, id: i64) {
    a.state.lock_db().unwrap().set_iptv_favorite(id, true).unwrap();
}

#[test]
fn bad_sources_are_refused_and_leave_nothing_behind() {
    let a = app();
    assert!(add_m3u_file(&a.state, "x", "C:\\bestaat\\niet.m3u", None).is_err());
    assert!(add_m3u_url(&a.state, "x", "ftp://example.com/l.m3u", None, None).is_err());
    assert!(add_m3u_url(&a.state, "x", "http://127.0.0.1/l.m3u", None, None).is_err(), "lokaal zonder toestemming");
    assert!(add_xtream(&a.state, "x", "geen url", "u", "p").is_err());
    assert!(add_xtream(&a.state, "x", "http://prov.example", "", "p").is_err());
    assert!(add_m3u_url(&a.state, "x", "https://prov.example/l.m3u", Some("ftp://gids"), None).is_err(), "kapotte gidslink");
    assert!(sources(&a.state).unwrap().is_empty(), "een mislukte bron blijft niet half staan");
}

#[test]
fn a_link_source_keeps_its_secret_out_of_the_database_and_the_interface() {
    let a = app();
    allow_local_streams(&a);
    let list = m3u("http://prov.example/live/jan/wachtwoord1");
    let s = serve(HashMap::from([("/lijst.m3u".to_string(), ("text/plain".to_string(), list.into_bytes()))]));
    let link = s.url("/lijst.m3u?token=geheim99&user=jan");
    let src = add_m3u_url(&a.state, "Via link", &link, None, Some("MijnAgent/1")).unwrap();
    assert!(src.location.contains("token=***") && !src.location.contains("geheim99"), "{}", src.location);
    assert!(
        a.state.secrets.get(&format!("source-{}-url", src.id)).unwrap().unwrap().contains("geheim99"),
        "het geheim staat wel in de opslag"
    );

    let report = refresh_source(&a.state, src.id).unwrap();
    assert_eq!(report.channels, 3);
    assert!(s.hits.lock().unwrap().iter().any(|h| h.contains("token=geheim99")), "de echte link is wel gebruikt");

    // niets wat naar de interface gaat bevat het geheim of de stream-URL's
    let json = format!("{}{}", sources_json(&a), serde_json::to_string(&all_channels(&a)).unwrap());
    for secret in ["geheim99", "wachtwoord1", "jan"] {
        assert!(!json.contains(secret), "{secret} lekt: {json}");
    }
    // verwijderen ruimt ook de opslag op
    delete_source(&a.state, src.id).unwrap();
    assert_eq!(a.state.secrets.get(&format!("source-{}-url", src.id)).unwrap(), None);
    assert!(all_channels(&a).is_empty());
}

#[test]
fn updating_a_source_sets_and_clears_the_guide_link() {
    let a = app();
    let dir = a.dir.path().join("l.m3u");
    std::fs::write(&dir, m3u("http://prov.example")).unwrap();
    let src = add_m3u_file(&a.state, "L", dir.to_str().unwrap(), None).unwrap();
    let s = update_source(&a.state, src.id, "Nieuwe naam", Some("https://gids.example/g.xml?key=abc"), Some("UA")).unwrap();
    assert_eq!((s.name.as_str(), s.user_agent.as_deref()), ("Nieuwe naam", Some("UA")));
    assert!(s.epg_location.contains("key=***") && !s.epg_location.contains("abc"));
    assert!(a.state.secrets.get(&format!("source-{}-epg", src.id)).unwrap().is_some());
    let s = update_source(&a.state, src.id, "Nieuwe naam", Some(""), Some("UA")).unwrap();
    assert_eq!(s.epg_location, "");
    assert_eq!(a.state.secrets.get(&format!("source-{}-epg", src.id)).unwrap(), None);
}

// ---------- gids ----------

#[test]
fn the_guide_is_fetched_from_the_playlist_header_and_matched_by_id_and_name() {
    let a = app();
    allow_local_streams(&a);
    let t = now();
    let xml = xmltv(t);
    let s = serve(HashMap::from([("/gids.xml".to_string(), ("text/xml".to_string(), xml.into_bytes()))]));
    let list =
        format!("#EXTM3U url-tvg=\"{}\"\n{}", s.url("/gids.xml"), m3u("http://prov.example/live/jan/pw").trim_start_matches("#EXTM3U\n"));
    let file = a.dir.path().join("tv.m3u");
    std::fs::write(&file, list).unwrap();
    let src = add_m3u_file(&a.state, "Thuis", file.to_str().unwrap(), None).unwrap();
    assert!(refresh_source(&a.state, src.id).unwrap().guide_link_found);
    assert!(!sources(&a.state).unwrap()[0].epg_location.is_empty(), "de gidslink uit de lijst is overgenomen");

    let g = refresh_guide(&a.state).unwrap();
    assert_eq!((g.programmes, g.matched), (2, 2), "Een via tvg-id en Drie HD via de naam");
    let list = all_channels(&a);
    let by = |n: &str| list.iter().find(|c| c.name == n).unwrap().clone();
    assert_eq!(
        (by("Een").epg_id.as_deref(), by("Drie HD").epg_id.as_deref(), by("Twee").epg_id.clone()),
        (Some("een.nl"), Some("drie"), None)
    );

    let nn = now_next(&a.state, &[by("Een").id, by("Twee").id]).unwrap();
    assert_eq!(nn.len(), 1);
    assert_eq!(nn[0].programmes.now.as_ref().unwrap().title, "Journaal");
    assert_eq!(nn[0].programmes.next.as_ref().unwrap().title, "Film");
    assert_eq!(guide(&a.state, by("Een").id, t - 3600, t + 3600 * 3).unwrap().len(), 2);
    assert!(guide(&a.state, by("Twee").id, 0, t + 86_400).unwrap().is_empty());

    // handmatig koppelen wint, en loslaten geeft de automatische koppeling terug
    set_epg_manual(&a.state, by("Twee").id, Some("een.nl")).unwrap();
    assert_eq!(all_channels(&a).iter().find(|c| c.name == "Twee").unwrap().epg_id.as_deref(), Some("een.nl"));
    assert_eq!(epg_channels(&a.state).unwrap().len(), 2);
    assert_eq!(search_programmes(&a.state, "film").unwrap().len(), 1);
    assert!(search_programmes(&a.state, "f").unwrap().is_empty(), "te korte zoekterm");
}

#[test]
fn refreshing_the_guide_without_any_guide_link_explains_what_is_missing() {
    let a = app();
    let file = a.dir.path().join("tv.m3u");
    std::fs::write(&file, m3u("http://prov.example")).unwrap();
    add_m3u_file(&a.state, "Thuis", file.to_str().unwrap(), None).unwrap();
    let err = refresh_guide(&a.state).unwrap_err().to_string();
    assert!(err.contains("programmagids"), "{err}");
}

// ---------- Xtream ----------

fn xtream_server() -> Server {
    let video = video_bytes();
    serve_with(move |t| {
        let json = |s: &str| Some(("application/json".to_string(), s.as_bytes().to_vec()));
        if t.starts_with("/player_api.php") {
            return if t.contains("action=get_live_categories") {
                json(r#"[{"category_id":"1","category_name":"Nieuws"}]"#)
            } else if t.contains("action=get_live_streams") {
                json(
                    r#"[{"num":1,"name":"Een","stream_id":101,"stream_icon":"https://l/een.png","epg_channel_id":"een.nl","category_id":"1","tv_archive":1,"tv_archive_duration":"3"},{"num":2,"name":"Twee","stream_id":102,"category_id":"1"}]"#,
                )
            } else if t.contains("action=get_vod_categories") {
                json(r#"[{"category_id":"5","category_name":"Films"}]"#)
            } else if t.contains("action=get_vod_streams") {
                json(r#"[{"name":"Film","stream_id":501,"category_id":"5","container_extension":"mkv"}]"#)
            } else if t.contains("action=get_series_categories") {
                json(r#"[{"category_id":"9","category_name":"Series"}]"#)
            } else if t.contains("action=get_series_info") {
                json(r#"{"episodes":{"1":[{"id":"9001","episode_num":1,"title":"Begin","container_extension":"mkv","season":1}]}}"#)
            } else if t.contains("action=get_series") {
                json(r#"[{"series_id":77,"name":"Serie","category_id":"9","cover":"c.jpg"}]"#)
            } else if t.contains("password=wrong") {
                json(r#"{"user_info":{"auth":0}}"#)
            } else {
                json(r#"{"user_info":{"auth":1,"status":"Active","exp_date":"1893456000","max_connections":"1"}}"#)
            };
        }
        if t.starts_with("/live/") || t.starts_with("/movie/") || t.starts_with("/series/") || t.starts_with("/timeshift/") {
            return Some(("video/x-matroska".to_string(), video.clone()));
        }
        None
    })
}

#[test]
fn an_xtream_source_loads_live_movies_and_series_and_hides_the_credentials() {
    let a = app();
    allow_local_streams(&a);
    let s = xtream_server();
    let src = add_xtream(&a.state, "Provider", &s.url(""), "gebruiker9", "hunter2").unwrap();
    assert_eq!(src.kind, "xtream");
    let report = refresh_source(&a.state, src.id).unwrap();
    assert_eq!(report.channels, 4, "2 live, 1 film, 1 serie");

    let list = all_channels(&a);
    let kinds: Vec<(&str, &str)> = list.iter().map(|c| (c.name.as_str(), c.kind.as_str())).collect();
    assert_eq!(kinds, vec![("Een", "live"), ("Twee", "live"), ("Film", "movie"), ("Serie", "series")]);
    let een = &list[0];
    assert_eq!(
        (een.group.as_deref(), een.catchup_mode.as_deref(), een.catchup_days, een.tvg_id.as_deref()),
        (Some("Nieuws"), Some("xc"), Some(3), Some("een.nl"))
    );
    assert_eq!(list[2].group.as_deref(), Some("Films"));

    let json = format!("{}{}", sources_json(&a), serde_json::to_string(&list).unwrap());
    for secret in ["gebruiker9", "hunter2"] {
        assert!(!json.contains(secret), "{secret} lekt naar de interface: {json}");
    }
    // de gebruikersnaam en het wachtwoord zijn wel (alleen) in de opslag te vinden
    assert!(a.state.secrets.get(&format!("source-{}-xtream", src.id)).unwrap().unwrap().contains("hunter2"));

    // de database bewaart alleen een neutrale verwijzing; de echte URL wordt pas bij het afspelen gebouwd
    {
        let db = a.state.lock_db().unwrap();
        let stored = db.list_iptv_channels(&IptvChannelQuery { include_hidden: true, ..Default::default() }).unwrap();
        for c in &stored {
            assert!(!c.url.contains("gebruiker9") && !c.url.contains("hunter2"), "{} bewaart inloggegevens: {}", c.name, c.url);
        }
    }
    if a.state.player.is_ok() {
        play_channel(&a.state, list[0].id).unwrap();
        assert!(
            wait_until(10, || s.hits.lock().unwrap().iter().any(|h| h.starts_with("/live/gebruiker9/hunter2/101.ts"))),
            "live-URL met inlog"
        );
        play_catchup(&a.state, list[0].id, now() - 7200, now() - 3600).unwrap();
        assert!(
            wait_until(10, || s.hits.lock().unwrap().iter().any(|h| h.starts_with("/timeshift/gebruiker9/hunter2/60/"))),
            "timeshift-URL met inlog"
        );
    }

    // series: afleveringen ophalen, daarna afspelen
    let eps = series_episodes(&a.state, list[3].id).unwrap();
    assert_eq!(eps, vec![EpisodeInfo { id: "9001".into(), season: 1, episode: 1, title: "Begin".into(), extension: "mkv".into() }]);
    assert!(series_episodes(&a.state, list[0].id).is_err(), "een zender is geen serie");
    assert!(play_channel(&a.state, list[3].id).is_err(), "een serie speel je niet direct af");
    assert!(play_episode(&a.state, list[3].id, "9001; rm", "mkv").is_err());
    assert!(play_episode(&a.state, list[3].id, "9001", "../x").is_err());
}

#[test]
fn a_wrong_xtream_login_is_reported_without_the_password() {
    let a = app();
    allow_local_streams(&a);
    let s = xtream_server();
    let src = add_xtream(&a.state, "Fout", &s.url(""), "jan", "wrong").unwrap();
    let err = refresh_source(&a.state, src.id).unwrap_err().to_string();
    assert!(err.contains("Inloggen is mislukt"), "{err}");
    assert!(!err.contains("password=wrong"), "{err}");
    assert_eq!(err.matches("Ongeldige invoer").count(), 1, "de melding herhaalt het voorvoegsel niet: {err}");
    let stored = sources(&a.state).unwrap()[0].last_error.clone().unwrap();
    assert!(!stored.contains("password=wrong"), "{stored}");
}

// ---------- afspelen ----------

fn imported_channels(a: &TestApp, s: &Server) -> Vec<IptvChannel> {
    let file = a.dir.path().join("play.m3u");
    std::fs::write(&file, m3u(&format!("http://127.0.0.1:{}/live/jan/pw", s.port))).unwrap();
    let src = add_m3u_file(&a.state, "Lokaal", file.to_str().unwrap(), None).unwrap();
    refresh_source(&a.state, src.id).unwrap();
    all_channels(a)
}

#[test]
fn playing_a_channel_uses_a_neutral_name_and_leaves_no_history() {
    let a = app();
    if a.state.player.is_err() {
        eprintln!("OVERGESLAGEN: geen LibVLC beschikbaar");
        return;
    }
    allow_local_streams(&a);
    let s = media_server();
    let list = imported_channels(&a, &s);
    play_channel(&a.state, list[0].id).unwrap();
    let token = format!("iptv://channel/{}", list[0].id);
    assert!(wait_until(10, || current(&a).as_deref() == Some(token.as_str())), "de speler toont de neutrale naam");
    assert!(wait_until(10, || s.hits.lock().unwrap().iter().any(|h| h.contains("/live/jan/pw/a.ts"))), "de echte URL is wel opgehaald");
    let view = format!("{:?}", a.state.player.as_ref().unwrap().view().unwrap());
    assert!(!view.contains("jan/pw"), "{view}");
    assert!(a.state.lock_db().unwrap().list_history(50).unwrap().is_empty(), "een livestream komt niet in de geschiedenis");
    assert!(all_channels(&a)[0].last_watched.is_some() && !recent(&a.state).unwrap().is_empty());
}

#[test]
fn zapping_wraps_around_and_numbers_and_favorites_work() {
    let a = app();
    if a.state.player.is_err() {
        eprintln!("OVERGESLAGEN: geen LibVLC beschikbaar");
        return;
    }
    allow_local_streams(&a);
    let s = media_server();
    let list = imported_channels(&a, &s);
    let ids: Vec<i64> = list.iter().map(|c| c.id).collect(); // Een(1), Twee(2), Drie HD (zonder nummer, laatst)
    assert_eq!(zap(&a.state, ids[2], 1, false).unwrap().id, ids[0], "voorbij het einde begint de lijst opnieuw");
    assert_eq!(zap(&a.state, ids[0], -1, false).unwrap().id, ids[2], "voor het begin komt het einde");
    assert_eq!(zap(&a.state, ids[0], 1, false).unwrap().id, ids[1]);
    assert_eq!(zap(&a.state, 99_999, 1, false).unwrap().id, ids[0], "onbekende start: begin vooraan");
    assert_eq!(play_number(&a.state, 2).unwrap().id, ids[1]);
    assert!(play_number(&a.state, 99).is_err());
    a.state.lock_db().unwrap().set_iptv_favorite(ids[2], true).unwrap();
    assert_eq!(zap(&a.state, ids[2], 1, true).unwrap().id, ids[2], "met één favoriet blijf je erop");
    a.state.lock_db().unwrap().set_iptv_favorite(ids[2], false).unwrap();
    assert!(zap(&a.state, ids[0], 1, true).is_err(), "geen favorieten");
}

#[test]
fn catchup_builds_a_url_checks_the_archive_and_plays() {
    let a = app();
    allow_local_streams(&a);
    let s = media_server();
    let list = imported_channels(&a, &s);
    let (een, twee) = (&list[0], &list[1]);
    let t = now();
    assert!(play_catchup(&a.state, twee.id, t - 7200, t - 3600).is_err(), "Twee biedt geen terugkijken");
    let err = play_catchup(&a.state, een.id, t - 10 * 86_400, t - 10 * 86_400 + 3600).unwrap_err().to_string();
    assert!(err.contains("archief"), "{err}");
    assert!(play_catchup(&a.state, een.id, t + 100, t + 3700).is_err(), "nog niet begonnen");
    if a.state.player.is_ok() {
        play_catchup(&a.state, een.id, t - 7200, t - 3600).unwrap();
        let token = format!("iptv://catchup/{}/{}", een.id, t - 7200);
        assert!(wait_until(10, || current(&a).as_deref() == Some(token.as_str())));
        assert!(wait_until(10, || s.hits.lock().unwrap().iter().any(|h| h.contains("a.ts?utc="))), "shift-modus: ?utc=... achter de URL");
    }
}

// ---------- herinneringen en opnemen ----------

#[test]
fn reminders_are_reported_once_when_the_programme_is_about_to_start() {
    let a = app();
    let list = {
        let file = a.dir.path().join("l.m3u");
        std::fs::write(&file, m3u("http://prov.example/live/jan/pw")).unwrap();
        let src = add_m3u_file(&a.state, "L", file.to_str().unwrap(), None).unwrap();
        refresh_source(&a.state, src.id).unwrap();
        all_channels(&a)
    };
    let t = now();
    let r = reminder_add(&a.state, list[0].id, "Nieuws", t + 60).unwrap();
    reminder_add(&a.state, list[0].id, "Later", t + 7200).unwrap();
    assert_eq!(reminders(&a.state).unwrap().len(), 2);
    let tick1 = tick(&a.state).unwrap();
    assert_eq!(tick1.reminders.iter().map(|x| x.id).collect::<Vec<_>>(), vec![r.id]);
    assert!(tick(&a.state).unwrap().reminders.is_empty(), "maar één keer melden");
    reminder_delete(&a.state, r.id).unwrap();
    assert_eq!(reminders(&a.state).unwrap().len(), 1);
}

fn wait_for_status(a: &TestApp, id: i64, want: &str) -> IptvRecording {
    let mut last = None;
    wait_until(40, || {
        let _ = tick(&a.state);
        last = a.state.lock_db().unwrap().iptv_recording(id).unwrap();
        last.as_ref().is_some_and(|r| r.status == want)
    });
    last.unwrap()
}

#[test]
fn recording_needs_a_folder_then_copies_the_stream_to_a_ts_file() {
    let a = app();
    if a.state.tools.ffmpeg.is_none() {
        eprintln!("OVERGESLAGEN: geen FFmpeg beschikbaar");
        return;
    }
    allow_local_streams(&a);
    let s = media_server();
    let list = imported_channels(&a, &s);

    // zonder map mislukt de opname met een duidelijke melding
    let rec = record_now(&a.state, list[0].id, 1).unwrap();
    assert_eq!(rec.status, "failed");
    assert!(rec.message.as_deref().unwrap_or("").contains("map voor opnames"), "{:?}", rec.message);

    // met map: opnemen tot het einde van de bron
    let out = a.dir.path().join("opnames");
    let mut settings = app_settings::get(&a.state).unwrap().0;
    settings.iptv_record_dir = Some(out.to_string_lossy().into_owned());
    app_settings::save(&a.state, &settings).unwrap();
    let rec = record_now(&a.state, list[0].id, 1).unwrap();
    assert_eq!(rec.status, "recording");
    let done = wait_for_status(&a, rec.id, "done");
    assert_eq!(done.status, "done", "{:?}", done.message);
    let path = std::path::PathBuf::from(done.path.clone().unwrap());
    let (y, mo, d, h, mi, _) = utc_parts(done.start + localtime::offset_secs(done.start));
    let stamp = format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}");
    assert!(path.file_name().unwrap().to_string_lossy().contains(&stamp), "de bestandsnaam gebruikt lokale tijd ({stamp}): {path:?}");
    assert!(path.extension().is_some_and(|e| e == "ts") && std::fs::metadata(&path).unwrap().len() > 1000, "{path:?}");
    assert!(!std::path::Path::new(&format!("{}.part", path.display())).exists(), "het .part-bestand is hernoemd");
    assert!(!serde_json::to_string(&recordings(&a.state).unwrap()).unwrap().contains("jan/pw"), "de URL gaat niet naar de interface");

    // de opname uit de lijst halen laat het bestand staan
    recording_delete(&a.state, done.id).unwrap();
    assert!(path.exists());
}

#[test]
fn missed_and_cancelled_recordings_get_the_right_status() {
    let a = app();
    let file = a.dir.path().join("l.m3u");
    std::fs::write(&file, m3u("http://prov.example/live/jan/pw")).unwrap();
    let src = add_m3u_file(&a.state, "L", file.to_str().unwrap(), None).unwrap();
    refresh_source(&a.state, src.id).unwrap();
    let ch = all_channels(&a)[0].id;
    let t = now();
    assert!(recording_add(&a.state, ch, "Voorbij", t - 7200, t - 3600).is_err(), "einde in het verleden");
    assert!(recording_add(&a.state, ch, "Omgekeerd", t + 100, t + 50).is_err());

    // een geplande opname waarvan de tijd voorbij is (app was uit) wordt als gemist gemeld
    let id = a
        .state
        .lock_db()
        .unwrap()
        .add_iptv_recording(&NewRecording {
            channel_name: "Een",
            url: "http://prov.example/x",
            user_agent: None,
            referrer: None,
            title: "Gemist",
            start: t - 7200,
            stop: t - 3600,
        })
        .unwrap();
    assert!(tick(&a.state).unwrap().recordings_changed);
    let r = a.state.lock_db().unwrap().iptv_recording(id).unwrap().unwrap();
    assert_eq!(r.status, "failed");
    assert!(r.message.unwrap().contains("Gemist"));

    // een geplande opname in de toekomst kan worden geannuleerd
    let planned = recording_add(&a.state, ch, "Later", t + 3600, t + 7200).unwrap();
    assert_eq!(planned.status, "planned");
    recording_cancel(&a.state, planned.id).unwrap();
    assert_eq!(a.state.lock_db().unwrap().iptv_recording(planned.id).unwrap().unwrap().status, "cancelled");

    // stoppen verandert een al afgeronde of mislukte opname niet (die status blijft kloppen)
    recording_cancel(&a.state, id).unwrap();
    assert_eq!(a.state.lock_db().unwrap().iptv_recording(id).unwrap().unwrap().status, "failed");
    assert!(recording_cancel(&a.state, 99_999).is_err(), "onbekende opname");
}

// ---------- VPN ----------

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Zet een neppe VPN-controle in de app; de teruggegeven schakelaar bepaalt of de VPN "actief" is.
fn vpn_switch(a: &mut TestApp, checked: bool) -> Arc<AtomicBool> {
    let active = Arc::new(AtomicBool::new(false));
    let flag = active.clone();
    a.state.vpn_probe = Box::new(move |_| vpn::VpnStatus {
        checked,
        active: flag.load(Ordering::SeqCst),
        adapter: flag.load(Ordering::SeqCst).then(|| "TestVPN".to_string()),
        detail: if flag.load(Ordering::SeqCst) {
            "Je internetverkeer loopt via de VPN".into()
        } else {
            "Geen actieve VPN-adapter gevonden".into()
        },
    });
    active
}

fn set_require_vpn(a: &TestApp, on: bool) {
    let mut s = app_settings::get(&a.state).unwrap().0;
    s.iptv_require_vpn = on;
    app_settings::save(&a.state, &s).unwrap();
}

#[test]
fn nothing_connects_without_a_vpn_when_the_vpn_is_required() {
    let mut a = app();
    allow_local_streams(&a);
    let vpn_on = vpn_switch(&mut a, true);
    let s =
        serve(HashMap::from([("/lijst.m3u".to_string(), ("text/plain".to_string(), m3u("http://127.0.0.1:1/live/jan/pw").into_bytes()))]));
    let src = add_m3u_url(&a.state, "Via link", &s.url("/lijst.m3u"), None, None).unwrap();

    set_require_vpn(&a, true);
    let err = refresh_source(&a.state, src.id).unwrap_err().to_string();
    assert!(err.contains("Geen VPN actief"), "{err}");
    assert!(s.hits.lock().unwrap().is_empty(), "er is niets opgehaald zonder VPN");
    assert!(refresh_guide(&a.state).is_err());
    let status = vpn_check(&a.state).unwrap();
    assert!(status.required && !status.status.active && !status.stopped, "{status:?}");

    // met VPN werkt alles
    vpn_on.store(true, Ordering::SeqCst);
    assert_eq!(refresh_source(&a.state, src.id).unwrap().channels, 3);
    let status = vpn_check(&a.state).unwrap();
    assert!(status.status.active && status.status.adapter.as_deref() == Some("TestVPN"), "{status:?}");

    // de eis uit: ook zonder VPN
    vpn_on.store(false, Ordering::SeqCst);
    set_require_vpn(&a, false);
    assert_eq!(refresh_source(&a.state, src.id).unwrap().channels, 3);
    assert!(!vpn_check(&a.state).unwrap().stopped);
}

#[test]
fn an_unavailable_vpn_check_blocks_when_required_and_says_why() {
    let mut a = app();
    let _ = vpn_switch(&mut a, false);
    set_require_vpn(&a, true);
    let file = a.dir.path().join("l.m3u");
    std::fs::write(&file, m3u("http://prov.example/live/jan/pw")).unwrap();
    let src = add_m3u_file(&a.state, "L", file.to_str().unwrap(), None).unwrap();
    refresh_source(&a.state, src.id).unwrap(); // een lokaal bestand gebruikt het netwerk niet
    let id = all_channels(&a)[0].id;
    let err = play_channel(&a.state, id).unwrap_err().to_string();
    assert!(err.contains("VPN-controle kon niet worden uitgevoerd"), "{err}");
}

#[test]
fn the_kill_switch_stops_a_live_stream_when_the_vpn_drops() {
    let mut a = app();
    if a.state.player.is_err() {
        eprintln!("OVERGESLAGEN: geen LibVLC beschikbaar");
        return;
    }
    allow_local_streams(&a);
    let vpn_on = vpn_switch(&mut a, true);
    vpn_on.store(true, Ordering::SeqCst);
    set_require_vpn(&a, true);
    let s = media_server();
    let list = imported_channels(&a, &s);
    play_channel(&a.state, list[0].id).unwrap();
    let token = format!("iptv://channel/{}", list[0].id);
    assert!(wait_until(10, || current(&a).as_deref() == Some(token.as_str())));
    assert!(!vpn_check(&a.state).unwrap().stopped, "met VPN blijft de stream lopen");

    vpn_on.store(false, Ordering::SeqCst);
    let c = vpn_check(&a.state).unwrap();
    assert!(c.stopped, "{c:?}");
    assert!(
        wait_until(10, || !matches!(
            a.state.player.as_ref().unwrap().view().unwrap().status.state,
            vu_core::player::session::PlaybackState::Playing
        )),
        "de stream is gestopt"
    );
    assert!(play_channel(&a.state, list[0].id).is_err(), "opnieuw starten zonder VPN lukt niet");
}

#[test]
fn a_planned_recording_waits_for_the_vpn_and_then_starts() {
    let mut a = app();
    if a.state.tools.ffmpeg.is_none() {
        eprintln!("OVERGESLAGEN: geen FFmpeg beschikbaar");
        return;
    }
    allow_local_streams(&a);
    let vpn_on = vpn_switch(&mut a, true);
    set_require_vpn(&a, true);
    let out = a.dir.path().join("opnames");
    let mut settings = app_settings::get(&a.state).unwrap().0;
    settings.iptv_record_dir = Some(out.to_string_lossy().into_owned());
    app_settings::save(&a.state, &settings).unwrap();
    let s = media_server();
    let list = imported_channels(&a, &s);

    let rec = record_now(&a.state, list[0].id, 1).unwrap();
    assert_eq!(rec.status, "planned", "zonder VPN wacht de opname");
    assert!(rec.message.as_deref().unwrap_or("").contains("Geen VPN actief"), "{:?}", rec.message);
    assert!(s.hits.lock().unwrap().is_empty());

    vpn_on.store(true, Ordering::SeqCst);
    let done = wait_for_status(&a, rec.id, "done");
    assert_eq!(done.status, "done", "{:?}", done.message);
    assert!(done.path.is_some());
}

// ---------- logo's ----------

fn png() -> Vec<u8> {
    let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    b.extend_from_slice(&[0; 32]);
    b
}

#[test]
fn logos_are_only_fetched_when_enabled_checked_and_cached() {
    let mut a = app();
    allow_local_streams(&a);
    let s = serve(HashMap::from([
        ("/goed.png".to_string(), ("image/png".to_string(), png())),
        ("/fout.png".to_string(), ("text/html".to_string(), b"<html>geen plaatje</html>".to_vec())),
    ]));
    let list = format!(
        "#EXTM3U\n#EXTINF:-1 tvg-logo=\"{u}/goed.png\",Goed\nhttp://prov.example/1\n#EXTINF:-1 tvg-logo=\"{u}/fout.png\",Fout\nhttp://prov.example/2\n#EXTINF:-1,Zonder\nhttp://prov.example/3\n",
        u = format!("http://127.0.0.1:{}", s.port)
    );
    let file = a.dir.path().join("l.m3u");
    std::fs::write(&file, list).unwrap();
    let src = add_m3u_file(&a.state, "L", file.to_str().unwrap(), None).unwrap();
    refresh_source(&a.state, src.id).unwrap();
    let by = |n: &str| all_channels(&a).into_iter().find(|c| c.name == n).unwrap().id;
    let (goed, fout, zonder) = (by("Goed"), by("Fout"), by("Zonder"));

    // standaard uit: niets ophalen
    assert_eq!(logo_path(&a.state, goed).unwrap(), None);
    assert!(s.hits.lock().unwrap().is_empty(), "logo's staan standaard uit: er is geen verbinding gemaakt");

    let mut settings = app_settings::get(&a.state).unwrap().0;
    settings.iptv_show_logos = true;
    app_settings::save(&a.state, &settings).unwrap();
    let path = logo_path(&a.state, goed).unwrap().expect("logo opgehaald");
    assert!(path.ends_with(".png") && std::fs::metadata(&path).unwrap().len() > 8, "{path}");
    assert!(path.contains("thumbnails"), "binnen de map die de interface mag lezen: {path}");
    assert_eq!(logo_path(&a.state, goed).unwrap().as_deref(), Some(path.as_str()));
    assert_eq!(s.hits.lock().unwrap().len(), 1, "de tweede keer komt uit de cache");

    // geen plaatje: geen bestand en niet nog eens proberen
    assert_eq!(logo_path(&a.state, fout).unwrap(), None);
    assert_eq!(logo_path(&a.state, fout).unwrap(), None);
    assert_eq!(s.hits.lock().unwrap().len(), 2, "een mislukt logo wordt niet steeds opnieuw geprobeerd");
    assert_eq!(logo_path(&a.state, zonder).unwrap(), None);

    // zonder VPN (als die verplicht is) wordt er niets opgehaald
    let _ = vpn_switch(&mut a, true);
    set_require_vpn(&a, true);
    let other = a.dir.path().join("l2.m3u");
    std::fs::write(
        &other,
        format!("#EXTM3U\n#EXTINF:-1 tvg-logo=\"http://127.0.0.1:{}/goed.png?v=2\",Nieuw\nhttp://prov.example/9\n", s.port),
    )
    .unwrap();
    let src2 = add_m3u_file(&a.state, "L2", other.to_str().unwrap(), None).unwrap();
    refresh_source(&a.state, src2.id).unwrap();
    let nieuw = all_channels(&a).into_iter().find(|c| c.name == "Nieuw").unwrap().id;
    let before = s.hits.lock().unwrap().len();
    assert_eq!(logo_path(&a.state, nieuw).unwrap(), None);
    assert_eq!(s.hits.lock().unwrap().len(), before, "geen verbinding zonder VPN");
}
