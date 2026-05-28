use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use serde::de::DeserializeOwned;

use super::dto::*;
use super::error::{Result, SpotifyError};
use crate::auth::{refresh_token, TokenSet, TokenStore, SPOTIFY_TOKEN_URL};

const DEFAULT_API_BASE: &str = "https://api.spotify.com/v1";
const MAX_REFRESH_RETRIES: u32 = 1;
const MAX_RATE_LIMIT_RETRIES: u32 = 5;

#[derive(Clone)]
pub struct SpotifyClient {
    http: reqwest::Client,
    api_base: String,
    token_url: String,
    client_id: String,
    tokens: Arc<TokenStore>,
    /// Cached current access token; reloaded from `tokens` when None.
    cached: Arc<Mutex<Option<TokenSet>>>,
}

impl SpotifyClient {
    pub fn new(client_id: impl Into<String>, tokens: Arc<TokenStore>) -> Self {
        Self::with_endpoints(client_id, tokens, DEFAULT_API_BASE, SPOTIFY_TOKEN_URL)
    }

    pub fn with_endpoints(
        client_id: impl Into<String>,
        tokens: Arc<TokenStore>,
        api_base: impl Into<String>,
        token_url: impl Into<String>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client build");
        Self {
            http,
            api_base: api_base.into(),
            token_url: token_url.into(),
            client_id: client_id.into(),
            tokens,
            cached: Arc::new(Mutex::new(None)),
        }
    }

    async fn current_token(&self) -> Result<TokenSet> {
        let mut guard = self.cached.lock().await;
        if let Some(t) = guard.as_ref() {
            return Ok(t.clone());
        }
        let loaded = self.tokens.load()?.ok_or_else(|| SpotifyError::Api {
            status: 401,
            body: "no stored token".into(),
        })?;
        *guard = Some(loaded.clone());
        Ok(loaded)
    }

    async fn refresh(&self) -> Result<TokenSet> {
        let current = self.current_token().await?;
        let new = refresh_token(
            &self.http,
            &self.token_url,
            &self.client_id,
            &current.refresh_token,
        )
        .await?;
        self.tokens.save(&new)?;
        let mut guard = self.cached.lock().await;
        *guard = Some(new.clone());
        Ok(new)
    }

    async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut refresh_attempts = 0;
        let mut rate_attempts = 0;
        loop {
            let token = self.current_token().await?;
            let res = self
                .http
                .get(url)
                .bearer_auth(&token.access_token)
                .send()
                .await?;
            let status = res.status();
            if status.as_u16() == 401 && refresh_attempts < MAX_REFRESH_RETRIES {
                refresh_attempts += 1;
                self.refresh().await?;
                continue;
            }
            if status.as_u16() == 429 {
                if rate_attempts >= MAX_RATE_LIMIT_RETRIES {
                    return Err(SpotifyError::RateLimited {
                        tries: rate_attempts,
                    });
                }
                rate_attempts += 1;
                let retry_after = res
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(1)
                    .min(30);
                tokio::time::sleep(Duration::from_secs(retry_after)).await;
                continue;
            }
            if !status.is_success() {
                let body = res.text().await.unwrap_or_default();
                return Err(SpotifyError::Api {
                    status: status.as_u16(),
                    body,
                });
            }
            let parsed = res.json::<T>().await?;
            return Ok(parsed);
        }
    }

    pub async fn current_user(&self) -> Result<CurrentUser> {
        self.get_json(&format!("{}/me", self.api_base)).await
    }

    pub async fn liked_songs(&self) -> Result<Vec<SavedTrack>> {
        let mut url = format!("{}/me/tracks?limit=50", self.api_base);
        let mut acc = Vec::new();
        loop {
            let page: PageEnvelope<SavedTrack> = self.get_json(&url).await?;
            acc.extend(page.items);
            match page.next {
                Some(next) => url = next,
                None => break,
            }
        }
        Ok(acc)
    }

    pub async fn user_playlists(&self, owner_id: &str) -> Result<Vec<SimplePlaylist>> {
        let mut url = format!("{}/me/playlists?limit=50", self.api_base);
        let mut acc = Vec::new();
        loop {
            let page: PageEnvelope<SimplePlaylist> = self.get_json(&url).await?;
            for pl in page.items {
                if pl.owner.id == owner_id {
                    acc.push(pl);
                }
            }
            match page.next {
                Some(next) => url = next,
                None => break,
            }
        }
        Ok(acc)
    }

    pub async fn playlist_items(&self, playlist_id: &str) -> Result<Vec<PlaylistItem>> {
        let mut url = format!(
            "{}/playlists/{}/tracks?limit=100",
            self.api_base, playlist_id
        );
        let mut acc = Vec::new();
        loop {
            let page: PageEnvelope<PlaylistItem> = self.get_json(&url).await?;
            acc.extend(page.items.into_iter().filter(|i| !i.is_local));
            match page.next {
                Some(next) => url = next,
                None => break,
            }
        }
        Ok(acc)
    }
}

/// Convert a Spotify saved-track or playlist-item into the diff-engine's
/// `FetchedItem`, collapsing tombstones into a dedicated variant so the
/// upstream sync code does not need to inspect raw API shapes.
pub fn classify_saved(t: SavedTrack) -> FetchedItem {
    classify(t.added_at, t.track)
}

pub fn classify_playlist(item: PlaylistItem) -> Option<FetchedItem> {
    if item.is_local {
        return None;
    }
    Some(classify(
        item.added_at
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".into()),
        item.track,
    ))
}

fn classify(added_at: String, track: Option<SpotifyTrack>) -> FetchedItem {
    match track {
        Some(t) if t.id.is_some() && !t.name.is_empty() && !t.artists.is_empty() => {
            FetchedItem::Track { added_at, track: t }
        }
        _ => FetchedItem::Tombstone { added_at },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::TokenSet;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn token() -> TokenSet {
        TokenSet {
            access_token: "AT".into(),
            refresh_token: "RT".into(),
            expires_in: 3600,
            token_type: "Bearer".into(),
            scope: "user-library-read".into(),
        }
    }

    fn client_for(server: &MockServer, store: TokenStore) -> SpotifyClient {
        store.save(&token()).unwrap();
        SpotifyClient::with_endpoints(
            "CID",
            Arc::new(store),
            server.uri(),
            format!("{}/api/token", server.uri()),
        )
    }

    #[tokio::test]
    async fn current_user_round_trips() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .and(header("authorization", "Bearer AT"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "u1",
                "display_name": "User One"
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, TokenStore::memory());
        let u = c.current_user().await.unwrap();
        assert_eq!(u.id, "u1");
    }

    #[tokio::test]
    async fn refresh_on_401_then_retry_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .and(header("authorization", "Bearer AT"))
            .respond_with(ResponseTemplate::new(401).set_body_string("expired"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "AT2",
                "refresh_token": "RT2",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .and(header("authorization", "Bearer AT2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "u1"})))
            .mount(&server)
            .await;
        let c = client_for(&server, TokenStore::memory());
        let u = c.current_user().await.unwrap();
        assert_eq!(u.id, "u1");
    }

    #[tokio::test]
    async fn double_401_returns_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(401).set_body_string("expired"))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "AT2",
                "refresh_token": "RT2",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, TokenStore::memory());
        let err = c.current_user().await.unwrap_err();
        match err {
            SpotifyError::Api { status, .. } => assert_eq!(status, 401),
            o => panic!("expected Api 401, got {o:?}"),
        }
    }

    #[tokio::test]
    async fn rate_limit_respects_retry_after_then_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "0")
                    .set_body_string("slow down"),
            )
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"u1"})))
            .mount(&server)
            .await;
        let c = client_for(&server, TokenStore::memory());
        let u = c.current_user().await.unwrap();
        assert_eq!(u.id, "u1");
    }

    #[tokio::test]
    async fn liked_songs_paginates() {
        let server = MockServer::start().await;
        let next = format!("{}/me/tracks?offset=50", server.uri());
        Mock::given(method("GET"))
            .and(path("/me/tracks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"added_at": "2026-01-01T00:00:00Z", "track": {
                        "id":"t1","uri":"spotify:track:t1","name":"One",
                        "artists":[{"id":"a1","name":"A1"}],
                        "album":{"id":"al1","name":"Alb"}}}
                ],
                "next": next
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/me/tracks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"added_at": "2026-01-02T00:00:00Z", "track": {
                        "id":"t2","uri":"spotify:track:t2","name":"Two",
                        "artists":[{"id":"a2","name":"A2"}],
                        "album":{"id":"al2","name":"Alb2"}}}
                ],
                "next": null
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, TokenStore::memory());
        let items = c.liked_songs().await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].track.as_ref().unwrap().id.as_deref(), Some("t1"));
        assert_eq!(items[1].track.as_ref().unwrap().id.as_deref(), Some("t2"));
    }

    #[tokio::test]
    async fn user_playlists_filters_by_owner() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/me/playlists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id":"pl1","name":"Mine","owner":{"id":"u1"},"tracks":{"total":10}},
                    {"id":"pl2","name":"Theirs","owner":{"id":"u2"},"tracks":{"total":20}}
                ],
                "next": null
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, TokenStore::memory());
        let pls = c.user_playlists("u1").await.unwrap();
        assert_eq!(pls.len(), 1);
        assert_eq!(pls[0].id, "pl1");
    }

    #[tokio::test]
    async fn playlist_items_skips_local_tracks() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/playlists/pl1/tracks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"added_at":"2026-01-01T00:00:00Z","is_local":false,"track":{
                        "id":"t1","uri":"spotify:track:t1","name":"One",
                        "artists":[{"id":"a1","name":"A1"}],
                        "album":{"id":"al1","name":"Alb"}}},
                    {"added_at":"2026-01-02T00:00:00Z","is_local":true,"track":null}
                ],
                "next": null
            })))
            .mount(&server)
            .await;
        let c = client_for(&server, TokenStore::memory());
        let items = c.playlist_items("pl1").await.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn classify_saved_returns_track_for_full_payload() {
        let saved = SavedTrack {
            added_at: "2026-01-01T00:00:00Z".into(),
            track: Some(SpotifyTrack {
                id: Some("t1".into()),
                uri: Some("spotify:track:t1".into()),
                name: "One".into(),
                artists: vec![TrackArtist {
                    id: Some("a".into()),
                    name: "A".into(),
                }],
                album: TrackAlbum {
                    id: Some("al".into()),
                    name: "Alb".into(),
                },
            }),
        };
        match classify_saved(saved) {
            FetchedItem::Track { track, .. } => assert_eq!(track.id.as_deref(), Some("t1")),
            other => panic!("expected Track, got {other:?}"),
        }
    }

    #[test]
    fn classify_saved_returns_tombstone_when_track_null() {
        let saved = SavedTrack {
            added_at: "2026-01-01T00:00:00Z".into(),
            track: None,
        };
        assert!(matches!(
            classify_saved(saved),
            FetchedItem::Tombstone { .. }
        ));
    }

    #[test]
    fn classify_playlist_returns_none_for_local() {
        let item = PlaylistItem {
            added_at: Some("2026-01-01T00:00:00Z".into()),
            track: None,
            is_local: true,
        };
        assert!(classify_playlist(item).is_none());
    }

    #[test]
    fn classify_playlist_returns_tombstone_for_stripped_metadata() {
        let item = PlaylistItem {
            added_at: Some("2026-01-01T00:00:00Z".into()),
            track: Some(SpotifyTrack {
                id: None,
                uri: None,
                name: "".into(),
                artists: vec![],
                album: TrackAlbum {
                    id: None,
                    name: "".into(),
                },
            }),
            is_local: false,
        };
        assert!(matches!(
            classify_playlist(item).unwrap(),
            FetchedItem::Tombstone { .. }
        ));
    }
}
