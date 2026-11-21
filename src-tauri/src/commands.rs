use chrono::{DateTime, Duration as ChronoDuration, Utc};
use tauri::State;

use crate::api::{self, hardcover, igdb, tmdb, tvmaze};
use crate::db;
use crate::error::{err, AppResult};
use crate::models::*;
use crate::series;
use crate::AppState;

fn check_category(c: &str) -> AppResult<()> {
    match c {
        "movie" | "tv" | "book" | "game" => Ok(()),
        _ => err(format!("Unknown category '{c}'")),
    }
}

fn check_section(s: &str) -> AppResult<()> {
    match s {
        "active" | "done" => Ok(()),
        _ => err(format!("Unknown section '{s}'")),
    }
}

fn clean(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

async fn fetch_details(state: &AppState, category: &str, source: &str, external_id: &str) -> AppResult<ItemData> {
    let settings = state.with_db(db::load_settings)?;
    match (category, source) {
        ("movie", "tmdb") => tmdb::details(&state.http, &settings.tmdb_key, external_id).await,
        ("tv", "tvmaze") => tvmaze::details(&state.http, external_id).await,
        ("book", "hardcover") => hardcover::details(&state.http, &settings.hardcover_token, external_id).await,
        ("game", "igdb") => igdb::details(state, &settings, external_id).await,
        _ => err(format!("No online source available for {category}/{source}")),
    }
}

async fn refresh_one(state: &AppState, id: i64) -> AppResult<Item> {
    let item = state.with_db(|c| db::get_item(c, id))?;
    let Some(ext) = item.external_id.clone().filter(|_| item.source != "manual") else {
        return err("Manually created items have no online source to refresh from.");
    };
    let data = fetch_details(state, &item.category, &item.source, &ext).await?;
    state.with_db(|c| {
        db::update_item_data(c, id, &data)?;
        if let Some(eps) = &data.episodes {
            let added = db::sync_episodes(c, id, eps)?;
            db::auto_tv_section(c, id, added > 0)?;
        }
        if item.category == "book" {
            series::auto_assign(c, id)?;
        }
        db::get_item(c, id)
    })
}

/// After inserting a book: attach to the requested series, or auto-detect one.
fn place_book(c: &rusqlite::Connection, id: i64, series_id: Option<i64>) -> AppResult<()> {
    match series_id {
        Some(sid) => db::set_item_series(c, id, Some(sid)),
        None => series::auto_assign(c, id).map(|_| ()),
    }
}

// ---------------------------------------------------------------- settings / info

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo { db_path: state.db_path.clone(), version: env!("CARGO_PKG_VERSION").into() }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    state.with_db(db::load_settings)
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> AppResult<()> {
    state.with_db(|c| db::save_settings(c, &settings))?;
    if let Ok(mut t) = state.igdb_token.lock() {
        *t = None;
    }
    Ok(())
}

// ---------------------------------------------------------------- library

#[tauri::command]
pub fn list_library(state: State<'_, AppState>, category: String) -> AppResult<LibraryData> {
    check_category(&category)?;
    state.with_db(|c| {
        Ok(LibraryData {
            items: db::list_items(c, &category, true)?,
            series: db::list_series(c, &category)?,
        })
    })
}

#[tauri::command]
pub fn get_item(state: State<'_, AppState>, id: i64) -> AppResult<Item> {
    state.with_db(|c| db::get_item(c, id))
}

#[tauri::command]
pub async fn search(state: State<'_, AppState>, category: String, query: String) -> AppResult<Vec<SearchResult>> {
    check_category(&category)?;
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(vec![]);
    }
    let settings = state.with_db(db::load_settings)?;
    let mut results = match category.as_str() {
        "movie" => tmdb::search(&state.http, &settings.tmdb_key, &query).await?,
        "tv" => tvmaze::search(&state.http, &query).await?,
        "book" => hardcover::search(&state.http, &settings.hardcover_token, &query).await?,
        _ => igdb::search(&state, &settings, &query).await?,
    };
    state.with_db(|c| {
        for r in results.iter_mut() {
            r.in_library = db::find_existing(c, &category, &r.source, &r.external_id)?.is_some();
        }
        Ok(())
    })?;
    Ok(results)
}

#[tauri::command]
pub async fn add_from_search(
    state: State<'_, AppState>,
    category: String,
    source: String,
    external_id: String,
    section: String,
    series_id: Option<i64>,
) -> AppResult<Item> {
    check_category(&category)?;
    check_section(&section)?;
    if let Some(id) = state.with_db(|c| db::find_existing(c, &category, &source, &external_id))? {
        // Already in the library: if adding into a series, just attach it.
        return state.with_db(|c| {
            if category == "book" {
                if let Some(sid) = series_id {
                    db::set_item_series(c, id, Some(sid))?;
                }
            }
            db::get_item(c, id)
        });
    }
    let data = fetch_details(&state, &category, &source, &external_id).await?;
    state.with_db(|c| {
        let id = db::insert_item(c, &category, &section, &source, Some(external_id.as_str()), &data)?;
        if let Some(eps) = &data.episodes {
            db::sync_episodes(c, id, eps)?;
        }
        if category == "tv" && section == "done" {
            db::set_watched_bulk(c, id, None, true)?;
        }
        if category == "book" {
            place_book(c, id, series_id)?;
        }
        db::get_item(c, id)
    })
}

#[tauri::command]
pub fn add_manual(state: State<'_, AppState>, input: ManualItemInput) -> AppResult<Item> {
    check_category(&input.category)?;
    check_section(&input.section)?;
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return err("Title is required.");
    }
    let release_date = clean(input.release_date);
    let status = if input.category == "tv" { clean(input.status) } else { None };
    let mut facts = Vec::new();
    if let Some(d) = &release_date {
        facts.push(api::fact("Released", d.clone()));
    }
    if let Some(s) = &status {
        facts.push(api::fact("Status", s.clone()));
    }
    let data = ItemData {
        title,
        subtitle: clean(input.subtitle),
        summary: clean(input.summary),
        image_url: clean(input.image_url),
        release_date,
        status,
        extra: api::extra(facts, vec![]),
        episodes: None,
    };
    state.with_db(|c| {
        let id = db::insert_item(c, &input.category, &input.section, "manual", None, &data)?;
        if input.category == "book" {
            place_book(c, id, input.series_id)?;
        }
        db::get_item(c, id)
    })
}

#[tauri::command]
pub fn remove_item(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_db(|c| db::delete_item(c, id))
}

#[tauri::command]
pub fn set_section(state: State<'_, AppState>, id: i64, section: String) -> AppResult<Item> {
    check_section(&section)?;
    state.with_db(|c| {
        db::set_section(c, id, &section)?;
        db::get_item(c, id)
    })
}

#[tauri::command]
pub fn set_rating(state: State<'_, AppState>, id: i64, rating: i64) -> AppResult<Item> {
    let rating = rating.clamp(0, 10);
    state.with_db(|c| {
        db::set_rating(c, id, rating)?;
        db::get_item(c, id)
    })
}

#[tauri::command]
pub async fn refresh_item(state: State<'_, AppState>, id: i64) -> AppResult<Item> {
    refresh_one(&state, id).await
}

// ---------------------------------------------------------------- series (books)

#[tauri::command]
pub fn get_series(state: State<'_, AppState>, id: i64) -> AppResult<SeriesDetail> {
    state.with_db(|c| db::get_series(c, id))
}

#[tauri::command]
pub fn list_series(state: State<'_, AppState>, category: String) -> AppResult<Vec<SeriesSummary>> {
    check_category(&category)?;
    state.with_db(|c| db::list_series(c, &category))
}

#[tauri::command]
pub fn create_series(state: State<'_, AppState>, category: String, title: String, item_ids: Vec<i64>) -> AppResult<SeriesSummary> {
    check_category(&category)?;
    state.with_db(|c| {
        let sid = db::create_series(c, &category, &title)?;
        for id in &item_ids {
            db::set_item_series(c, *id, Some(sid))?;
        }
        Ok(db::get_series(c, sid)?.series)
    })
}

#[tauri::command]
pub fn rename_series(state: State<'_, AppState>, id: i64, title: String) -> AppResult<SeriesSummary> {
    state.with_db(|c| {
        db::rename_series(c, id, &title)?;
        Ok(db::get_series(c, id)?.series)
    })
}

#[tauri::command]
pub fn add_to_series(state: State<'_, AppState>, series_id: i64, item_ids: Vec<i64>) -> AppResult<SeriesSummary> {
    state.with_db(|c| {
        let cat = db::get_series(c, series_id)?.series.category;
        for id in &item_ids {
            let it = db::get_item(c, *id)?;
            if it.category != cat {
                return err(format!("'{}' is not a {}", it.title, cat));
            }
            db::set_item_series(c, *id, Some(series_id))?;
        }
        db::cleanup_empty_series(c)?;
        Ok(db::get_series(c, series_id)?.series)
    })
}

#[tauri::command]
pub fn remove_from_series(state: State<'_, AppState>, item_id: i64) -> AppResult<Item> {
    state.with_db(|c| {
        db::set_item_series(c, item_id, None)?;
        db::cleanup_empty_series(c)?;
        db::get_item(c, item_id)
    })
}

/// Dissolve the series; its books return to the library as individual entries.
#[tauri::command]
pub fn ungroup_series(state: State<'_, AppState>, series_id: i64) -> AppResult<()> {
    state.with_db(|c| db::delete_series(c, series_id, false))
}

/// Delete the series AND all of its books.
#[tauri::command]
pub fn delete_series(state: State<'_, AppState>, series_id: i64) -> AppResult<()> {
    state.with_db(|c| db::delete_series(c, series_id, true))
}

#[tauri::command]
pub fn set_series_read(state: State<'_, AppState>, series_id: i64, read: bool) -> AppResult<SeriesSummary> {
    state.with_db(|c| {
        let section = if read { "done" } else { "active" };
        for b in db::series_books(c, series_id)? {
            db::set_section(c, b.id, section)?;
        }
        Ok(db::get_series(c, series_id)?.series)
    })
}

#[tauri::command]
pub fn auto_group_books(state: State<'_, AppState>) -> AppResult<usize> {
    state.with_db(series::auto_group_all)
}

// ---------------------------------------------------------------- TV

#[tauri::command]
pub fn get_episodes(state: State<'_, AppState>, item_id: i64) -> AppResult<Vec<Episode>> {
    state.with_db(|c| db::list_episodes(c, item_id))
}

#[tauri::command]
pub fn set_episode_watched(state: State<'_, AppState>, episode_id: i64, watched: bool) -> AppResult<Item> {
    state.with_db(|c| {
        let item_id = db::set_episode_watched(c, episode_id, watched)?;
        db::auto_tv_section(c, item_id, !watched)?;
        db::get_item(c, item_id)
    })
}

#[tauri::command]
pub fn set_season_watched(state: State<'_, AppState>, item_id: i64, season: i64, watched: bool) -> AppResult<Item> {
    state.with_db(|c| {
        db::set_watched_bulk(c, item_id, Some(season), watched)?;
        db::auto_tv_section(c, item_id, !watched)?;
        db::get_item(c, item_id)
    })
}

#[tauri::command]
pub fn mark_all_watched(state: State<'_, AppState>, item_id: i64) -> AppResult<Item> {
    state.with_db(|c| {
        db::set_watched_bulk(c, item_id, None, true)?;
        db::auto_tv_section(c, item_id, false)?;
        db::get_item(c, item_id)
    })
}

#[tauri::command]
pub fn add_manual_episodes(state: State<'_, AppState>, item_id: i64, season: i64, count: i64) -> AppResult<()> {
    if !(1..=500).contains(&count) {
        return err("Episode count must be between 1 and 500.");
    }
    if season < 0 {
        return err("Season must be 0 or higher.");
    }
    state.with_db(|c| {
        let item = db::get_item(c, item_id)?;
        if item.category != "tv" || item.source != "manual" {
            return err("Episodes can only be added manually to manually created shows.");
        }
        db::add_manual_episodes(c, item_id, season, count)
    })
}

#[tauri::command]
pub fn watch_next(state: State<'_, AppState>) -> AppResult<Vec<EpisodeWithShow>> {
    state.with_db(db::watch_next)
}

#[tauri::command]
pub fn upcoming(state: State<'_, AppState>) -> AppResult<Vec<EpisodeWithShow>> {
    state.with_db(db::upcoming)
}

#[tauri::command]
pub async fn refresh_all_tv(state: State<'_, AppState>, stale_hours: Option<i64>) -> AppResult<RefreshSummary> {
    let items = state.with_db(|c| db::list_items_raw(c, "tv", Some("active"), false))?;
    let cutoff = stale_hours.map(|h| Utc::now() - ChronoDuration::hours(h));
    let mut summary = RefreshSummary { refreshed: 0, failed: 0, errors: vec![] };
    for it in items.into_iter().filter(|i| i.source == "tvmaze") {
        if let (Some(cut), Some(last)) = (cutoff, it.last_refreshed.as_deref()) {
            if let Ok(t) = DateTime::parse_from_rfc3339(last) {
                if t.with_timezone(&Utc) > cut {
                    continue;
                }
            }
        }
        match refresh_one(&state, it.id).await {
            Ok(_) => summary.refreshed += 1,
            Err(e) => {
                summary.failed += 1;
                summary.errors.push(format!("{}: {}", it.title, e));
            }
        }
    }
    Ok(summary)
}