use std::collections::{HashMap, HashSet};

use crate::spotify::{FetchedItem, SpotifyTrack};
use crate::store::{Membership, Track};

/// What the engine plans to write to the store after diffing one source.
///
/// The plan is computed without touching the database so it can be unit-tested
/// in isolation. The `engine` module then applies it inside one transaction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffPlan {
    pub tracks_to_upsert: Vec<Track>,
    pub memberships_to_upsert: Vec<Membership>,
    pub newly_lost: Vec<String>,
    pub newly_pending: Vec<String>,
    pub cleared_pending: Vec<String>,
    pub total_present: usize,
}

pub fn apply_diff(
    source_id: i64,
    fetched: Vec<FetchedItem>,
    existing: &[Membership],
    now: &str,
) -> DiffPlan {
    let existing_by_track: HashMap<&str, &Membership> =
        existing.iter().map(|m| (m.track_id.as_str(), m)).collect();

    let mut plan = DiffPlan::default();
    let mut seen_track_ids = HashSet::new();
    let mut tombstoned_existing: HashSet<String> = HashSet::new();
    let mut next_position: i64 = 0;

    for item in fetched {
        match item {
            FetchedItem::Track { added_at, track } => {
                let Some(track_id) = track.id.clone() else {
                    next_position += 1;
                    continue;
                };
                if !seen_track_ids.insert(track_id.clone()) {
                    next_position += 1;
                    continue;
                }

                let track_row = build_track(&track_id, &track, now);
                plan.tracks_to_upsert.push(track_row);

                let prior = existing_by_track.get(track_id.as_str()).copied();
                let was_pending = prior.map(|m| m.pending_vanish).unwrap_or(false);
                if was_pending {
                    plan.cleared_pending.push(track_id.clone());
                }
                let m = Membership {
                    source_id,
                    track_id: track_id.clone(),
                    added_at,
                    position: next_position,
                    is_removed: false,
                    pending_vanish: false,
                };
                plan.memberships_to_upsert.push(m);
                next_position += 1;
            }
            FetchedItem::Tombstone { added_at: _ } => {
                if let Some(prior) = find_unique_existing_for_position(existing, next_position) {
                    if !prior.is_removed {
                        plan.newly_lost.push(prior.track_id.clone());
                    }
                    let mut m = prior.clone();
                    m.position = next_position;
                    m.is_removed = true;
                    m.pending_vanish = false;
                    tombstoned_existing.insert(m.track_id.clone());
                    plan.memberships_to_upsert.push(m);
                }
                next_position += 1;
            }
        }
    }

    for prior in existing {
        if seen_track_ids.contains(&prior.track_id) {
            continue;
        }
        if tombstoned_existing.contains(&prior.track_id) {
            continue;
        }
        if prior.is_removed {
            continue;
        }
        if prior.pending_vanish {
            plan.newly_lost.push(prior.track_id.clone());
            let m = Membership {
                is_removed: true,
                pending_vanish: false,
                ..prior.clone()
            };
            plan.memberships_to_upsert.push(m);
        } else {
            plan.newly_pending.push(prior.track_id.clone());
            let m = Membership {
                pending_vanish: true,
                ..prior.clone()
            };
            plan.memberships_to_upsert.push(m);
        }
    }

    plan.total_present = seen_track_ids.len();
    plan
}

fn build_track(id: &str, t: &SpotifyTrack, now: &str) -> Track {
    let artists_json = serde_json::to_string(&t.artists).unwrap_or_else(|_| "[]".to_string());
    Track {
        id: id.to_string(),
        uri: t.uri.clone().unwrap_or_default(),
        name: t.name.clone(),
        artists: artists_json,
        album: t.album.name.clone(),
        first_seen_at: now.to_string(),
    }
}

fn find_unique_existing_for_position(
    existing: &[Membership],
    position: i64,
) -> Option<&Membership> {
    let mut found: Option<&Membership> = None;
    for m in existing {
        if m.position == position && !m.is_removed {
            if found.is_some() {
                return None;
            }
            found = Some(m);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spotify::{TrackAlbum, TrackArtist};

    fn st(id: &str, name: &str) -> SpotifyTrack {
        SpotifyTrack {
            id: Some(id.into()),
            uri: Some(format!("spotify:track:{id}")),
            name: name.into(),
            artists: vec![TrackArtist {
                id: Some("a".into()),
                name: "A".into(),
            }],
            album: TrackAlbum {
                id: Some("al".into()),
                name: "Alb".into(),
            },
        }
    }

    fn fetched(id: &str, name: &str) -> FetchedItem {
        FetchedItem::Track {
            added_at: "2026-01-01T00:00:00Z".into(),
            track: st(id, name),
        }
    }

    fn member(track_id: &str, position: i64) -> Membership {
        Membership {
            source_id: 1,
            track_id: track_id.into(),
            added_at: "2026-01-01T00:00:00Z".into(),
            position,
            is_removed: false,
            pending_vanish: false,
        }
    }

    #[test]
    fn first_sync_with_no_history_inserts_everything() {
        let res = apply_diff(
            1,
            vec![fetched("t1", "One"), fetched("t2", "Two")],
            &[],
            "now",
        );
        assert_eq!(res.tracks_to_upsert.len(), 2);
        assert_eq!(res.memberships_to_upsert.len(), 2);
        assert!(res.newly_lost.is_empty());
        assert!(res.newly_pending.is_empty());
        assert_eq!(res.memberships_to_upsert[0].position, 0);
        assert_eq!(res.memberships_to_upsert[1].position, 1);
    }

    #[test]
    fn unchanged_resync_is_a_noop_for_loss_state() {
        let prior = vec![member("t1", 0), member("t2", 1)];
        let res = apply_diff(
            1,
            vec![fetched("t1", "One"), fetched("t2", "Two")],
            &prior,
            "now",
        );
        assert!(res.newly_lost.is_empty());
        assert!(res.newly_pending.is_empty());
        assert!(res.cleared_pending.is_empty());
    }

    #[test]
    fn vanished_track_first_sync_becomes_pending_not_lost() {
        let prior = vec![member("t1", 0), member("t2", 1)];
        let res = apply_diff(1, vec![fetched("t1", "One")], &prior, "now");
        assert_eq!(res.newly_pending, vec!["t2"]);
        assert!(res.newly_lost.is_empty());
        let updated_t2 = res
            .memberships_to_upsert
            .iter()
            .find(|m| m.track_id == "t2")
            .expect("t2 membership update");
        assert!(updated_t2.pending_vanish);
        assert!(!updated_t2.is_removed);
    }

    #[test]
    fn pending_vanish_promoted_to_lost_on_second_absence() {
        let mut prior = vec![member("t1", 0), member("t2", 1)];
        prior[1].pending_vanish = true;
        let res = apply_diff(1, vec![fetched("t1", "One")], &prior, "now");
        assert_eq!(res.newly_lost, vec!["t2"]);
        assert!(res.newly_pending.is_empty());
        let updated_t2 = res
            .memberships_to_upsert
            .iter()
            .find(|m| m.track_id == "t2")
            .unwrap();
        assert!(updated_t2.is_removed);
        assert!(!updated_t2.pending_vanish);
    }

    #[test]
    fn pending_vanish_cleared_when_track_reappears() {
        let mut prior = vec![member("t1", 0), member("t2", 1)];
        prior[1].pending_vanish = true;
        let res = apply_diff(
            1,
            vec![fetched("t1", "One"), fetched("t2", "Two")],
            &prior,
            "now",
        );
        assert!(res.newly_lost.is_empty());
        assert!(res.newly_pending.is_empty());
        assert_eq!(res.cleared_pending, vec!["t2"]);
        let t2 = res
            .memberships_to_upsert
            .iter()
            .find(|m| m.track_id == "t2")
            .unwrap();
        assert!(!t2.pending_vanish);
        assert!(!t2.is_removed);
    }

    #[test]
    fn tombstone_marks_unique_position_as_lost_immediately() {
        let prior = vec![member("t1", 0), member("t2", 1)];
        let fetched = vec![
            FetchedItem::Track {
                added_at: "2026-01-01T00:00:00Z".into(),
                track: st("t1", "One"),
            },
            FetchedItem::Tombstone {
                added_at: "2026-01-01T00:00:00Z".into(),
            },
        ];
        let res = apply_diff(1, fetched, &prior, "now");
        assert_eq!(res.newly_lost, vec!["t2"]);
        let t2 = res
            .memberships_to_upsert
            .iter()
            .find(|m| m.track_id == "t2")
            .unwrap();
        assert!(t2.is_removed);
    }

    #[test]
    fn tombstone_at_unknown_position_is_skipped() {
        let res = apply_diff(
            1,
            vec![FetchedItem::Tombstone {
                added_at: "2026-01-01T00:00:00Z".into(),
            }],
            &[],
            "now",
        );
        assert!(res.memberships_to_upsert.is_empty());
        assert!(res.newly_lost.is_empty());
    }

    #[test]
    fn duplicate_fetched_track_id_is_deduplicated() {
        let res = apply_diff(
            1,
            vec![fetched("t1", "One"), fetched("t1", "One")],
            &[],
            "now",
        );
        assert_eq!(res.memberships_to_upsert.len(), 1);
        assert_eq!(res.tracks_to_upsert.len(), 1);
    }

    #[test]
    fn already_removed_membership_is_not_re_flagged() {
        let mut prior = vec![member("t1", 0)];
        prior[0].is_removed = true;
        let res = apply_diff(1, vec![], &prior, "now");
        assert!(res.newly_lost.is_empty());
        assert!(res.newly_pending.is_empty());
        assert!(res.memberships_to_upsert.is_empty());
    }

    #[test]
    fn previously_lost_track_reappearing_clears_flag() {
        let mut prior = vec![member("t1", 0)];
        prior[0].is_removed = true;
        let res = apply_diff(1, vec![fetched("t1", "One")], &prior, "now");
        let t1 = res
            .memberships_to_upsert
            .iter()
            .find(|m| m.track_id == "t1")
            .unwrap();
        assert!(!t1.is_removed);
        assert!(!t1.pending_vanish);
    }

    #[test]
    fn fetched_track_without_id_is_skipped() {
        let mut t = st("ignored", "X");
        t.id = None;
        let res = apply_diff(
            1,
            vec![FetchedItem::Track {
                added_at: "2026-01-01T00:00:00Z".into(),
                track: t,
            }],
            &[],
            "now",
        );
        assert!(res.memberships_to_upsert.is_empty());
    }
}
