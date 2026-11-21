use std::time::{Duration, Instant};

use reqwest::StatusCode;
use serde_json::{json, Value};

use super::{check, dedupe, extra, fact, join_opts, name_list, s, year_of};
use crate::error::{err, AppError, AppResult};
use crate::models::{ItemData, SearchResult, Settings};
use crate::AppState;

const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const API: &str = "https://api.igdb.com/v4";
const IMG: &str = "https://images.igdb.com/igdb/image/upload";

async fn token(state: &AppState, st: &Settings, force: bool) -> AppResult<String> {
    let id = st.igdb_client_id.trim();
    let secret = st.igdb_client_secret.trim();
    if id.is_empty() || secret.is_empty() {
        return err("IGDB Client ID / Client Secret are not set. Add them in Settings.");
    }
    if !force {
        let cached = state.igdb_token.lock().map(|g| g.clone()).unwrap_or(None);
        if let Some((tok, exp)) = cached {
            if Instant::now() < exp {
                return Ok(tok);
            }
        }
    }
    let resp = state
        .http
        .post(TOKEN_URL)
        .query(&[("client_id", id), ("client_secret", secret), ("grant_type", "client_credentials")])
        .send()
        .await?;
    let v: Value = check(resp, "Twitch authentication").await?.json().await?;
    let tok = s(&v["access_token"]).ok_or_else(|| AppError::Msg("Twitch did not return an access token".into()))?;
    let ttl = v["expires_in"].as_u64().unwrap_or(3600).saturating_sub(300).max(60);
    if let Ok(mut g) = state.igdb_token.lock() {
        *g = Some((tok.clone(), Instant::now() + Duration::from_secs(ttl)));
    }
    Ok(tok)
}

async fn igdb_post(state: &AppState, st: &Settings, endpoint: &str, body: String) -> AppResult<Value> {
    let mut tok = token(state, st, false).await?;
    for attempt in 0..2 {
        let resp = state
            .http
            .post(format!("{API}/{endpoint}"))
            .header("Client-ID", st.igdb_client_id.trim())
            .header("Accept", "application/json")
            .bearer_auth(&tok)
            .body(body.clone())
            .send()
            .await?;
        if resp.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
            tok = token(state, st, true).await?;
            continue;
        }
        if resp.status() == StatusCode::TOO_MANY_REQUESTS && attempt == 0 {
            tokio::time::sleep(Duration::from_millis(700)).await;
            continue;
        }
        let v: Value = check(resp, "IGDB").await?.json().await?;
        return Ok(v);
    }
    err("IGDB request failed after retrying")
}

fn cover(v: &Value, size: &str) -> Option<String> {
    s(v).map(|id| format!("{IMG}/{size}/{id}.jpg"))
}

fn unix_date(v: &Value) -> Option<String> {
    v.as_i64()
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|d| d.format("%Y-%m-%d").to_string())
}

fn companies(g: &Value, role: &str) -> Vec<String> {
    dedupe(
        g["involved_companies"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|c| c[role].as_bool() == Some(true))
                    .filter_map(|c| s(&c["company"]["name"]))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

const STORES: &[(&str, &str)] = &[
    ("store.steampowered.com", "Steam"),
    ("gog.com", "GOG"),
    ("epicgames.com", "Epic Games Store"),
    ("itch.io", "itch.io"),
    ("nintendo.com", "Nintendo eShop"),
    ("nintendo.co", "Nintendo eShop"),
    ("playstation.com", "PlayStation Store"),
    ("xbox.com", "Xbox Store"),
    ("microsoft.com", "Microsoft Store"),
    ("apps.apple.com", "App Store"),
    ("play.google.com", "Google Play"),
    ("humblebundle.com", "Humble Store"),
];

const NON_OFFICIAL: &[&str] = &[
    "wikipedia.org", "fandom.com", "wikia.com", "twitter.com", "x.com", "facebook.com", "instagram.com",
    "youtube.com", "twitch.tv", "discord.gg", "discord.com", "reddit.com", "bsky.app", "tiktok.com",
];

/// Turn IGDB `websites` into [{label, url}] — storefronts plus the official site.
fn links_from_websites(g: &Value) -> Vec<Value> {
    let mut stores = Vec::new();
    let mut official = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for w in g["websites"].as_array().cloned().unwrap_or_default() {
        let Some(url) = s(&w["url"]) else { continue };
        let lower = url.to_lowercase();
        if let Some((_, label)) = STORES.iter().find(|(dom, _)| lower.contains(dom)) {
            if seen.insert(label.to_string()) {
                stores.push(json!({ "label": label, "url": url }));
            }
        } else if !NON_OFFICIAL.iter().any(|d| lower.contains(d)) && official.is_empty() {
            official.push(json!({ "label": "Official site", "url": url }));
        }
    }
    official.extend(stores);
    official
}

pub async fn search(state: &AppState, st: &Settings, query: &str) -> AppResult<Vec<SearchResult>> {
    let q = query.replace('\\', "\\\\").replace('"', "\\\"");
    let body = format!(
        "search \"{q}\"; \
         fields name,summary,first_release_date,cover.image_id,involved_companies.company.name,\
         involved_companies.developer,platforms.abbreviation; \
         where version_parent = null; limit 25;"
    );
    let v = igdb_post(state, st, "games", body).await?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr
        .iter()
        .filter_map(|g| {
            let id = g["id"].as_i64()?;
            let date = unix_date(&g["first_release_date"]);
            let devs = companies(g, "developer");
            let platforms: Vec<String> = g["platforms"]
                .as_array()
                .map(|a| a.iter().filter_map(|p| s(&p["abbreviation"])).take(4).collect())
                .unwrap_or_default();
            Some(SearchResult {
                source: "igdb".into(),
                external_id: id.to_string(),
                title: s(&g["name"]).unwrap_or_else(|| "Untitled".into()),
                subtitle: join_opts([devs.first().cloned(), (!platforms.is_empty()).then(|| platforms.join(", "))]),
                summary: s(&g["summary"]),
                image_url: cover(&g["cover"]["image_id"], "t_cover_big"),
                year: year_of(&date),
                in_library: false,
            })
        })
        .collect())
}

pub async fn details(state: &AppState, st: &Settings, id: &str) -> AppResult<ItemData> {
    let id: i64 = id.trim().parse().map_err(|_| AppError::Msg("Invalid IGDB game id".into()))?;
    let body = format!(
        "fields name,summary,storyline,first_release_date,cover.image_id,genres.name,themes.name,\
         platforms.name,involved_companies.company.name,involved_companies.developer,\
         involved_companies.publisher,game_modes.name,total_rating,total_rating_count,websites.url; \
         where id = {id}; limit 1;"
    );
    let v = igdb_post(state, st, "games", body).await?;
    let g = v.get(0).cloned().ok_or_else(|| AppError::Msg("Game not found on IGDB".into()))?;

    let date = unix_date(&g["first_release_date"]);
    let devs = companies(&g, "developer");
    let pubs = companies(&g, "publisher");
    let platforms = name_list(&g["platforms"]);
    let modes = name_list(&g["game_modes"]);

    let mut facts = Vec::new();
    if let Some(d) = &date {
        facts.push(fact("Released", d.clone()));
    }
    if !devs.is_empty() {
        facts.push(fact("Developer", devs.join(", ")));
    }
    if !pubs.is_empty() {
        facts.push(fact("Publisher", pubs.join(", ")));
    }
    if !platforms.is_empty() {
        facts.push(fact("Platforms", platforms.join(", ")));
    }
    if !modes.is_empty() {
        facts.push(fact("Modes", modes.join(", ")));
    }
    if let Some(r) = g["total_rating"].as_f64() {
        facts.push(fact("IGDB rating", format!("{r:.0} / 100")));
    }

    let mut genres = name_list(&g["genres"]);
    genres.extend(name_list(&g["themes"]));

    let mut ex = extra(facts, dedupe(genres));
    ex["links"] = Value::Array(links_from_websites(&g));

    Ok(ItemData {
        title: s(&g["name"]).unwrap_or_else(|| "Untitled".into()),
        subtitle: join_opts([year_of(&date), devs.first().cloned()]),
        summary: s(&g["summary"]).or_else(|| s(&g["storyline"])),
        image_url: cover(&g["cover"]["image_id"], "t_cover_big_2x"),
        release_date: date,
        status: None,
        extra: ex,
        episodes: None,
    })
}