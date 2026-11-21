//! Automatic book-series detection.
//!
//! Order of preference:
//!   1. Hardcover's own series metadata (`extra.seriesName`), matched case-insensitively.
//!   2. Title similarity: a shared leading run of words (ignoring articles/"Book 2"/numerals)
//!      plus a matching author. e.g. "Mistborn: The Final Empire" / "Mistborn: The Well of Ascension",
//!      "Harry Potter and the ..." / "Harry Potter and the ...", "Dune" / "Dune Messiah".

use std::collections::HashSet;

use rusqlite::Connection;

use crate::db;
use crate::error::AppResult;
use crate::models::Item;

const STOP: &[&str] = &[
    "the", "a", "an", "and", "of", "to", "in", "on", "at", "for", "or", "vol", "volume", "book",
    "part", "no", "series", "novel", "trilogy", "saga", "chronicles",
];

fn is_stop(w: &str) -> bool {
    STOP.contains(&w)
}

fn is_number_word(w: &str) -> bool {
    w.chars().all(|c| c.is_ascii_digit())
        || matches!(
            w,
            "i" | "ii" | "iii" | "iv" | "v" | "vi" | "vii" | "viii" | "ix" | "x"
                | "one" | "two" | "three" | "four" | "five" | "six" | "seven" | "eight" | "nine" | "ten"
        )
}

/// Lower-cased words of a title with any (parenthetical) or [bracketed] segments removed.
pub fn title_words(title: &str) -> Vec<String> {
    let mut s = String::new();
    let mut depth = 0u32;
    for c in title.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => s.push(c),
            _ => {}
        }
    }
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

pub struct TitleMatch {
    pub prefix: Vec<String>,
    pub content_words: usize,
}

/// If two titles share a meaningful leading phrase, returns it.
pub fn series_prefix(a: &str, b: &str) -> Option<TitleMatch> {
    let wa = title_words(a);
    let wb = title_words(b);
    if wa.is_empty() || wb.is_empty() || wa == wb {
        return None; // identical titles are probably editions, not series entries
    }
    let mut prefix: Vec<String> = wa
        .iter()
        .zip(wb.iter())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.clone())
        .collect();
    while let Some(last) = prefix.last() {
        if is_stop(last) || is_number_word(last) {
            prefix.pop();
        } else {
            break;
        }
    }
    let content_words = prefix.iter().filter(|w| !is_stop(w)).count();
    let ok = content_words >= 2 || (content_words == 1 && prefix.len() == 1 && prefix[0].len() >= 4);
    ok.then_some(TitleMatch { prefix, content_words })
}

pub fn pretty_name(words: &[String]) -> String {
    words
        .iter()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn author_set(item: &Item) -> HashSet<String> {
    item.subtitle
        .as_deref()
        .unwrap_or("")
        .split(|c| c == ',' || c == ';' || c == '·')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn series_name_of(item: &Item) -> Option<String> {
    item.extra
        .get("seriesName")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Try to place an ungrouped book into a series. Returns the series id if grouped.
pub fn auto_assign(conn: &Connection, item_id: i64) -> AppResult<Option<i64>> {
    let item = db::get_item(conn, item_id)?;
    if item.category != "book" || item.series_id.is_some() {
        return Ok(None);
    }
    let others: Vec<Item> = db::list_items_raw(conn, "book", None, false)?
        .into_iter()
        .filter(|o| o.id != item_id)
        .collect();

    // 1. Provider series metadata
    if let Some(name) = series_name_of(&item) {
        if let Some(sid) = db::find_series_by_title(conn, "book", &name)? {
            db::set_item_series(conn, item_id, Some(sid))?;
            return Ok(Some(sid));
        }
        let mates: Vec<&Item> = others
            .iter()
            .filter(|o| o.series_id.is_none())
            .filter(|o| series_name_of(o).map(|n| n.eq_ignore_ascii_case(&name)).unwrap_or(false))
            .collect();
        if !mates.is_empty() {
            let sid = db::create_series(conn, "book", &name)?;
            db::set_item_series(conn, item_id, Some(sid))?;
            for m in mates {
                db::set_item_series(conn, m.id, Some(sid))?;
            }
            return Ok(Some(sid));
        }
    }

    // 2. Title similarity
    let mine = author_set(&item);
    let mut ungrouped_candidate: Option<(&Item, Vec<String>)> = None;
    for o in &others {
        let Some(m) = series_prefix(&item.title, &o.title) else { continue };
        let theirs = author_set(o);
        let author_ok = match (mine.is_empty(), theirs.is_empty()) {
            (false, false) => !mine.is_disjoint(&theirs),
            _ => m.content_words >= 2, // unknown author: only accept strong title matches
        };
        if !author_ok {
            continue;
        }
        if let Some(sid) = o.series_id {
            db::set_item_series(conn, item_id, Some(sid))?;
            return Ok(Some(sid));
        }
        if ungrouped_candidate.is_none() {
            ungrouped_candidate = Some((o, m.prefix));
        }
    }
    if let Some((o, prefix)) = ungrouped_candidate {
        let sid = db::create_series(conn, "book", &pretty_name(&prefix))?;
        db::set_item_series(conn, item_id, Some(sid))?;
        db::set_item_series(conn, o.id, Some(sid))?;
        return Ok(Some(sid));
    }
    Ok(None)
}

/// Run detection across every ungrouped book. Returns how many books were grouped.
pub fn auto_group_all(conn: &Connection) -> AppResult<usize> {
    let ids: Vec<i64> = db::list_items_raw(conn, "book", None, true)?.iter().map(|i| i.id).collect();
    let mut n = 0;
    for id in ids {
        if auto_assign(conn, id)?.is_some() {
            n += 1;
        }
    }
    Ok(n)
}