# ADR 0002 — Two-Sync Confirmation for Vanished Tracks

- Status: Accepted
- Date: 2026-05-28

## Context

A Sync that completes but missed pages, hit a transient 500, or saw an API blip will look identical to a Sync that correctly observed a track removal. If the archivist promotes every Vanished item to Loss on first observation, those transient errors create false-positive Losses, eroding trust in the archive's central claim ("these tracks were taken from you").

Tombstones are different — they are an explicit signal from Spotify that a track was removed server-side, not absent due to a fetch error. They can be flagged as Loss immediately.

Three policies were considered:

- **Immediate** — any Disappearance becomes a Loss on first observation. Lowest detection lag, highest false-positive risk.
- **All-or-nothing per Sync** — only Syncs that complete fully are eligible to flag Losses. Reduces but does not eliminate false positives, since a "complete" Sync can still ship subtly wrong data.
- **Two-Sync confirmation** — Vanished items enter a Pending Vanish quarantine on first absence. Promotion to Loss requires a second consecutive Sync to also report absence. If the track reappears in the interim, the pending state is cancelled.

## Decision

Use two-Sync confirmation for Vanished items. Tombstones bypass the quarantine and are flagged as Loss immediately.

## Consequences

**Positive**

- Transient API errors, partial fetches, and brief region-availability flickers do not pollute the Loss view.
- The archive's strongest claim — "this track was deliberately taken from you" — is backed by at least two independent observations.

**Negative**

- Loss detection lag equals one Sync interval. A daily Sync schedule means up to ~24 hours between the true removal and the user-visible Loss notification.
- An extra state (Pending Vanish) must be modelled, persisted, and surfaced in the UI distinctly from confirmed Loss.

**Mitigations**

- The Pending Vanish state is itself useful UX: a "checking again on next sync" indicator that signals the system is being careful, not slow.
- Users who want immediate signal can manually trigger a Sync from the UI, which collapses the lag to seconds.
