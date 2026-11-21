use reqwest::Client;
use serde_json::{json, Value};

use super::{check, dedupe, extra, fact, s, str_list, year_of};
use crate::error::{err, AppError, AppResult};
use crate::models::{ItemData, SearchResult};

const ENDPOINT: &str = "https://api.hardcover.app/v1/graphql";

const SEARCH_QUERY: &str = r#"
query MediaSpotSearch($q: String!) {
  search(query: $q, query_type: "Book", per_page: 20, page: 1) {
    results
  }
}"#;

// Full query, including series membership.
const BOOK_QUERY_FULL: &str = r#"
query MediaSpotBook($id: Int!) {
  books(where: {id: {_eq: $id}}, limit: 1) {
    id
    title
    subtitle
    description
    release_date
    release_year
    pages
    rating
    image { url }
    contributions { author { name } }
    cached_tags
    book_series { position series { name } }
  }
}"#;

// Fallback without series, then a minimal one, in case the schema changes.
const BOOK_QUERY_MID: &str = r#"
query MediaSpotBook($id: Int!) {
  books(where: {id: {_eq: $id}}, limit: 1) {
    id
    title
    subtitle
    description
    release_date
    release_year
    pages
    rating
    image { url }
    contributions { author { name } }
    cached_tags
  }
}"#;

const BOOK_QUERY_MIN: &str = r#"
query MediaSpotBook($id: Int!) {
  books(where: {id: {_eq: $id}}, limit: 1) {
    id
    title
    description
    release_date
    image { url }
    contributions { author { name } }
  }
}"#;

fn auth_header(token: &str) -> AppResult<String> {
    let t = token.trim();
    let t = t
        .strip_prefix("Bearer ")
        .or_else(|| t.strip_prefix("bearer "))
        .unwrap_or(t)
        .trim();
    if t.is_empty() {
        return err("Hardcover API token is not set. Add it in Settings.");
    }
    Ok(format!("Bearer {t}"))
}

async fn gql(client: &Client, token: &str, query: &str, variables: Value) -> AppResult<Value> {
    let auth = auth_header(token)?;
    let resp = client
        .post(ENDPOINT)
        .header("authorization", auth)
        .json(&json!({ "query": query, "variables": variables }))
        .send()
        .await?;
    let v: Value = check(resp, "Hardcover").await?.json().await?;
    if let Some(errors) = v["errors"].as_array() {
        if !errors.is_empty() {
            let msg = errors.iter().filter_map(|e| s(&e["message"])).collect::<Vec<_>>().join("; ");
            return err(format!("Hardcover API error: {msg}"));
        }
    }
    if let Some(e) = s(&v["error"]) {
        return err(format!("Hardcover API error: {e}"));
    }
    Ok(v["data"].clone())
}

fn id_string(v: &Value) -> Option<String> {
    v.as_str().map(String::from).or_else(|| v.as_i64().map(|i| i.to_string()))
}

pub async fn search(client: &Client, token: &str, query: &str) -> AppResult<Vec<SearchResult>> {
    let data = gql(client, token, SEARCH_QUERY, json!({ "q": query })).await?;
    let raw = &data["search"]["results"];
    let results: Value = match raw.as_str() {
        Some(st) => serde_json::from_str(st)?,
        None => raw.clone(),
    };
    let hits = results["hits"].as_array().cloned().unwrap_or_default();
    Ok(hits
        .iter()
        .filter_map(|h| {
            let d = &h["document"];
            let id = id_string(&d["id"])?;
            let authors = str_list(&d["author_names"]);
            let year = d["release_year"].as_i64().map(|y| y.to_string()).or_else(|| s(&d["release_year"]));
            Some(SearchResult {
                source: "hardcover".into(),
                external_id: id,
                title: s(&d["title"]).unwrap_or_else(|| "Untitled".into()),
                subtitle: if authors.is_empty() {
                    None
                } else {
                    Some(authors.into_iter().take(3).collect::<Vec<_>>().join(", "))
                },
                summary: s(&d["description"]),
                image_url: s(&d["image"]["url"]),
                year,
                in_library: false,
            })
        })
        .collect())
}

pub async fn details(client: &Client, token: &str, id: &str) -> AppResult<ItemData> {
    let id_num: i64 = id
        .trim()
        .parse()
        .map_err(|_| AppError::Msg("Invalid Hardcover book id".into()))?;
    let vars = json!({ "id": id_num });
    let data = match gql(client, token, BOOK_QUERY_FULL, vars.clone()).await {
        Ok(d) => d,
        Err(_) => match gql(client, token, BOOK_QUERY_MID, vars.clone()).await {
            Ok(d) => d,
            Err(_) => gql(client, token, BOOK_QUERY_MIN, vars).await?,
        },
    };
    let b = data["books"]
        .get(0)
        .cloned()
        .ok_or_else(|| AppError::Msg("Book not found on Hardcover".into()))?;

    let authors = dedupe(
        b["contributions"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| s(&c["author"]["name"])).collect())
            .unwrap_or_default(),
    );
    let release = s(&b["release_date"]).or_else(|| b["release_year"].as_i64().map(|y| y.to_string()));

    let mut tags = b["cached_tags"].clone();
    if let Some(st) = tags.as_str() {
        tags = serde_json::from_str(st).unwrap_or(Value::Null);
    }
    let genres: Vec<String> = tags["Genre"]
        .as_array()
        .map(|a| a.iter().filter_map(|t| s(&t["tag"]).or_else(|| s(t))).take(6).collect())
        .unwrap_or_default();

    // Series membership (first entry)
    let (series_name, series_position): (Option<String>, Option<f64>) = b["book_series"]
        .as_array()
        .and_then(|a| {
            a.iter().find_map(|bs| {
                let name = s(&bs["series"]["name"])?;
                let pos = bs["position"]
                    .as_f64()
                    .or_else(|| s(&bs["position"]).and_then(|p| p.parse().ok()));
                Some((Some(name), pos))
            })
        })
        .unwrap_or((None, None));

    let mut facts = Vec::new();
    if let Some(st) = s(&b["subtitle"]) {
        facts.push(fact("Subtitle", st));
    }
    if !authors.is_empty() {
        facts.push(fact(if authors.len() > 1 { "Authors" } else { "Author" }, authors.join(", ")));
    }
    if let Some(name) = &series_name {
        let label = match series_position {
            Some(p) if p.fract() == 0.0 => format!("{name} #{}", p as i64),
            Some(p) => format!("{name} #{p}"),
            None => name.clone(),
        };
        facts.push(fact("Series", label));
    }
    if let Some(r) = &release {
        facts.push(fact("Published", r.clone()));
    }
    if let Some(p) = b["pages"].as_i64().filter(|p| *p > 0) {
        facts.push(fact("Pages", p.to_string()));
    }
    let rating = b["rating"].as_f64().or_else(|| s(&b["rating"]).and_then(|x| x.parse().ok()));
    if let Some(r) = rating.filter(|r| *r > 0.0) {
        facts.push(fact("Hardcover rating", format!("{r:.2} / 5")));
    }

    let subtitle = if authors.is_empty() {
        year_of(&release)
    } else {
        Some(authors.iter().take(3).cloned().collect::<Vec<_>>().join(", "))
    };

    let mut ex = extra(facts, genres);
    ex["seriesName"] = json!(series_name);
    ex["seriesPosition"] = json!(series_position);

    Ok(ItemData {
        title: s(&b["title"]).unwrap_or_else(|| "Untitled".into()),
        subtitle,
        summary: s(&b["description"]),
        image_url: s(&b["image"]["url"]),
        release_date: release,
        status: None,
        extra: ex,
        episodes: None,
    })
}