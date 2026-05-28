pub mod auth;
pub mod spotify;
pub mod store;
pub mod sync;

pub fn app_name() -> &'static str {
    "Spotify Archivist"
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
        .invoke_handler(tauri::generate_handler![ping])
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
