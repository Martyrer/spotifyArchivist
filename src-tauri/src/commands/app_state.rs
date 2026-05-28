use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::auth::TokenStore;
use crate::spotify::SpotifyClient;
use crate::store::Store;
use crate::sync::{Clock, SystemClock};

/// Application-wide state shared with every Tauri command.
///
/// `current_user_id` is filled in after a successful login or after a
/// keyring-backed startup re-auth, and then used to filter
/// `/me/playlists` to the user's own playlists per Decision 1.
pub struct AppState {
    pub store: Store,
    pub tokens: Arc<TokenStore>,
    pub spotify: Arc<SpotifyClient>,
    pub clock: Arc<dyn Clock>,
    pub client_id: String,
    pub current_user_id: RwLock<Option<String>>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(
        store: Store,
        tokens: TokenStore,
        client_id: impl Into<String>,
        data_dir: PathBuf,
    ) -> Self {
        let client_id = client_id.into();
        let tokens = Arc::new(tokens);
        let spotify = Arc::new(SpotifyClient::new(client_id.clone(), tokens.clone()));
        Self {
            store,
            tokens,
            spotify,
            clock: Arc::new(SystemClock),
            client_id,
            current_user_id: RwLock::new(None),
            data_dir,
        }
    }
}
