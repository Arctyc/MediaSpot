use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, Local, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::{
    Episode, EpisodeData, EpisodeWithShow, Item, ItemData, SeriesDetail, SeriesSummary, Settings, TvProgress,
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    category       TEXT NOT NULL CHECK (category IN ('movie','tv','book','game')),
    section        TEXT NOT NULL CHECK (section IN ('active','done')),
    source         TEXT NOT NULL,
    external_id    TEXT,
    title          TEXT NOT NULL,
    subtitle       TEXT,
    summary        TEXT,
    image_url      TEXT,
    release_date   TEXT,
    status         TEXT,
    extra          TEXT,
    rating         INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 10),
    added_at       TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    last_refreshed TEXT,
    UNIQUE (category, source, external_id)
);
CREATE INDEX IF NOT EXISTS idx_items_cat_section ON items (category, section);

CREATE TABLE IF NOT EXISTS episodes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id     INTEGER NOT NULL REFERENCES items (id) ON DELETE CASCADE,
    external_id TEXT,
    season      INTEGER NOT NULL,
    number      INTEGER,
    name        TEXT,
    airdate     TEXT,
    airstamp    TEXT,
    runtime     INTEGER,
    summary     TEXT,
    image_url   TEXT,
    watched     INTEGER NOT NULL DEFAULT 0,
    watched_at  TEXT,
    UNIQUE (item_id, external_id)
);
CREATE INDEX IF NOT EXISTS idx_episodes_item ON episodes (item_id, season, number);
"#;

const SCHEMA_V2: &str = r#"
CREATE TABLE IF NOT EXISTS series (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    category   TEXT NOT NULL,
    title      TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_series_category ON series (category);
"#;

pub fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn not_found() -> AppError {
    AppError::Msg("Item not found".into())
}

fn has_column(conn: &Connection, table: &str, col: &str) -> AppResult<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let cols = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cols.iter().any(|c| c == col))
}

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(SCHEMA)?;

    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 2 {
        conn.execute_batch(SCHEMA_V2)?;
        if !has_column(&conn, "items", "series_id")? {
            conn.execute_batch("ALTER TABLE items ADD COLUMN series_id INTEGER REFERENCES series (id) ON DELETE SET NULL;")?;
        }
        if !has_column(&conn, "items", "series_position")? {
            conn.execute_batch("ALTER TABLE items ADD COLUMN series_position REAL;")?;
        }
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_items_series ON items (series_id); PRAGMA user_version = 2;")?;
    }
    Ok(conn)
}

// ---------------------------------------------------------------- settings

pub fn load_settings(conn: &Connection) -> AppResult<Settings> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let map = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<HashMap<String, String>, _>>()?;
    let get = |k: &str| map.get(k).cloned().unwrap_or_default();
    Ok(Settings {
        tmdb_key: get("tmdb_key"),
        igdb_client_id: get("igdb_client_id"),
        igdb_client_secret: get("igdb_client_secret"),
        hardcover_token: get("hardcover_token"),
    })
}

pub fn save_settings(conn: &Connection, s: &Settings) -> AppResult<()> {
    let pairs = [
        ("tmdb_key", &s.tmdb_key),
        ("igdb_client_id", &s.igdb_client_id),
        ("igdb_client_secret", &s.igdb_client_secret),
        ("hardcover_token", &s.hardcover_token),
    ];
    for (k, v) in pairs {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![k, v.trim()],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------- items

const ITEM_SELECT: &str = "SELECT i.id, i.category, i.section, i.source, i.external_id, i.title, i.subtitle, \
    i.summary, i.image_url, i.release_date, i.status, i.extra, i.rating, i.added_at, i.updated_at, \
    i.last_refreshed, i.series_id, s.title, i.series_position \
    FROM items i LEFT JOIN series s ON s.id = i.series_id";

fn row_to_item(r: &Row) -> rusqlite::Result<Item> {
    let extra: Option<String> = r.get(11)?;
    Ok(Item {
        id: r.get(0)?,
        category: r.get(1)?,
        section: r.get(2)?,
        source: r.get(3)?,
        external_id: r.get(4)?,
        title: r.get(5)?,
        subtitle: r.get(6)?,
        summary: r.get(7)?,
        image_url: r.get(8)?,
        release_date: r.get(9)?,
        status: r.get(10)?,
        extra: extra
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null),
        rating: r.get(12)?,
        added_at: r.get(13)?,
        updated_at: r.get(14)?,
        last_refreshed: r.get(15)?,
        series_id: r.get(16)?,
        series_title: r.get(17)?,
        series_position: r.get(18)?,
        tv: None,
    })
}

fn attach_tv(conn: &Connection, item: &mut Item) -> AppResult<()> {
    if item.category == "tv" {
        let eps = list_episodes_for(conn, item.id, item.source == "manual")?;
        item.tv = Some(compute_progress(&eps, item.status.as_deref()));
    }
    Ok(())
}

pub fn get_item(conn: &Connection, id: i64) -> AppResult<Item> {
    let mut item = conn
        .query_row(&format!("{ITEM_SELECT} WHERE i.id = ?1"), [id], row_to_item)
        .optional()?
        .ok_or_else(not_found)?;
    attach_tv(conn, &mut item)?;
    Ok(item)
}

/// Items of a category. `only_ungrouped` excludes items that belong to a series.
pub fn list_items_raw(conn: &Connection, category: &str, section: Option<&str>, only_ungrouped: bool) -> AppResult<Vec<Item>> {
    let sql = format!(
        "{ITEM_SELECT} WHERE i.category = ?1 AND (?2 IS NULL OR i.section = ?2)
           AND (?3 = 0 OR i.series_id IS NULL)
         ORDER BY i.added_at DESC, i.id DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params![category, section, only_ungrouped as i64], row_to_item)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

pub fn list_items(conn: &Connection, category: &str, only_ungrouped: bool) -> AppResult<Vec<Item>> {
    let mut items = list_items_raw(conn, category, None, only_ungrouped)?;
    for it in items.iter_mut() {
        attach_tv(conn, it)?;
    }
    Ok(items)
}

pub fn find_existing(conn: &Connection, category: &str, source: &str, external_id: &str) -> AppResult<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM items WHERE category = ?1 AND source = ?2 AND external_id = ?3",
            params![category, source, external_id],
            |r| r.get(0),
        )
        .optional()?)
}

fn series_position_of(d: &ItemData) -> Option<f64> {
    d.extra.get("seriesPosition").and_then(|v| v.as_f64())
}

pub fn insert_item(
    conn: &Connection,
    category: &str,
    section: &str,
    source: &str,
    external_id: Option<&str>,
    d: &ItemData,
) -> AppResult<i64> {
    let now = now_str();
    let refreshed = if source == "manual" { None } else { Some(now.clone()) };
    conn.execute(
        "INSERT INTO items (category, section, source, external_id, title, subtitle, summary,
                            image_url, release_date, status, extra, rating, added_at, updated_at,
                            last_refreshed, series_position)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?12, ?13, ?14)",
        params![
            category, section, source, external_id, d.title, d.subtitle, d.summary,
            d.image_url, d.release_date, d.status, d.extra.to_string(), now, refreshed,
            series_position_of(d)
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_item_data(conn: &Connection, id: i64, d: &ItemData) -> AppResult<()> {
    let now = now_str();
    conn.execute(
        "UPDATE items SET title = ?1, subtitle = ?2, summary = ?3, image_url = ?4, release_date = ?5,
                status = ?6, extra = ?7, updated_at = ?8, last_refreshed = ?8,
                series_position = COALESCE(?10, series_position)
         WHERE id = ?9",
        params![
            d.title, d.subtitle, d.summary, d.image_url, d.release_date, d.status,
            d.extra.to_string(), now, id, series_position_of(d)
        ],
    )?;
    Ok(())
}

pub fn set_section(conn: &Connection, id: i64, section: &str) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE items SET section = ?1, updated_at = ?2 WHERE id = ?3",
        params![section, now_str(), id],
    )?;
    if n == 0 {
        return Err(not_found());
    }
    Ok(())
}

pub fn set_rating(conn: &Connection, id: i64, rating: i64) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE items SET rating = ?1, updated_at = ?2 WHERE id = ?3",
        params![rating, now_str(), id],
    )?;
    if n == 0 {
        return Err(not_found());
    }
    Ok(())
}

pub fn delete_item(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM items WHERE id = ?1", [id])?;
    cleanup_empty_series(conn)?;
    Ok(())
}

// ---------------------------------------------------------------- series

pub fn create_series(conn: &Connection, category: &str, title: &str) -> AppResult<i64> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Msg("Series name is required".into()));
    }
    let now = now_str();
    conn.execute(
        "INSERT INTO series (category, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![category, title, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn find_series_by_title(conn: &Connection, category: &str, title: &str) -> AppResult<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM series WHERE category = ?1 AND LOWER(title) = LOWER(?2)",
            params![category, title.trim()],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn rename_series(conn: &Connection, id: i64, title: &str) -> AppResult<()> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Msg("Series name is required".into()));
    }
    let n = conn.execute(
        "UPDATE series SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now_str(), id],
    )?;
    if n == 0 {
        return Err(AppError::Msg("Series not found".into()));
    }
    Ok(())
}

pub fn set_item_series(conn: &Connection, item_id: i64, series_id: Option<i64>) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE items SET series_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![series_id, now_str(), item_id],
    )?;
    if n == 0 {
        return Err(not_found());
    }
    Ok(())
}

pub fn delete_series(conn: &Connection, id: i64, delete_books: bool) -> AppResult<()> {
    if delete_books {
        conn.execute("DELETE FROM items WHERE series_id = ?1", [id])?;
    } else {
        conn.execute("UPDATE items SET series_id = NULL WHERE series_id = ?1", [id])?;
    }
    conn.execute("DELETE FROM series WHERE id = ?1", [id])?;
    Ok(())
}

/// Series with zero books are meaningless; drop them.
pub fn cleanup_empty_series(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "DELETE FROM series WHERE id NOT IN (SELECT DISTINCT series_id FROM items WHERE series_id IS NOT NULL)",
        [],
    )?;
    Ok(())
}

pub fn series_books(conn: &Connection, series_id: i64) -> AppResult<Vec<Item>> {
    let sql = format!(
        "{ITEM_SELECT} WHERE i.series_id = ?1
         ORDER BY (i.series_position IS NULL), i.series_position, i.release_date, i.title COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt.query_map([series_id], row_to_item)?.collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

fn summarize(id: i64, category: String, title: String, created_at: String, books: &[Item]) -> SeriesSummary {
    let book_count = books.len() as i64;
    let read_count = books.iter().filter(|b| b.section == "done").count() as i64;
    let mut seen = HashSet::new();
    let authors: Vec<String> = books
        .iter()
        .filter_map(|b| b.subtitle.clone())
        .filter(|a| seen.insert(a.to_lowercase()))
        .take(2)
        .collect();
    SeriesSummary {
        id,
        category,
        title,
        section: if book_count > 0 && read_count == book_count { "done".into() } else { "active".into() },
        image_url: books.iter().find_map(|b| b.image_url.clone()),
        book_count,
        read_count,
        authors: if authors.is_empty() { None } else { Some(authors.join("; ")) },
        created_at,
    }
}

pub fn get_series(conn: &Connection, id: i64) -> AppResult<SeriesDetail> {
    let (category, title, created_at): (String, String, String) = conn
        .query_row(
            "SELECT category, title, created_at FROM series WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Msg("Series not found".into()))?;
    let books = series_books(conn, id)?;
    Ok(SeriesDetail { series: summarize(id, category, title, created_at, &books), books })
}

pub fn list_series(conn: &Connection, category: &str) -> AppResult<Vec<SeriesSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, category, title, created_at FROM series WHERE category = ?1 ORDER BY title COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map([category], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::new();
    for (id, cat, title, created) in rows {
        let books = series_books(conn, id)?;
        out.push(summarize(id, cat, title, created, &books));
    }
    Ok(out)
}

// ---------------------------------------------------------------- episodes

const EP_COLS: &str = "id, item_id, external_id, season, number, name, airdate, airstamp, runtime, \
    summary, image_url, watched, watched_at";

pub fn is_aired(airstamp: Option<&str>, airdate: Option<&str>, no_date_aired: bool, now: DateTime<Utc>) -> bool {
    if let Some(s) = airstamp {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return dt.with_timezone(&Utc) <= now;
        }
    }
    if let Some(d) = airdate {
        if let Ok(nd) = NaiveDate::parse_from_str(d, "%Y-%m-%d") {
            return nd <= Local::now().date_naive();
        }
    }
    no_date_aired
}

fn row_to_episode(r: &Row, no_date_aired: bool, now: DateTime<Utc>) -> rusqlite::Result<Episode> {
    let airdate: Option<String> = r.get(6)?;
    let airstamp: Option<String> = r.get(7)?;
    let aired = is_aired(airstamp.as_deref(), airdate.as_deref(), no_date_aired, now);
    Ok(Episode {
        id: r.get(0)?,
        item_id: r.get(1)?,
        external_id: r.get(2)?,
        season: r.get(3)?,
        number: r.get(4)?,
        name: r.get(5)?,
        airdate,
        airstamp,
        runtime: r.get(8)?,
        summary: r.get(9)?,
        image_url: r.get(10)?,
        watched: r.get::<_, i64>(11)? != 0,
        watched_at: r.get(12)?,
        aired,
    })
}

pub fn list_episodes_for(conn: &Connection, item_id: i64, no_date_aired: bool) -> AppResult<Vec<Episode>> {
    let now = Utc::now();
    let mut stmt = conn.prepare(&format!(
        "SELECT {EP_COLS} FROM episodes WHERE item_id = ?1 ORDER BY season ASC, number ASC, id ASC"
    ))?;
    let eps = stmt
        .query_map([item_id], |r| row_to_episode(r, no_date_aired, now))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(eps)
}

pub fn list_episodes(conn: &Connection, item_id: i64) -> AppResult<Vec<Episode>> {
    let source: String = conn
        .query_row("SELECT source FROM items WHERE id = ?1", [item_id], |r| r.get(0))
        .optional()?
        .ok_or_else(not_found)?;
    list_episodes_for(conn, item_id, source == "manual")
}

pub fn compute_progress(eps: &[Episode], status: Option<&str>) -> TvProgress {
    let total = eps.len() as i64;
    let aired = eps.iter().filter(|e| e.aired).count() as i64;
    let watched_aired = eps.iter().filter(|e| e.aired && e.watched).count() as i64;
    let unaired = total - aired;
    let series_ended = status.map(|s| s.eq_ignore_ascii_case("ended")).unwrap_or(false);
    let caught_up = aired > 0 && watched_aired == aired;
    let fully_complete = caught_up && unaired == 0 && series_ended;
    let next_airdate = eps
        .iter()
        .find(|e| !e.aired)
        .and_then(|e| e.airstamp.clone().or_else(|| e.airdate.clone()));
    TvProgress { total, aired, watched_aired, unaired, series_ended, caught_up, fully_complete, next_airdate }
}

pub fn sync_episodes(conn: &Connection, item_id: i64, eps: &[EpisodeData]) -> AppResult<usize> {
    let tx = conn.unchecked_transaction()?;
    let existing: HashSet<String> = {
        let mut stmt =
            tx.prepare("SELECT external_id FROM episodes WHERE item_id = ?1 AND external_id IS NOT NULL")?;
        let rows = stmt
            .query_map([item_id], |r| r.get::<_, String>(0))?
            .collect::<Result<HashSet<_>, _>>()?;
        rows
    };
    let mut added = 0;
    for e in eps {
        if existing.contains(&e.external_id) {
            tx.execute(
                "UPDATE episodes SET season = ?1, number = ?2, name = ?3, airdate = ?4, airstamp = ?5,
                        runtime = ?6, summary = ?7, image_url = ?8
                 WHERE item_id = ?9 AND external_id = ?10",
                params![e.season, e.number, e.name, e.airdate, e.airstamp, e.runtime, e.summary, e.image_url, item_id, e.external_id],
            )?;
        } else {
            tx.execute(
                "INSERT INTO episodes (item_id, external_id, season, number, name, airdate, airstamp, runtime, summary, image_url)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![item_id, e.external_id, e.season, e.number, e.name, e.airdate, e.airstamp, e.runtime, e.summary, e.image_url],
            )?;
            added += 1;
        }
    }
    if !eps.is_empty() {
        let incoming: HashSet<&str> = eps.iter().map(|e| e.external_id.as_str()).collect();
        for old in existing.iter().filter(|x| !incoming.contains(x.as_str())) {
            tx.execute("DELETE FROM episodes WHERE item_id = ?1 AND external_id = ?2", params![item_id, old])?;
        }
    }
    tx.commit()?;
    Ok(added)
}

pub fn set_episode_watched(conn: &Connection, episode_id: i64, watched: bool) -> AppResult<i64> {
    let item_id: i64 = conn
        .query_row("SELECT item_id FROM episodes WHERE id = ?1", [episode_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::Msg("Episode not found".into()))?;
    let at = if watched { Some(now_str()) } else { None };
    conn.execute(
        "UPDATE episodes SET watched = ?1, watched_at = ?2 WHERE id = ?3",
        params![watched as i64, at, episode_id],
    )?;
    Ok(item_id)
}

pub fn set_watched_bulk(conn: &Connection, item_id: i64, season: Option<i64>, watched: bool) -> AppResult<()> {
    let eps = list_episodes(conn, item_id)?;
    let now = now_str();
    let tx = conn.unchecked_transaction()?;
    for e in eps.iter().filter(|e| season.map_or(true, |s| e.season == s)) {
        if watched && e.aired && !e.watched {
            tx.execute("UPDATE episodes SET watched = 1, watched_at = ?1 WHERE id = ?2", params![now, e.id])?;
        } else if !watched && e.watched {
            tx.execute("UPDATE episodes SET watched = 0, watched_at = NULL WHERE id = ?1", [e.id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn add_manual_episodes(conn: &Connection, item_id: i64, season: i64, count: i64) -> AppResult<()> {
    let max: Option<i64> = conn.query_row(
        "SELECT MAX(number) FROM episodes WHERE item_id = ?1 AND season = ?2",
        params![item_id, season],
        |r| r.get(0),
    )?;
    let start = max.unwrap_or(0) + 1;
    let tx = conn.unchecked_transaction()?;
    for n in start..start + count {
        tx.execute(
            "INSERT INTO episodes (item_id, season, number, name) VALUES (?1, ?2, ?3, ?4)",
            params![item_id, season, n, format!("Episode {n}")],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn auto_tv_section(conn: &Connection, item_id: i64, allow_reopen: bool) -> AppResult<()> {
    let item = get_item(conn, item_id)?;
    let Some(p) = item.tv.as_ref() else { return Ok(()) };
    if item.section == "active" && p.fully_complete {
        set_section(conn, item_id, "done")?;
    } else if item.section == "done" && allow_reopen && !p.fully_complete {
        set_section(conn, item_id, "active")?;
    }
    Ok(())
}

// ---------------------------------------------------------------- TV feeds

fn air_key(e: &Episode) -> String {
    e.airstamp.clone().or_else(|| e.airdate.clone()).unwrap_or_default()
}

pub fn watch_next(conn: &Connection) -> AppResult<Vec<EpisodeWithShow>> {
    let mut out = Vec::new();
    for it in list_items_raw(conn, "tv", Some("active"), false)? {
        let eps = list_episodes_for(conn, it.id, it.source == "manual")?;
        let unwatched: Vec<&Episode> = eps.iter().filter(|e| e.aired && !e.watched).collect();
        if let Some(first) = unwatched.first() {
            out.push(EpisodeWithShow {
                episode: (*first).clone(),
                show_id: it.id,
                show_title: it.title.clone(),
                show_image: it.image_url.clone(),
                remaining: unwatched.len() as i64 - 1,
            });
        }
    }
    out.sort_by(|a, b| air_key(&b.episode).cmp(&air_key(&a.episode)));
    Ok(out)
}

pub fn upcoming(conn: &Connection) -> AppResult<Vec<EpisodeWithShow>> {
    let mut out = Vec::new();
    for it in list_items_raw(conn, "tv", Some("active"), false)? {
        let eps = list_episodes_for(conn, it.id, it.source == "manual")?;
        let future: Vec<&Episode> = eps.iter().filter(|e| !e.aired).collect();
        if let Some(first) = future.first() {
            out.push(EpisodeWithShow {
                episode: (*first).clone(),
                show_id: it.id,
                show_title: it.title.clone(),
                show_image: it.image_url.clone(),
                remaining: future.len() as i64 - 1,
            });
        }
    }
    out.sort_by(|a, b| {
        let (ka, kb) = (air_key(&a.episode), air_key(&b.episode));
        match (ka.is_empty(), kb.is_empty()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => ka.cmp(&kb),
        }
    });
    Ok(out)
}