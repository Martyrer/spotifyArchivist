pub mod auth;
pub mod commands;
pub mod export;
pub mod spotify;
pub mod store;
pub mod sync;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use commands::handlers;
use commands::AppState;
use store::{MembershipFilter, Row, Source};

pub fn app_name() -> &'static str {
    "Spotify Archivist"
}

const DEFAULT_LOGIN_TIMEOUT_SECS: u64 = 300;

/// Tauri-managed wrapper that holds the shared AppState plus the in-flight
/// login state, if any.
pub struct AppHandle {
    pub state: AppState,
    pub login: Mutex<Option<handlers::StartedLogin>>,
}

impl AppHandle {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            login: Mutex::new(None),
        }
    }
}

#[tauri::command]
async fn list_sources(app: tauri::State<'_, AppHandle>) -> Result<Vec<Source>, handlers::CommandError> {
    handlers::list_sources(&app.state).await
}

#[tauri::command]
async fn toggle_source(
    app: tauri::State<'_, AppHandle>,
    id: i64,
    enabled: bool,
) -> Result<(), handlers::CommandError> {
    handlers::toggle_source(&app.state, id, enabled).await
}

#[tauri::command]
async fn list_memberships(
    app: tauri::State<'_, AppHandle>,
    source_id: i64,
    filter: MembershipFilter,
) -> Result<Vec<Row>, handlers::CommandError> {
    handlers::list_memberships(&app.state, source_id, filter).await
}

#[tauri::command]
async fn get_settings(app: tauri::State<'_, AppHandle>) -> Result<handlers::Settings, handlers::CommandError> {
    handlers::get_settings(&app.state).await
}

#[tauri::command]
async fn update_settings(
    app: tauri::State<'_, AppHandle>,
    sync_interval_hours: u32,
) -> Result<handlers::Settings, handlers::CommandError> {
    handlers::update_settings(&app.state, sync_interval_hours).await
}

#[tauri::command]
async fn logout(app: tauri::State<'_, AppHandle>) -> Result<(), handlers::CommandError> {
    handlers::logout(&app.state).await
}

#[tauri::command]
async fn list_available_playlists(
    app: tauri::State<'_, AppHandle>,
) -> Result<Vec<handlers::AvailablePlaylist>, handlers::CommandError> {
    handlers::list_available_playlists(&app.state).await
}

#[tauri::command]
async fn track_playlist(
    app: tauri::State<'_, AppHandle>,
    spotify_id: String,
    name: String,
) -> Result<i64, handlers::CommandError> {
    handlers::track_playlist(&app.state, &spotify_id, &name).await
}

#[tauri::command]
async fn trigger_sync(
    app: tauri::State<'_, AppHandle>,
) -> Result<Vec<sync::SyncOutcome>, handlers::CommandError> {
    handlers::trigger_sync(&app.state).await
}

#[tauri::command]
async fn export(
    app: tauri::State<'_, AppHandle>,
    scope: handlers::ExportScope,
    path: PathBuf,
) -> Result<usize, handlers::CommandError> {
    handlers::export(&app.state, scope, path).await
}

#[derive(serde::Serialize)]
struct StartLoginResponse {
    authorize_url: String,
}

#[tauri::command]
async fn start_login(
    app: tauri::State<'_, AppHandle>,
) -> Result<StartLoginResponse, handlers::CommandError> {
    let started = handlers::begin_login(&app.state).await?;
    let url = started.authorize_url.clone();
    *app.login.lock().expect("login mutex") = Some(started);
    Ok(StartLoginResponse { authorize_url: url })
}

#[tauri::command]
async fn await_login(
    app: tauri::State<'_, AppHandle>,
) -> Result<handlers::Settings, handlers::CommandError> {
    let started = app
        .login
        .lock()
        .expect("login mutex")
        .take()
        .ok_or(handlers::CommandError::NotAuthenticated)?;
    handlers::finish_login(&app.state, started, Duration::from_secs(DEFAULT_LOGIN_TIMEOUT_SECS)).await
}

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            list_sources,
            toggle_source,
            list_memberships,
            get_settings,
            update_settings,
            logout,
            list_available_playlists,
            track_playlist,
            trigger_sync,
            export,
            start_login,
            await_login,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_spotify_archivist() {
        assert_eq!(app_name(), "Spotify Archivist");
    }

    #[test]
    fn ping_returns_pong() {
        assert_eq!(ping(), "pong");
    }
}
