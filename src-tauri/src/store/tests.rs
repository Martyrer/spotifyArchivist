use super::*;

fn track(id: &str, name: &str) -> Track {
    Track {
        id: id.into(),
        uri: format!("spotify:track:{id}"),
        name: name.into(),
        artists: r#"[{"id":"a1","name":"Artist 1"}]"#.into(),
        album: "Album".into(),
        first_seen_at: "2026-01-01T00:00:00Z".into(),
    }
}

fn membership(sid: i64, tid: &str, pos: i64) -> Membership {
    Membership {
        source_id: sid,
        track_id: tid.into(),
        added_at: "2026-01-01T00:00:00Z".into(),
        position: pos,
        is_removed: false,
        pending_vanish: false,
    }
}

#[tokio::test]
async fn migrations_apply_and_seed_settings() {
    let s = Store::open_in_memory().await.unwrap();
    assert_eq!(s.sync_interval_hours().await.unwrap(), 6);
}

#[tokio::test]
async fn upsert_source_is_idempotent_on_kind_and_spotify_id() {
    let s = Store::open_in_memory().await.unwrap();
    let a = s
        .upsert_source(SourceKind::Playlist, Some("pl1"), "Mix")
        .await
        .unwrap();
    let b = s
        .upsert_source(SourceKind::Playlist, Some("pl1"), "Renamed")
        .await
        .unwrap();
    assert_eq!(a, b);
    let list = s.list_sources().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Renamed");
    assert_eq!(list[0].kind, SourceKind::Playlist);
}

#[tokio::test]
async fn liked_songs_and_playlist_are_distinct_rows() {
    let s = Store::open_in_memory().await.unwrap();
    let l = s
        .upsert_source(SourceKind::LikedSongs, None, "Liked Songs")
        .await
        .unwrap();
    let p = s
        .upsert_source(SourceKind::Playlist, Some("pl1"), "Mix")
        .await
        .unwrap();
    assert_ne!(l, p);
    assert_eq!(s.list_sources().await.unwrap().len(), 2);
}

#[tokio::test]
async fn set_source_enabled_returns_error_for_missing() {
    let s = Store::open_in_memory().await.unwrap();
    let err = s.set_source_enabled(999, false).await.unwrap_err();
    assert!(matches!(err, StoreError::SourceNotFound(999)));
}

#[tokio::test]
async fn set_source_enabled_persists() {
    let s = Store::open_in_memory().await.unwrap();
    let id = s
        .upsert_source(SourceKind::Playlist, Some("pl1"), "Mix")
        .await
        .unwrap();
    s.set_source_enabled(id, false).await.unwrap();
    let list = s.list_sources().await.unwrap();
    assert!(!list[0].enabled);
}

#[tokio::test]
async fn upsert_track_updates_metadata_on_conflict() {
    let s = Store::open_in_memory().await.unwrap();
    let mut t = track("t1", "Old");
    s.upsert_track(&t).await.unwrap();
    t.name = "New".into();
    s.upsert_track(&t).await.unwrap();
    let id = s
        .upsert_source(SourceKind::Playlist, Some("p"), "P")
        .await
        .unwrap();
    s.upsert_membership(&membership(id, "t1", 0)).await.unwrap();
    let rows = s.list_rows(id, MembershipFilter::All).await.unwrap();
    assert_eq!(rows[0].name, "New");
}

#[tokio::test]
async fn list_rows_filter_distinguishes_present_and_removed() {
    let s = Store::open_in_memory().await.unwrap();
    let id = s
        .upsert_source(SourceKind::Playlist, Some("p"), "P")
        .await
        .unwrap();
    s.upsert_track(&track("t1", "One")).await.unwrap();
    s.upsert_track(&track("t2", "Two")).await.unwrap();
    s.upsert_membership(&membership(id, "t1", 0)).await.unwrap();
    let mut m2 = membership(id, "t2", 1);
    m2.is_removed = true;
    s.upsert_membership(&m2).await.unwrap();

    let all = s.list_rows(id, MembershipFilter::All).await.unwrap();
    let present = s.list_rows(id, MembershipFilter::Present).await.unwrap();
    let removed = s.list_rows(id, MembershipFilter::Removed).await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(present.len(), 1);
    assert_eq!(present[0].track_id, "t1");
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].track_id, "t2");
}

#[tokio::test]
async fn list_rows_orders_by_position() {
    let s = Store::open_in_memory().await.unwrap();
    let id = s
        .upsert_source(SourceKind::Playlist, Some("p"), "P")
        .await
        .unwrap();
    s.upsert_track(&track("t1", "One")).await.unwrap();
    s.upsert_track(&track("t2", "Two")).await.unwrap();
    s.upsert_membership(&membership(id, "t2", 0)).await.unwrap();
    s.upsert_membership(&membership(id, "t1", 1)).await.unwrap();
    let rows = s.list_rows(id, MembershipFilter::All).await.unwrap();
    assert_eq!(rows[0].track_id, "t2");
    assert_eq!(rows[1].track_id, "t1");
}

#[tokio::test]
async fn membership_upsert_updates_flags() {
    let s = Store::open_in_memory().await.unwrap();
    let id = s
        .upsert_source(SourceKind::Playlist, Some("p"), "P")
        .await
        .unwrap();
    s.upsert_track(&track("t1", "One")).await.unwrap();
    s.upsert_membership(&membership(id, "t1", 0)).await.unwrap();
    let mut m = membership(id, "t1", 0);
    m.pending_vanish = true;
    s.upsert_membership(&m).await.unwrap();
    let rows = s.list_rows(id, MembershipFilter::All).await.unwrap();
    assert!(rows[0].pending_vanish);
    assert!(!rows[0].is_removed);
}

#[tokio::test]
async fn settings_round_trip_and_clamp() {
    let s = Store::open_in_memory().await.unwrap();
    s.set_sync_interval_hours(0).await.unwrap();
    assert_eq!(s.sync_interval_hours().await.unwrap(), 1);
    s.set_sync_interval_hours(99).await.unwrap();
    assert_eq!(s.sync_interval_hours().await.unwrap(), 24);
    s.set_sync_interval_hours(12).await.unwrap();
    assert_eq!(s.sync_interval_hours().await.unwrap(), 12);
}

#[tokio::test]
async fn settings_invalid_value_errors() {
    let s = Store::open_in_memory().await.unwrap();
    s.put_setting("sync_interval_hours", "not-a-number")
        .await
        .unwrap();
    let err = s.sync_interval_hours().await.unwrap_err();
    assert!(matches!(err, StoreError::InvalidSetting { .. }));
}

#[tokio::test]
async fn delete_source_removes_row_and_memberships() {
    let s = Store::open_in_memory().await.unwrap();
    let id = s
        .upsert_source(SourceKind::Playlist, Some("p"), "P")
        .await
        .unwrap();
    s.upsert_track(&track("t1", "One")).await.unwrap();
    s.upsert_membership(&membership(id, "t1", 0)).await.unwrap();
    s.delete_source(id).await.unwrap();
    assert!(s.list_sources().await.unwrap().is_empty());
    assert!(s
        .list_rows(id, MembershipFilter::All)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn delete_source_returns_error_for_missing() {
    let s = Store::open_in_memory().await.unwrap();
    let err = s.delete_source(999).await.unwrap_err();
    assert!(matches!(err, StoreError::SourceNotFound(999)));
}

#[tokio::test]
async fn reset_wipes_data_and_restores_default_settings() {
    let s = Store::open_in_memory().await.unwrap();
    let id = s
        .upsert_source(SourceKind::Playlist, Some("p"), "P")
        .await
        .unwrap();
    s.upsert_track(&track("t1", "One")).await.unwrap();
    s.upsert_membership(&membership(id, "t1", 0)).await.unwrap();
    s.set_sync_interval_hours(20).await.unwrap();
    s.set_onboarded(true).await.unwrap();
    s.add_unseen_losses(5).await.unwrap();

    s.reset().await.unwrap();

    assert!(s.list_sources().await.unwrap().is_empty());
    assert!(s
        .list_rows(id, MembershipFilter::All)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(s.sync_interval_hours().await.unwrap(), 6);
    assert!(!s.is_onboarded().await.unwrap());
    assert_eq!(s.unseen_losses().await.unwrap(), 0);
}

#[tokio::test]
async fn unseen_losses_round_trip() {
    let s = Store::open_in_memory().await.unwrap();
    assert_eq!(s.unseen_losses().await.unwrap(), 0);
    let after = s.add_unseen_losses(3).await.unwrap();
    assert_eq!(after, 3);
    let after = s.add_unseen_losses(2).await.unwrap();
    assert_eq!(after, 5);
    s.clear_unseen_losses().await.unwrap();
    assert_eq!(s.unseen_losses().await.unwrap(), 0);
}

#[tokio::test]
async fn get_setting_returns_none_for_missing() {
    let s = Store::open_in_memory().await.unwrap();
    assert!(s.get_setting("does-not-exist").await.unwrap().is_none());
}
