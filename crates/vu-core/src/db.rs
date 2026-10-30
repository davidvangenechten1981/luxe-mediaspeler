//! SQLite-opslag: bibliotheek, playlists, geschiedenis, bladwijzers, instellingen.
//! Schema-wijzigingen lopen via genummerde migraties (`PRAGMA user_version`).

use crate::audio::{builtin_presets, EqConfig, EqPreset};
use crate::error::{CoreError, Result};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATIONS: &[&str] = &[
    // 1: basisschema
    r#"
    CREATE TABLE media (
        id INTEGER PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        kind TEXT NOT NULL CHECK (kind IN ('video','audio')),
        title TEXT, artist TEXT, album TEXT, genre TEXT, year INTEGER, track INTEGER,
        duration_ms INTEGER NOT NULL DEFAULT 0,
        width INTEGER, height INTEGER, codec TEXT, container TEXT, bitrate INTEGER,
        size INTEGER NOT NULL DEFAULT 0,
        mtime INTEGER NOT NULL DEFAULT 0,
        added_at INTEGER NOT NULL,
        missing INTEGER NOT NULL DEFAULT 0,
        favorite INTEGER NOT NULL DEFAULT 0,
        play_count INTEGER NOT NULL DEFAULT 0,
        last_played INTEGER
    );
    CREATE INDEX idx_media_kind ON media(kind);
    CREATE INDEX idx_media_title ON media(title);
    CREATE INDEX idx_media_artist ON media(artist);
    CREATE INDEX idx_media_album ON media(album);
    CREATE INDEX idx_media_added ON media(added_at);
    CREATE INDEX idx_media_played ON media(last_played);
    CREATE INDEX idx_media_size ON media(size);

    CREATE TABLE library_folders (
        id INTEGER PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        added_at INTEGER NOT NULL
    );

    CREATE TABLE playlists (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );
    CREATE TABLE playlist_items (
        id INTEGER PRIMARY KEY,
        playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
        position INTEGER NOT NULL,
        location TEXT NOT NULL,
        title TEXT
    );
    CREATE INDEX idx_playlist_items ON playlist_items(playlist_id, position);

    CREATE TABLE resume (
        location TEXT PRIMARY KEY,
        position_ms INTEGER NOT NULL,
        duration_ms INTEGER NOT NULL,
        completed INTEGER NOT NULL DEFAULT 0,
        updated_at INTEGER NOT NULL
    );
    CREATE INDEX idx_resume_updated ON resume(updated_at);

    CREATE TABLE history (
        id INTEGER PRIMARY KEY,
        location TEXT NOT NULL,
        title TEXT,
        played_at INTEGER NOT NULL
    );
    CREATE INDEX idx_history_played ON history(played_at);

    CREATE TABLE bookmarks (
        id INTEGER PRIMARY KEY,
        location TEXT NOT NULL,
        position_ms INTEGER NOT NULL,
        label TEXT,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX idx_bookmarks_location ON bookmarks(location);

    CREATE TABLE settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
    // 2: eigen equalizer-presets en slimme collecties
    r#"
    CREATE TABLE eq_presets (
        name TEXT PRIMARY KEY COLLATE NOCASE,
        genre TEXT,
        preamp REAL NOT NULL,
        bands TEXT NOT NULL
    );
    CREATE TABLE collections (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL COLLATE NOCASE UNIQUE,
        query TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );
    "#,
    // 3: gekozen audio- en ondertitelspoor per bestand
    r#"
    CREATE TABLE track_prefs (
        location TEXT PRIMARY KEY,
        audio_key TEXT,
        subtitle_key TEXT,
        subtitle_off INTEGER NOT NULL DEFAULT 0,
        updated_at INTEGER NOT NULL
    );
    "#,
];

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewMedia {
    pub path: String,
    pub kind: MediaKind,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub duration_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub container: Option<String>,
    pub bitrate: Option<u64>,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Video,
    Audio,
}

impl MediaKind {
    fn as_str(self) -> &'static str {
        match self {
            MediaKind::Video => "video",
            MediaKind::Audio => "audio",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Media {
    pub id: i64,
    pub path: String,
    pub kind: MediaKind,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub duration_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub container: Option<String>,
    pub bitrate: Option<u64>,
    pub size: u64,
    pub mtime: i64,
    pub added_at: i64,
    pub missing: bool,
    pub favorite: bool,
    pub play_count: u32,
    pub last_played: Option<i64>,
}

const MEDIA_COLS: &str = "id, path, kind, title, artist, album, genre, year, track, duration_ms, width, height, \
    codec, container, bitrate, size, mtime, added_at, missing, favorite, play_count, last_played";

fn media_from_row(r: &Row) -> rusqlite::Result<Media> {
    Ok(Media {
        id: r.get(0)?,
        path: r.get(1)?,
        kind: if r.get::<_, String>(2)? == "audio" { MediaKind::Audio } else { MediaKind::Video },
        title: r.get(3)?,
        artist: r.get(4)?,
        album: r.get(5)?,
        genre: r.get(6)?,
        year: r.get(7)?,
        track: r.get(8)?,
        duration_ms: r.get::<_, i64>(9)? as u64,
        width: r.get(10)?,
        height: r.get(11)?,
        codec: r.get(12)?,
        container: r.get(13)?,
        bitrate: r.get::<_, Option<i64>>(14)?.map(|v| v as u64),
        size: r.get::<_, i64>(15)? as u64,
        mtime: r.get(16)?,
        added_at: r.get(17)?,
        missing: r.get::<_, i64>(18)? != 0,
        favorite: r.get::<_, i64>(19)? != 0,
        play_count: r.get(20)?,
        last_played: r.get(21)?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    #[default]
    Title,
    Artist,
    Album,
    Added,
    Played,
    Size,
    Duration,
    Year,
    PlayCount,
}

impl SortField {
    fn column(self) -> &'static str {
        match self {
            SortField::Title => "COALESCE(title, path) COLLATE NOCASE",
            SortField::Artist => "COALESCE(artist, '') COLLATE NOCASE",
            SortField::Album => "COALESCE(album, '') COLLATE NOCASE",
            SortField::Added => "added_at",
            SortField::Played => "COALESCE(last_played, 0)",
            SortField::Size => "size",
            SortField::Duration => "duration_ms",
            SortField::Year => "COALESCE(year, 0)",
            SortField::PlayCount => "play_count",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaQuery {
    pub kind: Option<MediaKind>,
    pub text: Option<String>,
    pub container: Option<String>,
    pub min_duration_ms: Option<u64>,
    pub max_duration_ms: Option<u64>,
    pub min_height: Option<u32>,
    pub added_since: Option<i64>,
    pub favorite_only: bool,
    pub played_only: bool,
    pub include_missing: bool,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub sort: SortField,
    pub descending: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumePoint {
    pub location: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub completed: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub location: String,
    pub title: Option<String>,
    pub played_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub item_count: u32,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: i64,
    pub position: u32,
    pub location: String,
    pub title: Option<String>,
}

/// De spoorkeuze van de gebruiker voor één bestand. Een sleutel is een taalcode (bijv. "nld") of, als de
/// taal onbekend is, de spoornaam in kleine letters; zo vinden we het spoor terug als de nummers anders zijn.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TrackPref {
    pub audio_key: Option<String>,
    pub subtitle_key: Option<String>,
    /// De gebruiker heeft de ondertitels bewust uitgezet.
    pub subtitle_off: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub location: String,
    pub position_ms: u64,
    pub label: Option<String>,
    pub created_at: i64,
}

impl Database {
    pub fn open(path: &Path) -> Result<Database> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Database> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Database> {
        conn.pragma_update(None, "foreign_keys", true)?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL"); // bij :memory: genegeerd
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let mut db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn schema_version(&self) -> Result<u32> {
        Ok(self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    fn migrate(&mut self) -> Result<()> {
        let current = self.schema_version()? as usize;
        if current > MIGRATIONS.len() {
            return Err(CoreError::Invalid(format!(
                "De database komt van een nieuwere versie van de app (schema {current}, deze app kent {})",
                MIGRATIONS.len()
            )));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as u32)?;
            tx.commit()?;
        }
        Ok(())
    }

    /// Snelle controle op beschadiging. `Ok(())` betekent gezond.
    pub fn integrity_check(&self) -> Result<()> {
        let res: String = self.conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if res == "ok" {
            Ok(())
        } else {
            Err(CoreError::Invalid(format!("Databasecontrole mislukt: {res}")))
        }
    }

    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        let mut out = Connection::open(dest)?;
        let b = rusqlite::backup::Backup::new(&self.conn, &mut out)?;
        b.run_to_completion(64, std::time::Duration::from_millis(5), None)?;
        Ok(())
    }

    /// Vervangt de inhoud van deze database door die van een back-up (na controle van die back-up).
    pub fn restore_from(&mut self, src: &Path) -> Result<()> {
        let source = Connection::open(src)?;
        let check: String = source.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let version: u32 = source.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if check != "ok" || version as usize > MIGRATIONS.len() || version == 0 {
            return Err(CoreError::Invalid("De back-up is beschadigd of afkomstig van een nieuwere versie".into()));
        }
        {
            let b = rusqlite::backup::Backup::new(&source, &mut self.conn)?;
            b.run_to_completion(64, std::time::Duration::from_millis(5), None)?;
        }
        self.migrate()
    }

    // ---------- bibliotheek ----------

    /// Voegt toe of werkt bij (op pad). Behoudt favoriet, afspeelteller en datum "toegevoegd".
    pub fn upsert_media(&mut self, m: &NewMedia) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO media (path, kind, title, artist, album, genre, year, track, duration_ms, width, height,
                codec, container, bitrate, size, mtime, added_at, missing)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,0)
             ON CONFLICT(path) DO UPDATE SET kind=?2, title=?3, artist=?4, album=?5, genre=?6, year=?7, track=?8,
                duration_ms=?9, width=?10, height=?11, codec=?12, container=?13, bitrate=?14, size=?15, mtime=?16,
                missing=0",
            params![
                m.path,
                m.kind.as_str(),
                m.title,
                m.artist,
                m.album,
                m.genre,
                m.year,
                m.track,
                m.duration_ms as i64,
                m.width,
                m.height,
                m.codec,
                m.container,
                m.bitrate.map(|b| b as i64),
                m.size as i64,
                m.mtime,
                now()
            ],
        )?;
        Ok(self.conn.query_row("SELECT id FROM media WHERE path = ?1", [&m.path], |r| r.get(0))?)
    }

    pub fn get_media(&self, id: i64) -> Result<Option<Media>> {
        Ok(self.conn.query_row(&format!("SELECT {MEDIA_COLS} FROM media WHERE id = ?1"), [id], media_from_row).optional()?)
    }

    pub fn get_media_by_path(&self, path: &str) -> Result<Option<Media>> {
        Ok(self.conn.query_row(&format!("SELECT {MEDIA_COLS} FROM media WHERE path = ?1"), [path], media_from_row).optional()?)
    }

    fn where_clause(q: &MediaQuery) -> (String, Vec<rusqlite::types::Value>) {
        use rusqlite::types::Value;
        let mut conds: Vec<String> = vec![];
        let mut args: Vec<Value> = vec![];
        if !q.include_missing {
            conds.push("missing = 0".into());
        }
        if let Some(k) = q.kind {
            conds.push("kind = ?".into());
            args.push(Value::Text(k.as_str().into()));
        }
        if let Some(t) = q.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            // elk woord moet voorkomen in pad, titel, artiest of album
            for word in t.split_whitespace() {
                conds.push(
                    "(path LIKE ? ESCAPE '\\' OR title LIKE ? ESCAPE '\\' OR artist LIKE ? ESCAPE '\\' OR album LIKE ? ESCAPE '\\')".into(),
                );
                let pat = Value::Text(format!("%{}%", escape_like(word)));
                args.extend([pat.clone(), pat.clone(), pat.clone(), pat]);
            }
        }
        if let Some(c) = &q.container {
            conds.push("container LIKE ? ESCAPE '\\'".into());
            args.push(Value::Text(format!("%{}%", escape_like(c))));
        }
        if let Some(v) = q.min_duration_ms {
            conds.push("duration_ms >= ?".into());
            args.push(Value::Integer(v as i64));
        }
        if let Some(v) = q.max_duration_ms {
            conds.push("duration_ms <= ?".into());
            args.push(Value::Integer(v as i64));
        }
        if let Some(v) = q.min_height {
            conds.push("height >= ?".into());
            args.push(Value::Integer(v as i64));
        }
        if let Some(v) = q.added_since {
            conds.push("added_at >= ?".into());
            args.push(Value::Integer(v));
        }
        if q.favorite_only {
            conds.push("favorite = 1".into());
        }
        if q.played_only {
            conds.push("last_played IS NOT NULL".into());
        }
        for (col, val) in [("artist", &q.artist), ("album", &q.album), ("genre", &q.genre)] {
            if let Some(v) = val {
                conds.push(format!("{col} = ? COLLATE NOCASE"));
                args.push(Value::Text(v.clone()));
            }
        }
        let clause = if conds.is_empty() { String::new() } else { format!(" WHERE {}", conds.join(" AND ")) };
        (clause, args)
    }

    pub fn query_media(&self, q: &MediaQuery) -> Result<Vec<Media>> {
        let (clause, args) = Self::where_clause(q);
        let dir = if q.descending { "DESC" } else { "ASC" };
        let limit = q.limit.unwrap_or(500).min(5000);
        let sql = format!(
            "SELECT {MEDIA_COLS} FROM media{clause} ORDER BY {} {dir}, id ASC LIMIT {limit} OFFSET {}",
            q.sort.column(),
            q.offset.unwrap_or(0)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(args.iter()), media_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn count_media(&self, q: &MediaQuery) -> Result<u64> {
        let (clause, args) = Self::where_clause(q);
        let n: i64 = self.conn.query_row(&format!("SELECT COUNT(*) FROM media{clause}"), params_from_iter(args.iter()), |r| r.get(0))?;
        Ok(n as u64)
    }

    pub fn set_favorite(&self, id: i64, favorite: bool) -> Result<()> {
        let n = self.conn.execute("UPDATE media SET favorite = ?2 WHERE id = ?1", params![id, favorite as i64])?;
        if n == 0 {
            return Err(CoreError::NotFound(format!("media {id}")));
        }
        Ok(())
    }

    pub fn remove_media(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM media WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- bibliotheekmappen, ontbrekende en dubbele bestanden ----------

    pub fn add_library_folder(&self, path: &str) -> Result<i64> {
        self.conn
            .execute("INSERT INTO library_folders (path, added_at) VALUES (?1, ?2) ON CONFLICT(path) DO NOTHING", params![path, now()])?;
        Ok(self.conn.query_row("SELECT id FROM library_folders WHERE path = ?1", [path], |r| r.get(0))?)
    }

    pub fn list_library_folders(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM library_folders ORDER BY path")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<String>>>()?)
    }

    /// Verwijdert een map uit de bibliotheek en de bijbehorende items; geen bestanden op schijf.
    pub fn remove_library_folder(&mut self, path: &str) -> Result<u64> {
        let tx = self.conn.transaction()?;
        let prefix = format!("{}%", escape_like(&with_trailing_sep(path)));
        let removed = tx.execute("DELETE FROM media WHERE path LIKE ?1 ESCAPE '\\'", [prefix])?;
        tx.execute("DELETE FROM library_folders WHERE path = ?1", [path])?;
        tx.commit()?;
        Ok(removed as u64)
    }

    /// Markeert items onder `folder` die niet in `seen` voorkomen als ontbrekend.
    pub fn mark_missing_in_folder(&mut self, folder: &str, seen: &std::collections::HashSet<String>) -> Result<u64> {
        let prefix = format!("{}%", escape_like(&with_trailing_sep(folder)));
        let tx = self.conn.transaction()?;
        let paths: Vec<(i64, String)> = {
            let mut stmt = tx.prepare("SELECT id, path FROM media WHERE path LIKE ?1 ESCAPE '\\' AND missing = 0")?;
            let rows = stmt.query_map([prefix], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut n = 0;
        for (id, p) in paths {
            if !seen.contains(&p) {
                tx.execute("UPDATE media SET missing = 1 WHERE id = ?1", [id])?;
                n += 1;
            }
        }
        tx.commit()?;
        Ok(n)
    }

    /// Koppelt een ontbrekend item aan een nieuw pad; verwijst ook playlist-items mee.
    pub fn relink_media(&mut self, id: i64, new_path: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        let old: String = tx
            .query_row("SELECT path FROM media WHERE id = ?1", [id], |r| r.get(0))
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("media {id}")))?;
        if tx.query_row("SELECT COUNT(*) FROM media WHERE path = ?1 AND id != ?2", params![new_path, id], |r| r.get::<_, i64>(0))? > 0 {
            return Err(CoreError::Invalid("Dit pad staat al in de bibliotheek".into()));
        }
        tx.execute("UPDATE media SET path = ?2, missing = 0 WHERE id = ?1", params![id, new_path])?;
        tx.execute("UPDATE playlist_items SET location = ?2 WHERE location = ?1", params![old, new_path])?;
        tx.execute("UPDATE resume SET location = ?2 WHERE location = ?1", params![old, new_path])?;
        tx.commit()?;
        Ok(())
    }

    /// Groepen bestanden met dezelfde grootte en (afgeronde) duur: waarschijnlijke dubbelen.
    pub fn find_duplicates(&self) -> Result<Vec<Vec<Media>>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {MEDIA_COLS} FROM media
             WHERE missing = 0 AND size > 0 AND (size, duration_ms / 1000) IN (
                 SELECT size, duration_ms / 1000 FROM media WHERE missing = 0 AND size > 0
                 GROUP BY size, duration_ms / 1000 HAVING COUNT(*) > 1)
             ORDER BY size, duration_ms / 1000, path"
        ))?;
        let items: Vec<Media> = stmt.query_map([], media_from_row)?.collect::<rusqlite::Result<_>>()?;
        let mut groups: Vec<Vec<Media>> = vec![];
        for m in items {
            match groups.last_mut() {
                Some(g) if g[0].size == m.size && g[0].duration_ms / 1000 == m.duration_ms / 1000 => g.push(m),
                _ => groups.push(vec![m]),
            }
        }
        Ok(groups)
    }

    // ---------- afspeelgeschiedenis en hervatten ----------

    pub fn history_enabled(&self) -> Result<bool> {
        Ok(self.get_setting("history_enabled")?.map(|v| v != "false").unwrap_or(true))
    }

    /// Legt een afspeelbeurt vast (geschiedenis + teller), tenzij geschiedenis is uitgeschakeld.
    pub fn record_play(&mut self, location: &str, title: Option<&str>) -> Result<()> {
        if !self.history_enabled()? {
            return Ok(());
        }
        let t = now();
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO history (location, title, played_at) VALUES (?1, ?2, ?3)", params![location, title, t])?;
        tx.execute("UPDATE media SET play_count = play_count + 1, last_played = ?2 WHERE path = ?1", params![location, t])?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_history(&self, limit: u32) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare("SELECT id, location, title, played_at FROM history ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit.min(1000)], |r| {
            Ok(HistoryEntry { id: r.get(0)?, location: r.get(1)?, title: r.get(2)?, played_at: r.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_history_entry(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM history WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Wist de geschiedenis en de hervatposities. Bibliotheek en favorieten blijven.
    pub fn clear_history(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM history", [])?;
        tx.execute("DELETE FROM resume", [])?;
        tx.execute("UPDATE media SET last_played = NULL, play_count = 0", [])?;
        tx.commit()?;
        Ok(())
    }

    /// Slaat de positie op. Binnen 5% van het einde (of de laatste 10 s) geldt het item als voltooid.
    pub fn save_resume(&self, location: &str, position_ms: u64, duration_ms: u64) -> Result<()> {
        if !self.history_enabled()? {
            return Ok(());
        }
        let completed =
            duration_ms > 0 && (position_ms >= duration_ms.saturating_sub(duration_ms / 20) || position_ms + 10_000 >= duration_ms);
        let stored_pos = if completed { 0 } else { position_ms };
        self.conn.execute(
            "INSERT INTO resume (location, position_ms, duration_ms, completed, updated_at) VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(location) DO UPDATE SET position_ms=?2, duration_ms=?3, completed=?4, updated_at=?5",
            params![location, stored_pos as i64, duration_ms as i64, completed as i64, now()],
        )?;
        Ok(())
    }

    pub fn get_resume(&self, location: &str) -> Result<Option<ResumePoint>> {
        Ok(self
            .conn
            .query_row(
                "SELECT location, position_ms, duration_ms, completed, updated_at FROM resume WHERE location = ?1",
                [location],
                resume_from_row,
            )
            .optional()?)
    }

    /// "Kijk verder": onvoltooide items met een betekenisvolle positie (> 5 s), nieuwste eerst.
    pub fn continue_watching(&self, limit: u32) -> Result<Vec<ResumePoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT location, position_ms, duration_ms, completed, updated_at FROM resume
             WHERE completed = 0 AND position_ms > 5000 ORDER BY updated_at DESC, rowid DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit.min(200)], resume_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn mark_completed(&self, location: &str, completed: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO resume (location, position_ms, duration_ms, completed, updated_at) VALUES (?1,0,0,?2,?3)
             ON CONFLICT(location) DO UPDATE SET completed=?2, position_ms=0, updated_at=?3",
            params![location, completed as i64, now()],
        )?;
        Ok(())
    }

    // ---------- playlists ----------

    pub fn create_playlist(&self, name: &str) -> Result<i64> {
        let name = clean_name(name)?;
        let t = now();
        self.conn.execute("INSERT INTO playlists (name, created_at, updated_at) VALUES (?1, ?2, ?2)", params![name, t])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_playlist(&self, id: i64, name: &str) -> Result<()> {
        let n =
            self.conn.execute("UPDATE playlists SET name = ?2, updated_at = ?3 WHERE id = ?1", params![id, clean_name(name)?, now()])?;
        if n == 0 {
            return Err(CoreError::NotFound(format!("playlist {id}")));
        }
        Ok(())
    }

    pub fn delete_playlist(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM playlists WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn duplicate_playlist(&mut self, id: i64) -> Result<i64> {
        let tx = self.conn.transaction()?;
        let name: String = tx
            .query_row("SELECT name FROM playlists WHERE id = ?1", [id], |r| r.get(0))
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("playlist {id}")))?;
        let t = now();
        tx.execute("INSERT INTO playlists (name, created_at, updated_at) VALUES (?1, ?2, ?2)", params![format!("{name} (kopie)"), t])?;
        let new_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO playlist_items (playlist_id, position, location, title)
             SELECT ?2, position, location, title FROM playlist_items WHERE playlist_id = ?1",
            params![id, new_id],
        )?;
        tx.commit()?;
        Ok(new_id)
    }

    pub fn list_playlists(&self) -> Result<Vec<Playlist>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, (SELECT COUNT(*) FROM playlist_items i WHERE i.playlist_id = p.id), p.updated_at
             FROM playlists p ORDER BY p.name COLLATE NOCASE, p.id",
        )?;
        let rows = stmt.query_map([], |r| Ok(Playlist { id: r.get(0)?, name: r.get(1)?, item_count: r.get(2)?, updated_at: r.get(3)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn playlist_items(&self, id: i64) -> Result<Vec<PlaylistItem>> {
        let mut stmt =
            self.conn.prepare("SELECT id, position, location, title FROM playlist_items WHERE playlist_id = ?1 ORDER BY position")?;
        let rows =
            stmt.query_map([id], |r| Ok(PlaylistItem { id: r.get(0)?, position: r.get(1)?, location: r.get(2)?, title: r.get(3)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Voegt items achteraan toe (dubbelen toegestaan, zoals in VLC).
    pub fn add_playlist_items(&mut self, id: i64, items: &[(String, Option<String>)]) -> Result<()> {
        let tx = self.conn.transaction()?;
        if tx.query_row("SELECT COUNT(*) FROM playlists WHERE id = ?1", [id], |r| r.get::<_, i64>(0))? == 0 {
            return Err(CoreError::NotFound(format!("playlist {id}")));
        }
        let start: u32 =
            tx.query_row("SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_items WHERE playlist_id = ?1", [id], |r| r.get(0))?;
        for (offset, (loc, title)) in items.iter().enumerate() {
            tx.execute(
                "INSERT INTO playlist_items (playlist_id, position, location, title) VALUES (?1,?2,?3,?4)",
                params![id, start + offset as u32, loc, title],
            )?;
        }
        tx.execute("UPDATE playlists SET updated_at = ?2 WHERE id = ?1", params![id, now()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn remove_playlist_item(&mut self, playlist_id: i64, item_id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM playlist_items WHERE id = ?1 AND playlist_id = ?2", params![item_id, playlist_id])?;
        renumber(&tx, playlist_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Verplaatst een item naar een nieuwe positie (0-gebaseerd).
    pub fn move_playlist_item(&mut self, playlist_id: i64, item_id: i64, new_position: u32) -> Result<()> {
        let tx = self.conn.transaction()?;
        let mut ids: Vec<i64> = {
            let mut stmt = tx.prepare("SELECT id FROM playlist_items WHERE playlist_id = ?1 ORDER BY position")?;
            let rows = stmt.query_map([playlist_id], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let from = ids.iter().position(|i| *i == item_id).ok_or_else(|| CoreError::NotFound(format!("item {item_id}")))?;
        let moved = ids.remove(from);
        ids.insert((new_position as usize).min(ids.len()), moved);
        for (pos, id) in ids.iter().enumerate() {
            tx.execute("UPDATE playlist_items SET position = ?2 WHERE id = ?1", params![id, pos as u32])?;
        }
        tx.commit()?;
        Ok(())
    }

    // ---------- spoorkeuze per bestand ----------

    pub fn get_track_pref(&self, location: &str) -> Result<Option<TrackPref>> {
        Ok(self
            .conn
            .query_row("SELECT audio_key, subtitle_key, subtitle_off FROM track_prefs WHERE location = ?1", [location], |r| {
                Ok(TrackPref { audio_key: r.get(0)?, subtitle_key: r.get(1)?, subtitle_off: r.get::<_, i64>(2)? != 0 })
            })
            .optional()?)
    }

    pub fn save_track_pref(&self, location: &str, pref: &TrackPref) -> Result<()> {
        self.conn.execute(
            "INSERT INTO track_prefs (location, audio_key, subtitle_key, subtitle_off, updated_at) VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(location) DO UPDATE SET audio_key=?2, subtitle_key=?3, subtitle_off=?4, updated_at=?5",
            params![location, pref.audio_key, pref.subtitle_key, i64::from(pref.subtitle_off), now()],
        )?;
        Ok(())
    }

    // ---------- bladwijzers ----------

    pub fn add_bookmark(&self, location: &str, position_ms: u64, label: Option<&str>) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO bookmarks (location, position_ms, label, created_at) VALUES (?1,?2,?3,?4)",
            params![location, position_ms as i64, label, now()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_bookmarks(&self, location: &str) -> Result<Vec<Bookmark>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, location, position_ms, label, created_at FROM bookmarks WHERE location = ?1 ORDER BY position_ms")?;
        let rows = stmt.query_map([location], |r| {
            Ok(Bookmark {
                id: r.get(0)?,
                location: r.get(1)?,
                position_ms: r.get::<_, i64>(2)? as u64,
                label: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_bookmark(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- equalizer-presets ----------

    /// Slaat een eigen preset op (of overschrijft er een met dezelfde naam). Ingebouwde namen zijn gereserveerd.
    pub fn save_eq_preset(&self, name: &str, genre: Option<&str>, cfg: &EqConfig) -> Result<()> {
        let name = clean_name(name)?;
        cfg.validate()?;
        if builtin_presets().iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
            return Err(CoreError::Invalid("Deze naam is gereserveerd voor een ingebouwde preset".into()));
        }
        let bands = serde_json::to_string(&cfg.bands_db).map_err(|e| CoreError::Invalid(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO eq_presets (name, genre, preamp, bands) VALUES (?1,?2,?3,?4)
             ON CONFLICT(name) DO UPDATE SET genre=?2, preamp=?3, bands=?4",
            params![name, genre, cfg.preamp_db as f64, bands],
        )?;
        Ok(())
    }

    pub fn rename_eq_preset(&self, old: &str, new: &str) -> Result<()> {
        let new = clean_name(new)?;
        if builtin_presets().iter().any(|p| p.name.eq_ignore_ascii_case(&new)) {
            return Err(CoreError::Invalid("Deze naam is gereserveerd voor een ingebouwde preset".into()));
        }
        let n = self.conn.execute("UPDATE eq_presets SET name = ?2 WHERE name = ?1", params![old, new]).map_err(|e| match e {
            rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => {
                CoreError::Invalid("Er bestaat al een preset met die naam".into())
            }
            other => other.into(),
        })?;
        if n == 0 {
            return Err(CoreError::NotFound(format!("preset '{old}'")));
        }
        Ok(())
    }

    pub fn delete_eq_preset(&self, name: &str) -> Result<()> {
        self.conn.execute("DELETE FROM eq_presets WHERE name = ?1", [name])?;
        Ok(())
    }

    /// Alleen de eigen presets van de gebruiker.
    pub fn list_user_eq_presets(&self) -> Result<Vec<EqPreset>> {
        let mut stmt = self.conn.prepare("SELECT name, genre, preamp, bands FROM eq_presets ORDER BY name COLLATE NOCASE")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, f64>(2)?, r.get::<_, String>(3)?)))?;
        let mut out = vec![];
        for row in rows {
            let (name, genre, preamp, bands) = row?;
            // een beschadigde rij slaan we over in plaats van de hele lijst te laten falen
            let Ok(bands_db) = serde_json::from_str::<[f32; 10]>(&bands) else {
                continue;
            };
            out.push(EqPreset { name, genre, config: EqConfig { preamp_db: preamp as f32, bands_db }, builtin: false });
        }
        Ok(out)
    }

    /// Ingebouwde presets gevolgd door de eigen presets.
    pub fn all_eq_presets(&self) -> Result<Vec<EqPreset>> {
        let mut all = builtin_presets();
        all.extend(self.list_user_eq_presets()?);
        Ok(all)
    }

    // ---------- slimme collecties ----------

    pub fn create_collection(&self, name: &str, query: &MediaQuery) -> Result<i64> {
        let name = clean_name(name)?;
        let json = serde_json::to_string(query).map_err(|e| CoreError::Invalid(e.to_string()))?;
        self.conn.execute("INSERT INTO collections (name, query, created_at) VALUES (?1, ?2, ?3)", params![name, json, now()]).map_err(
            |e| match e {
                rusqlite::Error::SqliteFailure(f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation => {
                    CoreError::Invalid("Er bestaat al een collectie met die naam".into())
                }
                other => other.into(),
            },
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_collections(&self) -> Result<Vec<(i64, String, MediaQuery)>> {
        let mut stmt = self.conn.prepare("SELECT id, name, query FROM collections ORDER BY name COLLATE NOCASE")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
        let mut out = vec![];
        for row in rows {
            let (id, name, q) = row?;
            out.push((id, name, serde_json::from_str(&q).unwrap_or_default()));
        }
        Ok(out)
    }

    pub fn delete_collection(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM collections WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Voert de opgeslagen zoekregels uit; de uitkomst volgt altijd de actuele bibliotheek.
    pub fn run_collection(&self, id: i64) -> Result<Vec<Media>> {
        let q: String = self
            .conn
            .query_row("SELECT query FROM collections WHERE id = ?1", [id], |r| r.get(0))
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("collectie {id}")))?;
        let query: MediaQuery = serde_json::from_str(&q).map_err(|e| CoreError::Invalid(format!("Beschadigde collectie: {e}")))?;
        self.query_media(&query)
    }

    // ---------- instellingen ----------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute("INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2", params![key, value])?;
        Ok(())
    }

    pub fn all_settings(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

fn resume_from_row(r: &Row) -> rusqlite::Result<ResumePoint> {
    Ok(ResumePoint {
        location: r.get(0)?,
        position_ms: r.get::<_, i64>(1)? as u64,
        duration_ms: r.get::<_, i64>(2)? as u64,
        completed: r.get::<_, i64>(3)? != 0,
        updated_at: r.get(4)?,
    })
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(CoreError::Invalid("Een naam mag niet leeg zijn".into()));
    }
    if n.chars().count() > 120 {
        return Err(CoreError::Invalid("De naam is te lang (maximaal 120 tekens)".into()));
    }
    Ok(n.to_string())
}

fn renumber(tx: &rusqlite::Transaction, playlist_id: i64) -> Result<()> {
    let ids: Vec<i64> = {
        let mut stmt = tx.prepare("SELECT id FROM playlist_items WHERE playlist_id = ?1 ORDER BY position")?;
        let rows = stmt.query_map([playlist_id], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for (pos, id) in ids.iter().enumerate() {
        tx.execute("UPDATE playlist_items SET position = ?2 WHERE id = ?1", params![id, pos as u32])?;
    }
    Ok(())
}

fn with_trailing_sep(p: &str) -> String {
    if p.ends_with('/') || p.ends_with('\\') {
        p.to_string()
    } else if p.contains('\\') && !p.contains('/') {
        format!("{p}\\")
    } else {
        format!("{p}/")
    }
}

#[cfg(test)]
mod tests;
