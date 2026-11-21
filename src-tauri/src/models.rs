use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: i64,
    pub category: String,
    pub section: String,
    pub source: String,
    pub external_id: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub release_date: Option<String>,
    pub status: Option<String>,
    pub extra: Value, // { facts, genres, links, seriesName, seriesPosition }
    pub rating: i64,
    pub added_at: String,
    pub updated_at: String,
    pub last_refreshed: Option<String>,
    pub series_id: Option<i64>,
    pub series_title: Option<String>,
    pub series_position: Option<f64>,
    pub tv: Option<TvProgress>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TvProgress {
    pub total: i64,
    pub aired: i64,
    pub watched_aired: i64,
    pub unaired: i64,
    pub series_ended: bool,
    pub caught_up: bool,
    pub fully_complete: bool,
    pub next_airdate: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: i64,
    pub item_id: i64,
    pub external_id: Option<String>,
    pub season: i64,
    pub number: Option<i64>,
    pub name: Option<String>,
    pub airdate: Option<String>,
    pub airstamp: Option<String>,
    pub runtime: Option<i64>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub watched: bool,
    pub watched_at: Option<String>,
    pub aired: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeWithShow {
    pub episode: Episode,
    pub show_id: i64,
    pub show_title: String,
    pub show_image: Option<String>,
    pub remaining: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub source: String,
    pub external_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub year: Option<String>,
    pub in_library: bool,
}

/// A book series (grouping of items). Section is derived: "done" when every book is read.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesSummary {
    pub id: i64,
    pub category: String,
    pub title: String,
    pub section: String,
    pub image_url: Option<String>,
    pub book_count: i64,
    pub read_count: i64,
    pub authors: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetail {
    pub series: SeriesSummary,
    pub books: Vec<Item>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryData {
    pub items: Vec<Item>,
    pub series: Vec<SeriesSummary>,
}

#[derive(Debug, Clone)]
pub struct ItemData {
    pub title: String,
    pub subtitle: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub release_date: Option<String>,
    pub status: Option<String>,
    pub extra: Value,
    pub episodes: Option<Vec<EpisodeData>>,
}

#[derive(Debug, Clone)]
pub struct EpisodeData {
    pub external_id: String,
    pub season: i64,
    pub number: Option<i64>,
    pub name: Option<String>,
    pub airdate: Option<String>,
    pub airstamp: Option<String>,
    pub runtime: Option<i64>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualItemInput {
    pub category: String,
    pub section: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub summary: Option<String>,
    pub image_url: Option<String>,
    pub release_date: Option<String>,
    pub status: Option<String>,
    pub series_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub tmdb_key: String,
    pub igdb_client_id: String,
    pub igdb_client_secret: String,
    pub hardcover_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSummary {
    pub refreshed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub db_path: String,
    pub version: String,
}