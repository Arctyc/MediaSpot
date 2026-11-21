use reqwest::{Client, RequestBuilder};
use serde_json::Value;

use super::{check, extra, fact, join_opts, name_list, s, year_of};
use crate::error::{err, AppResult};
use crate::models::{ItemData, SearchResult};

const BASE: &str = "https://api.themoviedb.org/3";
const IMG: &str = "https://image.tmdb.org/t/p";

/// Supports both the v3 API key and the v4 "Read Access Token" (a JWT starting with eyJ).
fn authed(req: RequestBuilder, key: &str) -> RequestBuilder {
    let key = key.trim();
    if key.starts_with("eyJ") {
        req.bearer_auth(key)
    } else {
        req.query(&[("api_key", key)])
    }
}

fn require(key: &str) -> AppResult<()> {
    if key.trim().is_empty() {
        err("TMDB API key is not set. Add it in Settings.")
    } else {
        Ok(())
    }
}

pub async fn search(client: &Client, key: &str, query: &str) -> AppResult<Vec<SearchResult>> {
    require(key)?;
    let req = client
        .get(format!("{BASE}/search/movie"))
        .query(&[("query", query), ("include_adult", "false")]);
    let v: Value = check(authed(req, key).send().await?, "TMDB").await?.json().await?;
    let results = v["results"].as_array().cloned().unwrap_or_default();
    Ok(results
        .iter()
        .filter_map(|r| {
            let id = r["id"].as_i64()?;
            let release = s(&r["release_date"]);
            Some(SearchResult {
                source: "tmdb".into(),
                external_id: id.to_string(),
                title: s(&r["title"]).unwrap_or_else(|| "Untitled".into()),
                subtitle: None,
                summary: s(&r["overview"]),
                image_url: s(&r["poster_path"]).map(|p| format!("{IMG}/w185{p}")),
                year: year_of(&release),
                in_library: false,
            })
        })
        .collect())
}

pub async fn details(client: &Client, key: &str, id: &str) -> AppResult<ItemData> {
    require(key)?;
    let req = client
        .get(format!("{BASE}/movie/{}", id.trim()))
        .query(&[("append_to_response", "credits")]);
    let v: Value = check(authed(req, key).send().await?, "TMDB").await?.json().await?;

    let title = s(&v["title"]).unwrap_or_else(|| "Untitled".into());
    let release = s(&v["release_date"]);
    let directors: Vec<String> = v["credits"]["crew"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|c| c["job"].as_str() == Some("Director"))
                .filter_map(|c| s(&c["name"]))
                .collect()
        })
        .unwrap_or_default();
    let cast: Vec<String> = v["credits"]["cast"]
        .as_array()
        .map(|a| a.iter().take(6).filter_map(|c| s(&c["name"])).collect())
        .unwrap_or_default();

    let mut facts = Vec::new();
    if let Some(t) = s(&v["tagline"]) {
        facts.push(fact("Tagline", t));
    }
    if let Some(r) = &release {
        facts.push(fact("Released", r.clone()));
    }
    if let Some(rt) = v["runtime"].as_i64().filter(|x| *x > 0) {
        facts.push(fact("Runtime", format!("{}h {:02}m", rt / 60, rt % 60)));
    }
    if !directors.is_empty() {
        facts.push(fact(if directors.len() > 1 { "Directors" } else { "Director" }, directors.join(", ")));
    }
    if !cast.is_empty() {
        facts.push(fact("Starring", cast.join(", ")));
    }
    if let Some(l) = s(&v["original_language"]) {
        facts.push(fact("Original language", l.to_uppercase()));
    }
    if let Some(va) = v["vote_average"].as_f64().filter(|x| *x > 0.0) {
        facts.push(fact("TMDB score", format!("{va:.1} / 10")));
    }

    Ok(ItemData {
        subtitle: join_opts([year_of(&release), directors.first().cloned()]),
        title,
        summary: s(&v["overview"]),
        image_url: s(&v["poster_path"]).map(|p| format!("{IMG}/w500{p}")),
        release_date: release,
        status: s(&v["status"]),
        extra: extra(facts, name_list(&v["genres"])),
        episodes: None,
    })
}
