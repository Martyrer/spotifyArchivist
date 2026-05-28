# ADR 0001 — Membership Store (was: Bitemporal)

- Status: Superseded in part — see Amendment 2026-05-28
- Date: 2026-05-28

## Context

The archivist must answer two distinct questions over time:

1. What is currently in each Tracked Source?
2. What was in each Tracked Source at any past point in time, and what was Lost?

Three storage shapes were considered:

- **Snapshot-per-sync** — write the full membership of each source on every Sync. Trivial queries, but row count grows as `tracks × syncs`. At a daily Sync over a 10,000-track library, ~3.65M rows per year.
- **Event-sourced** — record only ADDED / REMOVED / TOMBSTONED events. Tiny on disk, but every "state at date D" query requires replay or a materialized view, and a single missed REMOVED event silently corrupts membership.
- **Hybrid bitemporal memberships** — one row per `(source, track)` interval, with `added_at` and nullable `removed_at`. Open intervals (`removed_at IS NULL`) define current membership; closed intervals describe history. SCD Type 2 / temporal table pattern.

## Decision

Use the bitemporal `memberships` table as the system of record for "track × source × time", with companion tables for `tracks`, `sources`, `syncs`, and `losses`.

Each Sync writes only the diff: closes intervals for vanished/tombstoned tracks and opens intervals for newly added ones.

## Consequences

**Positive**

- Steady-state row count is O(unique track-membership intervals), not O(syncs × tracks).
- "Current membership of source X" = `WHERE source_id = X AND removed_at IS NULL`.
- "Membership of source X on date D" = `WHERE added_at <= D AND (removed_at IS NULL OR removed_at > D)`.
- Loss reporting reduces to joining `memberships` (closed intervals) with `losses` rows that classify how each closure happened (Tombstone vs probed-Vanished).

**Negative**

- More invariants to enforce in code than a snapshot table. Each Sync must atomically close removed intervals, open new ones, and emit Loss rows in a single transaction; partial writes corrupt history.
- Restating an interval after a bug (e.g. a Sync wrongly closed an interval) requires careful UPDATEs, not idempotent re-inserts.
- Writers must be serialized per source — no concurrent Syncs of the same source.

**Mitigations**

- Wrap each per-source Sync in a single SQLite transaction.
- Add a CHECK constraint that `removed_at IS NULL OR removed_at >= added_at`.
- Add a unique partial index on `(source_id, track_id) WHERE removed_at IS NULL` to forbid two open intervals for the same membership.

## Amendment — 2026-05-28

The user explicitly rejected storing removal timestamps. The Source View has no time-based filtering or sorting; only a removed-or-not boolean is needed.

The schema collapses to:

```sql
sources(id, kind, spotify_id, name)
syncs(id, source_id, started_at, finished_at, status)
tracks(id, uri, name, artists, album, first_seen_at)
memberships(source_id, track_id, added_at, is_removed BOOLEAN, pending_vanish BOOLEAN)
PRIMARY KEY (source_id, track_id)
```

`added_at` is retained because it comes from Spotify itself (`playlist_track.added_at`) and is needed to render rows in the same order as Spotify's UI. `removed_at` is dropped. Loss state reduces to two booleans: `is_removed` (confirmed) and `pending_vanish` (one Sync of quarantine).

The bitemporal motivation — answering "membership at past date D" — is no longer a requirement. The `memberships` table becomes a flat per-pair record, not an interval store. ADR 0002's two-Sync confirmation still works: `pending_vanish` is set on first absence and promoted to `is_removed` on second consecutive absence.

Tradeoffs accepted by the user:

- No "what disappeared this week" query.
- Cannot retroactively distinguish recent Losses from years-old Losses.
- Re-appearance after Loss simply clears `is_removed` with no audit trail.
