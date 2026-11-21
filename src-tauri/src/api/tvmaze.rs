use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde_json::Value;

use super::{check, extra, fact, join_opts, s, str_list, strip_html, year_of};
use crate::error::AppResult;
use crate::models::{EpisodeData, ItemData, SearchResult};

const BASE: &str = "https://api.tvmaze.com";

/// GET with automatic back-off on TVMaze's rate limit (HTTP 429).
async fn get_json(client: &Client, url: &str, query: &[(&str, &str)]) -> AppResult<Value> {
    let mut attempt: u64 = 0;
    loop {
        let mut req = client.get(url);
        if !query.is_empty() {
            req = req.query(query);
        }
        let resp = req.send().await?;
        if resp.status() == StatusCode::TOO_MANY_REQUESTS && attempt < 4 {
            attempt += 1;
            tokio::time::sleep(Duration::from_millis(1500 * attempt)).await;
            continue;
        }
        let v: Value = check(resp, "TVMaze").await?.json().await?;
        return Ok(v);
    }
}

fn network(sh: &Value) -> Option<String> {
    s(&sh["network"]["name"]).or_else(|| s(&sh["webChannel"]["name"]))
}

pub async fn search(client: &Client, query: &str) -> AppResult<Vec<SearchResult>> {
    let v = get_json(client, &format!("{BASE}/search/shows"), &[("q", query)]).await?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr
        .iter()
        .filter_map(|r| {
            let sh = &r["show"];
            let id = sh["id"].as_i64()?;
            let premiered = s(&sh["premiered"]);
            Some(SearchResult {
                source: "tvmaze".into(),
                external_id: id.to_string(),
                title: s(&sh["name"]).unwrap_or_else(|| "Untitled".into()),
                subtitle: join_opts([network(sh), s(&sh["status"])]),
                summary: s(&sh["summary"]).map(|x| strip_html(&x)),
                image_url: s(&sh["image"]["medium"]).or_else(|| s(&sh["image"]["original"])),
                year: year_of(&premiered),
                in_library: false,
            })
        })
        .collect())
}

fn parse_episode(e: &Value) -> Option<EpisodeData> {
    let id = e["id"].as_i64()?;
    let number = e["number"].as_i64()?; // skip unnumbered specials
    Some(EpisodeData {
        external_id: id.to_string(),
        season: e["season"].as_i64().unwrap_or(0),
        number: Some(number),
        name: s(&e["name"]),
        airdate: s(&e["airdate"]),
        airstamp: s(&e["airstamp"]),
        runtime: e["runtime"].as_i64(),
        summary: s(&e["summary"]).map(|x| strip_html(&x)),
        image_url: s(&e["image"]["medium"]),
    })
}

pub async fn details(client: &Client, id: &str) -> AppResult<ItemData> {
    let v = get_json(client, &format!("{BASE}/shows/{}", id.trim()), &[("embed", "episodes")]).await?;

    let premiered = s(&v["premiered"]);
    let ended = s(&v["ended"]);
    let status = s(&v["status"]); // Running | Ended | To Be Determined | In Development
    let net = network(&v);

    let episodes: Vec<EpisodeData> = v["_embedded"]["episodes"]
        .as_array()
        .map(|a| a.iter().filter_map(parse_episode).collect())
        .unwrap_or_default();
    let season_count = episodes.iter().map(|e| e.season).collect::<std::collections::HashSet<_>>().len();

    let mut facts = Vec::new();
    if let Some(n) = &net {
        facts.push(fact("Network", n.clone()));
    }
    if let Some(st) = &status {
        facts.push(fact("Status", st.clone()));
    }
    if let Some(p) = &premiered {
        facts.push(fact("Premiered", p.clone()));
    }
    if let Some(e) = &ended {
        facts.push(fact("Ended", e.clone()));
    }
    if season_count > 0 {
        facts.push(fact("Seasons", format!("{season_count} ({} episodes)", episodes.len())));
    }
    if let Some(rt) = v["averageRuntime"].as_i64().or_else(|| v["runtime"].as_i64()) {
        facts.push(fact("Runtime", format!("{rt} min")));
    }
    if let Some(l) = s(&v["language"]) {
        facts.push(fact("Language", l));
    }
    if let Some(r) = v["rating"]["average"].as_f64() {
        facts.push(fact("TVMaze rating", format!("{r:.1} / 10")));
    }

    let years = match (year_of(&premiered), year_of(&ended)) {
        (Some(a), Some(b)) => Some(if a == b { a } else { format!("{a}–{b}") }),
        (Some(a), None) => Some(format!("{a}–")),
        _ => None,
    };

    Ok(ItemData {
        title: s(&v["name"]).unwrap_or_else(|| "Untitled".into()),
        subtitle: join_opts([net, years]),
        summary: s(&v["summary"]).map(|x| strip_html(&x)),
        image_url: s(&v["image"]["original"]).or_else(|| s(&v["image"]["medium"])),
        release_date: premiered,
        status,
        extra: extra(facts, str_list(&v["genres"])),
        episodes: Some(episodes),
    })
}
