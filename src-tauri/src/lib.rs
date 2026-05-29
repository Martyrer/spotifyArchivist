pub mod auth;
pub mod commands;
pub mod export;
pub mod notify;
pub mod scheduler;
pub mod spotify;
pub mod store;
pub mod sync;
pub mod ui;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Listener, Manager};

use commands::handlers;
use commands::AppState;
use store::{MembershipFilter, Row, Source};

pub fn app_name() -> &'static str {
    "Spotify Archivist"
}

const DEFAULT_LOGIN_TIMEOUT_SECS: u64 = 120;

const CLIENT_ID_ENV: &str = "SPOTIFY_ARCHIVIST_CLIENT_ID";

pub struct AppHandle {
    pub state: std::sync::Arc<AppState>,
    pub login: Mutex<Option<handlers::StartedLogin>>,
    pub scheduler_tx: Mutex<Option<tokio::sync::mpsc::Sender<scheduler::Tick>>>,
    pub scheduler_join: Mutex<Option<JoinHandle<()>>>,
}

impl AppHandle {
    pub fn new(state: AppState) -> Self {
        Self {
            state: std::sync::Arc::new(state),
            login: Mutex::new(None),
            scheduler_tx: Mutex::new(None),
            scheduler_join: Mutex::new(None),
        }
    }
}

#[tauri::command]
async fn list_sources(
    app: tauri::State<'_, AppHandle>,
) -> Result<Vec<Source>, handlers::CommandError> {
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
async fn untrack_source(
    app: tauri::State<'_, AppHandle>,
    id: i64,
) -> Result<(), handlers::CommandError> {
    handlers::untrack_source(&app.state, id).await
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
async fn get_settings(
    app: tauri::State<'_, AppHandle>,
) -> Result<handlers::Settings, handlers::CommandError> {
    handlers::get_settings(&app.state).await
}

#[tauri::command]
async fn update_settings(
    app: tauri::State<'_, AppHandle>,
    sync_interval_hours: u32,
) -> Result<handlers::Settings, handlers::CommandError> {
    let result = handlers::update_settings(&app.state, sync_interval_hours).await?;
    let tx = app.scheduler_tx.lock().expect("scheduler mutex").clone();
    if let Some(tx) = tx {
        let _ = tx.send(scheduler::Tick::Reschedule).await;
    }
    Ok(result)
}

#[tauri::command]
async fn logout(app: tauri::State<'_, AppHandle>) -> Result<(), handlers::CommandError> {
    handlers::logout(&app.state).await
}

#[tauri::command]
async fn reset_app(app: tauri::State<'_, AppHandle>) -> Result<(), handlers::CommandError> {
    handlers::reset_app(&app.state).await
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
    handle: tauri::AppHandle,
) -> Result<Vec<sync::SyncOutcome>, handlers::CommandError> {
    let outcomes = handlers::trigger_sync(&app.state).await?;
    ui::dispatch_post_sync(&handle, &app.state, &outcomes).await;
    Ok(outcomes)
}

#[tauri::command]
async fn mark_seen(
    app: tauri::State<'_, AppHandle>,
    handle: tauri::AppHandle,
) -> Result<(), handlers::CommandError> {
    app.state
        .store
        .clear_unseen_losses()
        .await
        .map_err(handlers::CommandError::from)?;
    let _ = handle.emit("losses:updated", 0u32);
    ui::update_tray_badge(&handle, 0);
    Ok(())
}

#[tauri::command]
async fn get_unseen_losses(
    app: tauri::State<'_, AppHandle>,
) -> Result<u32, handlers::CommandError> {
    Ok(app.state.store.unseen_losses().await?)
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
async fn complete_onboarding(
    app: tauri::State<'_, AppHandle>,
) -> Result<(), handlers::CommandError> {
    handlers::complete_onboarding(&app.state).await
}

#[tauri::command]
async fn cancel_login(app: tauri::State<'_, AppHandle>) -> Result<(), handlers::CommandError> {
    let _ = app.login.lock().expect("login mutex").take();
    Ok(())
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
    handlers::finish_login(
        &app.state,
        started,
        Duration::from_secs(DEFAULT_LOGIN_TIMEOUT_SECS),
    )
    .await
}

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn build_tray<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> tauri::Result<tauri::tray::TrayIcon<R>> {
    let open_item = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let sync_item = MenuItem::with_id(app, "sync_now", "Sync now", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &sync_item, &quit_item])?;

    TrayIconBuilder::with_id("main")
        .menu(&menu)
        .icon(app.default_window_icon().cloned().unwrap())
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "sync_now" => {
                let _ = app.emit("sync:trigger-from-tray", ());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(w) = tray.app_handle().get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)
}

fn resolve_data_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
}

fn resolve_client_id() -> String {
    std::env::var(CLIENT_ID_ENV).unwrap_or_else(|_| {
        panic!("{CLIENT_ID_ENV} must be set to a Spotify application client id");
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load a .env from the project root during development. Missing in
    // release builds, where the variable comes from the real environment.
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let data_dir = resolve_data_dir(app.handle());
            std::fs::create_dir_all(&data_dir).ok();
            let db_path = data_dir.join("archivist.sqlite");

            let store =
                tauri::async_runtime::block_on(async { store::Store::open(&db_path).await })
                    .expect("open store");

            let tokens = auth::TokenStore::os_keyring("dev.archivist.spotify", "tokens");
            let client_id = resolve_client_id();
            let state = AppState::new(store, tokens, client_id, data_dir);
            let app_handle = AppHandle::new(state);

            // If a refresh token is already in the keyring, fetch the current
            // user once at startup so list_available_playlists / playlist
            // ownership filters work without forcing a fresh login. Also
            // ensure the Liked Songs source row exists — it is created on
            // first login but a returning user with an empty store could
            // otherwise end up with zero sources tracked.
            let rehydrate_state = app_handle.state.clone();
            tauri::async_runtime::block_on(async move {
                if rehydrate_state.tokens.load().ok().flatten().is_some() {
                    if let Ok(me) = rehydrate_state.spotify.current_user().await {
                        *rehydrate_state.current_user_id.write().await = Some(me.id);
                    }
                    let _ = handlers::ensure_liked_songs_source(&rehydrate_state).await;
                }
            });

            let sink = std::sync::Arc::new(ui::ToastSink {
                handle: app.handle().clone(),
                state: app_handle.state.clone(),
            });
            let sched = scheduler::spawn(app_handle.state.clone(), sink);
            *app_handle.scheduler_tx.lock().unwrap() = Some(sched.tx.clone());
            *app_handle.scheduler_join.lock().unwrap() = sched.into_join_handle();

            app.manage(app_handle);

            let _tray = build_tray(app.handle())?;

            let trigger_handle = app.handle().clone();
            app.handle().listen("sync:trigger-from-tray", move |_| {
                let h = trigger_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let tx = h
                        .try_state::<AppHandle>()
                        .and_then(|s| s.scheduler_tx.lock().ok().and_then(|g| g.clone()));
                    if let Some(tx) = tx {
                        let _ = tx.send(scheduler::Tick::Trigger).await;
                    }
                });
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            list_sources,
            toggle_source,
            untrack_source,
            list_memberships,
            get_settings,
            update_settings,
            logout,
            reset_app,
            list_available_playlists,
            track_playlist,
            trigger_sync,
            export,
            start_login,
            cancel_login,
            await_login,
            mark_seen,
            get_unseen_losses,
            complete_onboarding,
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

    #[test]
    fn resolve_client_id_reads_env() {
        std::env::set_var(CLIENT_ID_ENV, "abc123");
        assert_eq!(resolve_client_id(), "abc123");
        std::env::remove_var(CLIENT_ID_ENV);
    }
}
