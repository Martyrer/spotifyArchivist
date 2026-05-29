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

## Getting started

Prerequisites: [Bun](https://bun.sh), a Rust toolchain, and the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS.

1. **Create a Spotify app.** Go to the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard), create an app, and add `http://127.0.0.1:4202/callback` as a Redirect URI. Copy the **Client ID** (this is a public PKCE client — there is no client secret).

2. **Provide the client id via environment variable.** The app reads it from `SPOTIFY_ARCHIVIST_CLIENT_ID` at startup and will refuse to launch if it is unset. Copy the example file and fill in your id:

   ```sh
   cp .env.example .env
   # edit .env and set SPOTIFY_ARCHIVIST_CLIENT_ID
   ```

   Then export it into your shell (or use a tool like `direnv` / `dotenv`) before running:

   ```sh
   export SPOTIFY_ARCHIVIST_CLIENT_ID=your_client_id_here
   ```

3. **Install and run.**

   ```sh
   bun install
   bun run tauri dev
   ```

To build a release bundle, run `bun run tauri build` with the same environment variable set.

## Documentation

- [`CONTEXT.md`](CONTEXT.md) — domain glossary.
- [`docs/decisions.md`](docs/decisions.md) — frozen design decisions with links to source artefacts.
- [`docs/plan.md`](docs/plan.md) — phased implementation plan.
- [`docs/adr/`](docs/adr/) — architecture decision records.

## Security

Spotify authentication uses OAuth 2.0 + PKCE, so there is no client secret to leak. Access and refresh tokens are stored in the OS keyring, never on disk in plaintext. To report a vulnerability, see [`SECURITY.md`](SECURITY.md).

## License

[MIT](LICENSE).
