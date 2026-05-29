use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// True while any sync (manual, tray, or scheduled) is running. Lets the UI
    /// show an in-progress indicator even for syncs it did not itself trigger,
    /// and reflect an already-running sync when a view first mounts.
    pub sync_in_progress: AtomicBool,
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
            sync_in_progress: AtomicBool::new(false),
        }
    }

    /// Mark a sync as started. Returns a guard that clears the flag on drop, so
    /// the flag is correct even if the sync returns early or panics. Returns
    /// `None` if a sync is already running (caller should not start a second).
    pub fn begin_sync(&self) -> Option<SyncGuard<'_>> {
        if self.sync_in_progress.swap(true, Ordering::SeqCst) {
            return None;
        }
        Some(SyncGuard {
            flag: &self.sync_in_progress,
        })
    }
}

/// Clears `sync_in_progress` when dropped.
pub struct SyncGuard<'a> {
    flag: &'a AtomicBool,
}

impl Drop for SyncGuard<'_> {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}
