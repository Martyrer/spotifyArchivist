# Spotify Archivist

A cross-platform tray app that watches your Spotify Liked Songs and your own playlists, captures every track that has ever been in them, and tells you when Spotify silently removes one.

## What it does

- Authenticates against your Spotify account with OAuth 2.0 + PKCE (no client secret, no server).
- Syncs Liked Songs and selected user playlists on a configurable schedule (default every 6 hours).
- Compares each Sync against the previous state and flags any track that disappears.
- Distinguishes Tombstones (Spotify-side removals, flagged immediately) from Vanished items (ambiguous, confirmed after a second consecutive Sync).
- Surfaces Lost tracks inline in the playlist view with a removed-icon and a filter to show "removed only".
- Stores everything in SQLite locally. Manual JSONL export when you want a portable backup.

## What it does not do

- Track playlists you do not own (Discover Weekly, Release Radar, friends' playlists).
- Remember when a track was removed — the archive records that a Loss happened, not when.
- Cloud-sync between devices. Single-device, single-user tool.
- Restore tracks. The archive preserves enough metadata (title, artist, album) to manually re-find a Lost track on Spotify.

## Stack

Tauri 2 + React + TypeScript + Tailwind, built with Bun and Vite+. SQLite via sqlx on the Rust side. Spotify access via `rspotify`. Full stack and rationale: [`docs/decisions.md`](docs/decisions.md).

## Documentation

- [`CONTEXT.md`](CONTEXT.md) — domain glossary.
- [`docs/decisions.md`](docs/decisions.md) — frozen design decisions with links to source artefacts.
- [`docs/plan.md`](docs/plan.md) — phased implementation plan.
- [`docs/adr/`](docs/adr/) — architecture decision records.

## Status

Pre-implementation. Design and documentation complete; code not yet scaffolded.
