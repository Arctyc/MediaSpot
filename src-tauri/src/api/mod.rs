pub mod hardcover;
pub mod igdb;
pub mod tmdb;
pub mod tvmaze;

use std::collections::HashSet;

use reqwest::Response;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};

/// Turn non-2xx responses into readable errors.
pub async fn check(resp: Response, service: &str) -> AppResult<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    let snippet: String = body.chars().take(300).collect();
    let hint = match status.as_u16() {
        401 | 403 => " — check your API credentials in Settings",
        429 => " — rate limited, try again shortly",
        _ => "",
    };
    Err(AppError::Msg(format!("{service} request failed ({status}){hint}: {snippet}")))
}

/// Non-empty trimmed string from a JSON value.
pub fn s(v: &Value) -> Option<String> {
    v.as_str().map(|x| x.trim().to_string()).filter(|x| !x.is_empty())
}

pub fn str_list(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(s).collect()).unwrap_or_default()
}

pub fn name_list(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| s(&x["name"])).collect())
        .unwrap_or_default()
}

pub fn year_of(date: &Option<String>) -> Option<String> {
    date.as_ref()
        .and_then(|d| d.get(0..4))
        .filter(|y| y.chars().all(|c| c.is_ascii_digit()))
        .map(String::from)
}

pub fn join_opts<I: IntoIterator<Item = Option<String>>>(parts: I) -> Option<String> {
    let v: Vec<String> = parts.into_iter().flatten().collect();
    if v.is_empty() { None } else { Some(v.join(" · ")) }
}

pub fn fact(label: &str, value: impl Into<String>) -> Value {
    json!({ "label": label, "value": value.into() })
}

pub fn extra(facts: Vec<Value>, genres: Vec<String>) -> Value {
    json!({ "facts": facts, "genres": genres })
}

pub fn dedupe(v: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    v.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

/// Minimal HTML -> text (TVMaze summaries are HTML).
pub fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut tag = String::new();
    for ch in input.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let t = tag.to_lowercase();
                if t.starts_with("/p") || t.starts_with("br") {
                    out.push('\n');
                }
            }
            _ if in_tag => tag.push(ch),
            _ => out.push(ch),
        }
    }
    let out = out
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    out.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
