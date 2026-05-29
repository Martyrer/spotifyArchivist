use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::app_state::AppState;
use crate::auth::{
    authorize_url, build_pkce, exchange_code, LoopbackListener, LoopbackOutcome, SPOTIFY_TOKEN_URL,
};
use crate::export::{export_jsonl, ExportError};
use crate::spotify::SpotifyError;
use crate::store::{MembershipFilter, Row, Source, SourceKind, StoreError};
use crate::sync::{SyncError, Syncer};

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::AuthError),
    #[error("spotify error: {0}")]
    Spotify(#[from] SpotifyError),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("sync error: {0}")]
    Sync(#[from] SyncError),
    #[error("export error: {0}")]
    Export(#[from] ExportError),
    #[error("not authenticated")]
    NotAuthenticated,
    #[error("login state mismatch")]
    StateMismatch,
}

impl serde::Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CommandError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub sync_interval_hours: u32,
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub onboarded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailablePlaylist {
    pub id: String,
    pub name: String,
    pub already_tracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportScope {
    Source { id: i64 },
    All,
}

pub async fn list_sources(state: &AppState) -> Result<Vec<Source>> {
    Ok(state.store.list_sources().await?)
}

pub async fn toggle_source(state: &AppState, id: i64, enabled: bool) -> Result<()> {
    state.store.set_source_enabled(id, enabled).await?;
    Ok(())
}

pub async fn untrack_source(state: &AppState, id: i64) -> Result<()> {
    state.store.delete_source(id).await?;
    Ok(())
}

pub async fn list_memberships(
    state: &AppState,
    source_id: i64,
    filter: MembershipFilter,
) -> Result<Vec<Row>> {
    Ok(state.store.list_rows(source_id, filter).await?)
}

pub async fn get_settings(state: &AppState) -> Result<Settings> {
    let user_id_fut = async { state.current_user_id.read().await.clone() };
    let (interval, onboarded, user_id) = tokio::try_join!(
        async {
            state
                .store
                .sync_interval_hours()
                .await
                .map_err(CommandError::from)
        },
        async { state.store.is_onboarded().await.map_err(CommandError::from) },
        async { Ok::<_, CommandError>(user_id_fut.await) },
    )?;
    let token = state.tokens.load()?;
    Ok(Settings {
        sync_interval_hours: interval,
        authenticated: token.is_some(),
        user_id,
        onboarded,
    })
}

pub async fn complete_onboarding(state: &AppState) -> Result<()> {
    state.store.set_onboarded(true).await?;
    Ok(())
}

pub async fn update_settings(state: &AppState, sync_interval_hours: u32) -> Result<Settings> {
    state
        .store
        .set_sync_interval_hours(sync_interval_hours)
        .await?;
    get_settings(state).await
}

pub async fn logout(state: &AppState) -> Result<()> {
    state.tokens.clear()?;
    *state.current_user_id.write().await = None;
    state.store.set_onboarded(false).await?;
    Ok(())
}

/// Full reset: wipe all tracked data, clear credentials, and return to first-run.
pub async fn reset_app(state: &AppState) -> Result<()> {
    state.tokens.clear()?;
    *state.current_user_id.write().await = None;
    state.store.reset().await?;
    Ok(())
}

pub async fn list_available_playlists(state: &AppState) -> Result<Vec<AvailablePlaylist>> {
    let user_id = state
        .current_user_id
        .read()
        .await
        .clone()
        .ok_or(CommandError::NotAuthenticated)?;
    let remote = state.spotify.user_playlists(&user_id).await?;
    let local = state.store.list_sources().await?;
    let tracked: std::collections::HashSet<String> = local
        .iter()
        .filter(|s| s.kind == SourceKind::Playlist)
        .map(|s| s.spotify_id.clone())
        .collect();
    Ok(remote
        .into_iter()
        .map(|p| AvailablePlaylist {
            already_tracked: tracked.contains(&p.id),
            id: p.id,
            name: p.name,
        })
        .collect())
}

pub async fn track_playlist(state: &AppState, spotify_id: &str, name: &str) -> Result<i64> {
    Ok(state
        .store
        .upsert_source(SourceKind::Playlist, Some(spotify_id), name)
        .await?)
}

pub async fn ensure_liked_songs_source(state: &AppState) -> Result<i64> {
    Ok(state
        .store
        .upsert_source(SourceKind::LikedSongs, None, "Liked Songs")
        .await?)
}

pub async fn trigger_sync(state: &AppState) -> Result<Vec<crate::sync::SyncOutcome>> {
    let syncer = Syncer::new(
        state.store.clone(),
        state.spotify.clone(),
        state.clock.clone(),
    );
    let sources = state.store.list_sources().await?;
    syncer
        .sync_all(&sources)
        .await
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(CommandError::from)
}

pub async fn export(state: &AppState, scope: ExportScope, path: PathBuf) -> Result<usize> {
    let sources = state.store.list_sources().await?;
    let scoped: Vec<Source> = match scope {
        ExportScope::All => sources,
        ExportScope::Source { id } => sources.into_iter().filter(|s| s.id == id).collect(),
    };
    Ok(export_jsonl(&state.store, &scoped, &path).await?)
}

pub struct StartedLogin {
    pub authorize_url: String,
    pub redirect_uri: String,
    pub listener: LoopbackListener,
    pub verifier: String,
    pub state: String,
}

pub async fn begin_login(state: &AppState) -> Result<StartedLogin> {
    let listener = LoopbackListener::bind().await?;
    let pkce = build_pkce();
    let url = authorize_url(&state.client_id, &listener.redirect_uri, &pkce)?;
    Ok(StartedLogin {
        authorize_url: url.to_string(),
        redirect_uri: listener.redirect_uri.clone(),
        listener,
        verifier: pkce.verifier,
        state: pkce.state,
    })
}

pub async fn finish_login(
    state: &AppState,
    started: StartedLogin,
    timeout: Duration,
) -> Result<Settings> {
    let outcome = started.listener.wait(timeout).await?;
    match outcome {
        LoopbackOutcome::Code {
            code,
            state: returned,
        } => {
            if returned != started.state {
                return Err(CommandError::StateMismatch);
            }
            let http = reqwest::Client::new();
            let tokens = exchange_code(
                &http,
                SPOTIFY_TOKEN_URL,
                &state.client_id,
                &started.redirect_uri,
                &code,
                &started.verifier,
            )
            .await?;
            state.tokens.save(&tokens)?;
            let me = state.spotify.current_user().await?;
            *state.current_user_id.write().await = Some(me.id.clone());
            ensure_liked_songs_source(state).await?;
            get_settings(state).await
        }
        LoopbackOutcome::Error { error, .. } => {
            Err(CommandError::Auth(crate::auth::AuthError::TokenEndpoint {
                status: 0,
                body: error,
            }))
        }
    }
}

pub fn data_dir(state: &AppState) -> &PathBuf {
    &state.data_dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::TokenSet;
    use crate::store::Store;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    async fn fixture() -> AppState {
        let store = Store::open_in_memory().await.unwrap();
        let tokens = crate::auth::TokenStore::memory();
        let arc_tokens = Arc::new(tokens);
        let spotify = Arc::new(crate::spotify::SpotifyClient::new(
            "CID".to_string(),
            arc_tokens.clone(),
        ));
        AppState {
            store,
            tokens: arc_tokens,
            spotify,
            clock: Arc::new(crate::sync::SystemClock),
            client_id: "CID".into(),
            current_user_id: RwLock::new(None),
            data_dir: std::env::temp_dir(),
        }
    }

    #[tokio::test]
    async fn list_sources_empty_initially() {
        let state = fixture().await;
        assert_eq!(list_sources(&state).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn ensure_liked_songs_is_idempotent() {
        let state = fixture().await;
        let a = ensure_liked_songs_source(&state).await.unwrap();
        let b = ensure_liked_songs_source(&state).await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn track_playlist_then_toggle_disable() {
        let state = fixture().await;
        let id = track_playlist(&state, "pl1", "Mix").await.unwrap();
        toggle_source(&state, id, false).await.unwrap();
        let list = list_sources(&state).await.unwrap();
        assert!(!list[0].enabled);
    }

    #[tokio::test]
    async fn list_memberships_passes_filter_through() {
        let state = fixture().await;
        let id = ensure_liked_songs_source(&state).await.unwrap();
        let rows = list_memberships(&state, id, MembershipFilter::All)
            .await
            .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn settings_default_to_not_onboarded() {
        let state = fixture().await;
        let s = get_settings(&state).await.unwrap();
        assert!(!s.onboarded);
    }

    #[tokio::test]
    async fn complete_onboarding_flips_flag() {
        let state = fixture().await;
        complete_onboarding(&state).await.unwrap();
        let s = get_settings(&state).await.unwrap();
        assert!(s.onboarded);
    }

    #[tokio::test]
    async fn ensuring_liked_songs_does_not_imply_onboarded() {
        let state = fixture().await;
        ensure_liked_songs_source(&state).await.unwrap();
        let s = get_settings(&state).await.unwrap();
        assert!(!s.onboarded, "ensure_liked_songs must not set onboarded");
    }

    #[tokio::test]
    async fn logout_resets_onboarded() {
        let state = fixture().await;
        complete_onboarding(&state).await.unwrap();
        logout(&state).await.unwrap();
        let s = get_settings(&state).await.unwrap();
        assert!(!s.onboarded);
    }

    #[tokio::test]
    async fn settings_round_trip() {
        let state = fixture().await;
        let s = update_settings(&state, 4).await.unwrap();
        assert_eq!(s.sync_interval_hours, 4);
        assert!(!s.authenticated);
        let again = get_settings(&state).await.unwrap();
        assert_eq!(again.sync_interval_hours, 4);
    }

    #[tokio::test]
    async fn logout_clears_user_and_tokens() {
        let state = fixture().await;
        state
            .tokens
            .save(&TokenSet {
                access_token: "AT".into(),
                refresh_token: "RT".into(),
                expires_in: 3600,
                token_type: "Bearer".into(),
                scope: "user-library-read".into(),
            })
            .unwrap();
        *state.current_user_id.write().await = Some("u1".into());
        logout(&state).await.unwrap();
        let s = get_settings(&state).await.unwrap();
        assert!(!s.authenticated);
        assert!(s.user_id.is_none());
    }

    #[tokio::test]
    async fn list_available_playlists_requires_auth() {
        let state = fixture().await;
        let err = list_available_playlists(&state).await.unwrap_err();
        assert!(matches!(err, CommandError::NotAuthenticated));
    }

    /// Regression: after restart, tokens persist but `current_user_id` does
    /// not. The startup rehydrate step must populate it; otherwise this
    /// command returns NotAuthenticated even when the user is logged in.
    #[tokio::test]
    async fn list_available_playlists_works_after_rehydrate() {
        let state = fixture().await;
        *state.current_user_id.write().await = Some("u1".into());
        // No real Spotify call possible from a unit test; the guard fires
        // before the HTTP call. Confirm the guard now passes.
        let result = list_available_playlists(&state).await;
        assert!(!matches!(result, Err(CommandError::NotAuthenticated)));
    }

    #[tokio::test]
    async fn begin_login_returns_url_with_state_and_challenge() {
        let state = fixture().await;
        let started = begin_login(&state).await.unwrap();
        assert!(started.authorize_url.contains("client_id=CID"));
        assert!(started
            .authorize_url
            .contains(&format!("state={}", started.state)));
        assert!(started.authorize_url.contains("code_challenge_method=S256"));
        assert_eq!(started.redirect_uri, "http://127.0.0.1:4202/callback");
    }

    #[tokio::test]
    async fn export_all_writes_membership_rows() {
        let state = fixture().await;
        let sid = ensure_liked_songs_source(&state).await.unwrap();
        state
            .store
            .upsert_track(&crate::store::Track {
                id: "t1".into(),
                uri: "spotify:track:t1".into(),
                name: "One".into(),
                artists: r#"[{"id":"a","name":"A"}]"#.into(),
                album: "Alb".into(),
                first_seen_at: "2026-01-01T00:00:00Z".into(),
            })
            .await
            .unwrap();
        state
            .store
            .upsert_membership(&crate::store::Membership {
                source_id: sid,
                track_id: "t1".into(),
                added_at: "2026-01-01T00:00:00Z".into(),
                position: 0,
                is_removed: false,
                pending_vanish: false,
            })
            .await
            .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "archivist-export-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.jsonl");
        let n = export(&state, ExportScope::All, path.clone())
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn export_source_scope_filters() {
        let state = fixture().await;
        let _l = ensure_liked_songs_source(&state).await.unwrap();
        let p = track_playlist(&state, "pl1", "Mix").await.unwrap();
        let dir = std::env::temp_dir().join(format!(
            "archivist-export-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.jsonl");
        let n = export(&state, ExportScope::Source { id: p }, path.clone())
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn data_dir_returns_state_path() {
        let state = fixture().await;
        assert_eq!(data_dir(&state), &state.data_dir);
    }
}
