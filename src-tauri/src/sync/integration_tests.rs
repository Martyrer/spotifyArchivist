use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::engine::tests::FixedClock;
use super::*;
use crate::auth::{TokenSet, TokenStore};
use crate::spotify::SpotifyClient;
use crate::store::{MembershipFilter, Source, SourceKind, Store};

fn token() -> TokenSet {
    TokenSet {
        access_token: "AT".into(),
        refresh_token: "RT".into(),
        expires_in: 3600,
        token_type: "Bearer".into(),
        scope: "user-library-read".into(),
    }
}

async fn setup() -> (MockServer, Store, Arc<SpotifyClient>) {
    let server = MockServer::start().await;
    let store = Store::open_in_memory().await.unwrap();
    let ts = TokenStore::memory();
    ts.save(&token()).unwrap();
    let client = Arc::new(SpotifyClient::with_endpoints(
        "CID",
        Arc::new(ts),
        server.uri(),
        format!("{}/api/token", server.uri()),
    ));
    (server, store, client)
}

fn liked_track(id: &str, name: &str) -> serde_json::Value {
    json!({
        "added_at": "2026-01-01T00:00:00Z",
        "track": {
            "id": id, "uri": format!("spotify:track:{id}"), "name": name,
            "artists": [{"id":"a1","name":"A1"}],
            "album": {"id":"al","name":"Alb"}
        }
    })
}

fn playlist_item(id: &str, name: &str) -> serde_json::Value {
    json!({
        "added_at": "2026-01-01T00:00:00Z",
        "is_local": false,
        "track": {
            "id": id, "uri": format!("spotify:track:{id}"), "name": name,
            "artists": [{"id":"a1","name":"A1"}],
            "album": {"id":"al","name":"Alb"}
        }
    })
}

#[tokio::test]
async fn first_sync_writes_all_fetched_tracks() {
    let (server, store, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/me/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [liked_track("t1", "One"), liked_track("t2", "Two")],
            "next": null
        })))
        .mount(&server)
        .await;

    let sid = store
        .upsert_source(SourceKind::LikedSongs, None, "Liked Songs")
        .await
        .unwrap();
    let source = Source {
        id: sid,
        kind: SourceKind::LikedSongs,
        spotify_id: "__self__".into(),
        name: "Liked Songs".into(),
        enabled: true,
    };
    let syncer = Syncer::new(
        store.clone(),
        client,
        Arc::new(FixedClock::new("2026-01-01T00:00:00Z")),
    );
    let outcome = syncer.sync_source(&source).await.unwrap();
    assert_eq!(outcome.total_present, 2);
    assert!(outcome.newly_lost.is_empty());
    assert!(outcome.newly_pending.is_empty());

    let rows = store.list_rows(sid, MembershipFilter::All).await.unwrap();
    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn vanished_track_promoted_to_lost_after_two_syncs() {
    let (server, store, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/me/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [liked_track("t1", "One"), liked_track("t2", "Two")],
            "next": null
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/me/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [liked_track("t1", "One")],
            "next": null
        })))
        .mount(&server)
        .await;

    let sid = store
        .upsert_source(SourceKind::LikedSongs, None, "Liked Songs")
        .await
        .unwrap();
    let source = Source {
        id: sid,
        kind: SourceKind::LikedSongs,
        spotify_id: "__self__".into(),
        name: "Liked Songs".into(),
        enabled: true,
    };
    let syncer = Syncer::new(
        store.clone(),
        client,
        Arc::new(FixedClock::new("2026-01-01T00:00:00Z")),
    );

    let first = syncer.sync_source(&source).await.unwrap();
    assert!(first.newly_pending.is_empty());

    let second = syncer.sync_source(&source).await.unwrap();
    assert_eq!(second.newly_pending, vec!["t2"]);
    assert!(second.newly_lost.is_empty());

    let third = syncer.sync_source(&source).await.unwrap();
    assert_eq!(third.newly_lost, vec!["t2"]);
    assert!(third.newly_pending.is_empty());

    let removed = store
        .list_rows(sid, MembershipFilter::Removed)
        .await
        .unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].track_id, "t2");
}

#[tokio::test]
async fn tombstone_in_playlist_marks_loss_immediately() {
    let (server, store, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/playlists/pl1/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [playlist_item("t1", "One"), playlist_item("t2", "Two")],
            "next": null
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/playlists/pl1/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                playlist_item("t1", "One"),
                {"added_at":"2026-01-01T00:00:00Z","is_local":false,"track":null}
            ],
            "next": null
        })))
        .mount(&server)
        .await;

    let sid = store
        .upsert_source(SourceKind::Playlist, Some("pl1"), "Mix")
        .await
        .unwrap();
    let source = Source {
        id: sid,
        kind: SourceKind::Playlist,
        spotify_id: "pl1".into(),
        name: "Mix".into(),
        enabled: true,
    };
    let syncer = Syncer::new(
        store.clone(),
        client,
        Arc::new(FixedClock::new("2026-01-01T00:00:00Z")),
    );
    syncer.sync_source(&source).await.unwrap();
    let outcome = syncer.sync_source(&source).await.unwrap();
    assert_eq!(outcome.newly_lost, vec!["t2"]);
    assert!(outcome.newly_pending.is_empty());
}

#[tokio::test]
async fn disabled_source_is_skipped_without_fetching() {
    let (_server, store, client) = setup().await;
    let sid = store
        .upsert_source(SourceKind::LikedSongs, None, "Liked Songs")
        .await
        .unwrap();
    let source = Source {
        id: sid,
        kind: SourceKind::LikedSongs,
        spotify_id: "__self__".into(),
        name: "Liked Songs".into(),
        enabled: false,
    };
    let syncer = Syncer::new(
        store.clone(),
        client,
        Arc::new(FixedClock::new("2026-01-01T00:00:00Z")),
    );
    let outcome = syncer.sync_source(&source).await.unwrap();
    assert_eq!(outcome.total_present, 0);
}

#[tokio::test]
async fn playlist_source_without_spotify_id_errors() {
    let (_server, store, client) = setup().await;
    let source = Source {
        id: 999,
        kind: SourceKind::Playlist,
        spotify_id: "__self__".into(),
        name: "broken".into(),
        enabled: true,
    };
    let syncer = Syncer::new(
        store.clone(),
        client,
        Arc::new(FixedClock::new("2026-01-01T00:00:00Z")),
    );
    let err = syncer.sync_source(&source).await.unwrap_err();
    assert!(matches!(
        err,
        SyncError::UnsupportedSource(SourceKind::Playlist)
    ));
}
