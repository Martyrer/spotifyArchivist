use std::sync::Arc;

use sqlx::Acquire;

use super::diff::{apply_diff, DiffPlan};
use super::error::{Result, SyncError};
use crate::spotify::{
    classify_playlist, classify_saved, FetchedItem, SpotifyClient,
};
use crate::store::{Membership, Source, SourceKind, Store, Track};

pub trait Clock: Send + Sync {
    fn now_iso(&self) -> String;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_iso(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SyncOutcome {
    pub source_id: i64,
    pub newly_lost: Vec<String>,
    pub newly_pending: Vec<String>,
    pub cleared_pending: Vec<String>,
    pub total_present: usize,
}

pub struct Syncer {
    store: Store,
    spotify: Arc<SpotifyClient>,
    clock: Arc<dyn Clock>,
}

impl Syncer {
    pub fn new(store: Store, spotify: Arc<SpotifyClient>, clock: Arc<dyn Clock>) -> Self {
        Self { store, spotify, clock }
    }

    pub async fn sync_source(&self, source: &Source) -> Result<SyncOutcome> {
        if !source.enabled {
            return Ok(SyncOutcome {
                source_id: source.id,
                newly_lost: vec![],
                newly_pending: vec![],
                cleared_pending: vec![],
                total_present: 0,
            });
        }

        let fetched = self.fetch(source).await?;
        let now = self.clock.now_iso();
        let existing = current_memberships(&self.store, source.id).await?;
        let outcome = apply_diff(source.id, fetched, &existing, &now);

        commit_plan(&self.store, &outcome.plan).await?;

        Ok(SyncOutcome {
            source_id: source.id,
            newly_lost: outcome.plan.newly_lost,
            newly_pending: outcome.plan.newly_pending,
            cleared_pending: outcome.plan.cleared_pending,
            total_present: outcome.seen_track_ids.len(),
        })
    }

    async fn fetch(&self, source: &Source) -> Result<Vec<FetchedItem>> {
        match source.kind {
            SourceKind::LikedSongs => {
                let saved = self.spotify.liked_songs().await?;
                Ok(saved.into_iter().map(classify_saved).collect())
            }
            SourceKind::Playlist => {
                if source.spotify_id.is_empty() || source.spotify_id == "__self__" {
                    return Err(SyncError::UnsupportedSource(source.kind));
                }
                let items = self.spotify.playlist_items(&source.spotify_id).await?;
                Ok(items.into_iter().filter_map(classify_playlist).collect())
            }
        }
    }
}

async fn current_memberships(store: &Store, source_id: i64) -> Result<Vec<Membership>> {
    let rows: Vec<Membership> = sqlx::query_as::<_, Membership>(
        "SELECT source_id, track_id, added_at, position, is_removed, pending_vanish
         FROM memberships WHERE source_id = ?",
    )
    .bind(source_id)
    .fetch_all(store.pool())
    .await
    .map_err(crate::store::StoreError::from)?;
    Ok(rows)
}

async fn commit_plan(store: &Store, plan: &DiffPlan) -> Result<()> {
    let mut conn = store
        .pool()
        .acquire()
        .await
        .map_err(crate::store::StoreError::from)?;
    let mut tx = conn
        .begin()
        .await
        .map_err(crate::store::StoreError::from)?;

    for t in &plan.tracks_to_upsert {
        upsert_track(&mut tx, t).await?;
    }
    for m in &plan.memberships_to_upsert {
        upsert_membership(&mut tx, m).await?;
    }
    tx.commit().await.map_err(crate::store::StoreError::from)?;
    Ok(())
}

async fn upsert_track(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    t: &Track,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO tracks (id, uri, name, artists, album, first_seen_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             uri = excluded.uri,
             name = excluded.name,
             artists = excluded.artists,
             album = excluded.album",
    )
    .bind(&t.id)
    .bind(&t.uri)
    .bind(&t.name)
    .bind(&t.artists)
    .bind(&t.album)
    .bind(&t.first_seen_at)
    .execute(&mut **tx)
    .await
    .map_err(crate::store::StoreError::from)?;
    Ok(())
}

async fn upsert_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    m: &Membership,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO memberships (source_id, track_id, added_at, position, is_removed, pending_vanish)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (source_id, track_id) DO UPDATE SET
             added_at = excluded.added_at,
             position = excluded.position,
             is_removed = excluded.is_removed,
             pending_vanish = excluded.pending_vanish",
    )
    .bind(m.source_id)
    .bind(&m.track_id)
    .bind(&m.added_at)
    .bind(m.position)
    .bind(m.is_removed)
    .bind(m.pending_vanish)
    .execute(&mut **tx)
    .await
    .map_err(crate::store::StoreError::from)?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::Mutex;

    pub struct FixedClock(pub Mutex<String>);

    impl FixedClock {
        pub fn new(s: &str) -> Self {
            Self(Mutex::new(s.to_string()))
        }
    }

    impl Clock for FixedClock {
        fn now_iso(&self) -> String {
            self.0.lock().unwrap().clone()
        }
    }

    #[test]
    fn system_clock_returns_iso_string() {
        let s = SystemClock.now_iso();
        assert!(s.contains('T'));
    }

    #[tokio::test]
    async fn current_memberships_returns_all_rows() {
        let store = Store::open_in_memory().await.unwrap();
        let id = store
            .upsert_source(SourceKind::Playlist, Some("p"), "P")
            .await
            .unwrap();
        store
            .upsert_track(&Track {
                id: "t1".into(),
                uri: "spotify:track:t1".into(),
                name: "One".into(),
                artists: "[]".into(),
                album: "Alb".into(),
                first_seen_at: "2026-01-01T00:00:00Z".into(),
            })
            .await
            .unwrap();
        store
            .upsert_membership(&Membership {
                source_id: id,
                track_id: "t1".into(),
                added_at: "2026-01-01T00:00:00Z".into(),
                position: 0,
                is_removed: false,
                pending_vanish: false,
            })
            .await
            .unwrap();
        let rows = current_memberships(&store, id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].track_id, "t1");
    }

    #[tokio::test]
    async fn commit_plan_writes_tracks_and_memberships_atomically() {
        let store = Store::open_in_memory().await.unwrap();
        let sid = store
            .upsert_source(SourceKind::Playlist, Some("p"), "P")
            .await
            .unwrap();
        let plan = DiffPlan {
            tracks_to_upsert: vec![Track {
                id: "t1".into(),
                uri: "spotify:track:t1".into(),
                name: "One".into(),
                artists: "[]".into(),
                album: "Alb".into(),
                first_seen_at: "2026-01-01T00:00:00Z".into(),
            }],
            memberships_to_upsert: vec![Membership {
                source_id: sid,
                track_id: "t1".into(),
                added_at: "2026-01-01T00:00:00Z".into(),
                position: 0,
                is_removed: false,
                pending_vanish: false,
            }],
            ..Default::default()
        };
        commit_plan(&store, &plan).await.unwrap();
        let rows = store
            .list_rows(sid, crate::store::MembershipFilter::All)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "One");
    }
}
