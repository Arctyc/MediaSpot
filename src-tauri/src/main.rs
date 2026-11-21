#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod commands;
mod db;
mod error;
mod models;
mod series;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tauri::Manager;

use crate::error::{AppError, AppResult};

pub struct AppState {
    pub db: Mutex<Connection>,
    pub http: reqwest::Client,
    pub igdb_token: Mutex<Option<(String, Instant)>>,
    pub db_path: String,
}

impl AppState {
    pub fn with_db<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self
            .db
            .lock()
            .map_err(|_| AppError::Msg("Database lock was poisoned".into()))?;
        f(&*conn)
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_file = dir.join("mediaspot.db");
            let conn = db::open(&db_file)?;
            let http = reqwest::Client::builder()
                .user_agent(concat!("MediaSpot/", env!("CARGO_PKG_VERSION"), " (personal desktop app)"))
                .timeout(Duration::from_secs(30))
                .build()?;
            app.manage(AppState {
                db: Mutex::new(conn),
                http,
                igdb_token: Mutex::new(None),
                db_path: db_file.to_string_lossy().into_owned(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_settings,
            commands::save_settings,
            commands::list_library,
            commands::get_item,
            commands::search,
            commands::add_from_search,
            commands::add_manual,
            commands::remove_item,
            commands::set_section,
            commands::set_rating,
            commands::refresh_item,
            commands::get_episodes,
            commands::set_episode_watched,
            commands::set_season_watched,
            commands::mark_all_watched,
            commands::add_manual_episodes,
            commands::watch_next,
            commands::upcoming,
            commands::refresh_all_tv,
            commands::get_series,
            commands::list_series,
            commands::create_series,
            commands::rename_series,
            commands::add_to_series,
            commands::remove_from_series,
            commands::ungroup_series,
            commands::delete_series,
            commands::set_series_read,
            commands::auto_group_books,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MediaSpot");
}