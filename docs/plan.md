# Implementation Plan

Phased scaffolding plan for Spotify Archivist. Each phase is one PR-sized chunk. Stop at each gate, verify (tests green, manual smoke), proceed.

## Repo skeleton

```
spotifyArchivist/
├── CONTEXT.md                       # glossary
├── README.md
├── docs/
│   ├── decisions.md                 # decision log
│   ├── plan.md                      # this file
│   └── adr/
│       ├── 0001-bitemporal-membership-store.md
│       └── 0002-two-sync-loss-confirmation.md
├── .gitignore                        # Tauri + Node + Rust
├── .editorconfig
├── package.json                      # bun + viteplus
├── bun.lockb
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.ts
├── postcss.config.js
├── index.html
├── src/                              # React app
│   ├── main.tsx
│   ├── App.tsx
│   ├── routes/
│   ├── components/
│   ├── hooks/
│   ├── lib/
│   └── styles/
├── src-tauri/                        # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   ├── migrations/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── auth/
│       ├── spotify/
│       ├── store/
│       ├── sync/
│       └── commands/
├── tests/                            # Playwright E2E
└── .github/workflows/ci.yml
```

## Phases

### Phase 0 — Repo skeleton

Write `README.md`, `.gitignore`, `.editorconfig`. Confirm directory layout.

### Phase 1 — Bootstrap toolchain

- `bun init` then install Vite+.
- Scaffold Tauri 2 with `bun create tauri-app`, prune to React + TS.
- Add Tailwind 4 native pipeline.
- Frontend deps: `@tanstack/react-query`, `@tanstack/react-router`, `@tanstack/react-virtual`, `zod`, `zustand`, `lucide-react`, `date-fns`, `class-variance-authority`, `tailwind-merge`.
- shadcn/ui init.
- Rust deps: `tauri 2`, `tauri-plugin-{sql,notification,autostart,single-instance,deep-link,log,store,updater}`, `rspotify`, `reqwest` (rustls), `tokio`, `sqlx` (sqlite, runtime-tokio-rustls), `keyring`, `tokio-cron-scheduler`, `tracing`, `tracing-subscriber`, `thiserror`, `anyhow`, `serde`, `serde_json`, `specta`, `tauri-specta`.
- Dev deps: `wiremock`, `cargo-llvm-cov`, `playwright`, `vitest`.

Gate: `bun run tauri dev` opens an empty window; `cargo test` and `vitest run` both green on stub tests.

### Phase 2 — DB layer

sqlx migration `0001_init.sql`:

```sql
CREATE TABLE sources (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('liked_songs','playlist')),
  spotify_id TEXT,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  UNIQUE (kind, spotify_id)
);

CREATE TABLE tracks (
  id TEXT PRIMARY KEY,
  uri TEXT NOT NULL,
  name TEXT NOT NULL,
  artists TEXT NOT NULL,            -- JSON array
  album TEXT NOT NULL,
  first_seen_at TEXT NOT NULL
);

CREATE TABLE memberships (
  source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  track_id TEXT NOT NULL REFERENCES tracks(id),
  added_at TEXT NOT NULL,           -- from spotify
  position INTEGER NOT NULL,
  is_removed INTEGER NOT NULL DEFAULT 0,
  pending_vanish INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (source_id, track_id)
);
CREATE INDEX idx_memberships_source ON memberships(source_id, position);

CREATE TABLE syncs (
  id INTEGER PRIMARY KEY,
  source_id INTEGER NOT NULL REFERENCES sources(id),
  started_at TEXT NOT NULL,
  finished_at TEXT,
  status TEXT NOT NULL CHECK (status IN ('running','ok','failed')),
  error TEXT
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

Repository module wraps sqlx pool. One transaction per Sync per source.

Gate: integration tests against in-memory SQLite cover migrations + repository round-trip; 100% coverage on `store/`.

### Phase 3 — Spotify client + auth

- `auth/pkce.rs` — code_verifier / challenge generation, loopback listener (random port via `tokio::net::TcpListener::bind("127.0.0.1:0")`), code-for-token exchange, refresh-token persistence in `keyring`.
- `spotify/client.rs` — paginated fetchers for `/me/tracks`, `/me/playlists`, `/playlists/{id}/tracks`. Honors `Retry-After` on 429. On 401, refresh and retry once.
- DTOs typed via `serde`. Validate at IPC boundary on TS side via Zod.
- Tests use `wiremock` with committed fixture JSON.

Gate: tests cover happy path, 401 refresh, 429 backoff, pagination; 100% on `auth/` + `spotify/`.

### Phase 4 — Sync engine

`sync/mod.rs` orchestrates one source at a time inside a single SQLite transaction.

Diff algorithm:

1. Fetch full current set from Spotify.
2. Compare against `memberships` rows where `is_removed = 0`.
3. For each track in the current set: upsert track, upsert membership, clear both `is_removed` and `pending_vanish` if reappeared, update `position`.
4. For each track missing from the current set:
   - If response shape is a Tombstone → set `is_removed = 1`, clear `pending_vanish`.
   - Else (Vanished):
     - `pending_vanish = 0` → set `pending_vanish = 1`.
     - `pending_vanish = 1` → set `is_removed = 1`, clear `pending_vanish`.

Scheduler: `tokio-cron-scheduler` driven by `settings.sync_interval_hours` (default 6). Reschedule on settings change. Triggered also by app start, OS wake event (debounced 10 min), and manual button. After 3 consecutive failures, raise an error notification.

Gate: scenario tests for first sync, no-op resync, single Tombstone, single Vanished promoted across 2 syncs, Vanished cancelled by reappearance; 100% on `sync/`.

### Phase 5 — IPC commands

Tauri commands exposed to TS via `tauri-specta`:

```rust
list_sources() -> Vec<Source>
toggle_source(id, enabled)
list_memberships(source_id, filter: All | Present | Removed) -> Vec<Row>
trigger_sync()
get_settings() -> Settings
update_settings(patch)
start_login() -> ()
logout()
export_jsonl(scope: Source(id) | All, path) -> ()
```

`Row` carries flattened track + membership fields for the virtualized list.

Gate: generated TS bindings compile; commands exercised via `tauri::test` harness.

### Phase 6 — Frontend shell

- `App.tsx` with TanStack Router routes: `/login`, `/onboarding/sources`, `/source/:id`, `/settings`.
- Tray icon via `tauri-plugin-tray`. Menu: Open, Sync now, Quit. Badge dot when unseen Losses.
- Single-instance plugin focuses existing window on relaunch.
- Autostart plugin opt-in toggle in settings.

Gate: app launches into login route; tray icon present; relaunch focuses existing instance.

### Phase 7 — Source View

- `useMemberships(sourceId, filter)` — TanStack Query.
- TanStack Virtual rows mirror Spotify desktop layout: `[#] [cover-placeholder] [title / artist] [album] [added] [removed-icon]`.
- Filter pill: All / Present / Removed.
- Removed rows: `opacity-50 grayscale`, lucide `Ghost` icon, `aria-label="removed by Spotify"`.
- Filter switch animated via the CSS view transitions API — native, no JS animation lib.

Gate: virtualized list renders 10k stub rows smoothly; filter toggle visibly works; component coverage >80%.

### Phase 8 — Onboarding

- `/login` — single button → `start_login()` → poll auth state via `useQuery`.
- `/onboarding/sources` — checkbox grid of user playlists; Liked Songs auto-checked-and-locked. Submit enables sources, fires first sync, navigates to first source.
- Skip if already authed and sources exist; returning users land on the most recently viewed source.

Gate: full first-run flow works end to end against mocked Spotify.

### Phase 9 — Notifications

- After each Sync, count newly promoted-to-Loss memberships; if > 0, emit one OS toast: "N tracks lost — open archivist".
- Tray badge `set_icon` to badged variant when `unseen_loss_count > 0`. Cleared on Source View open of an affected source.

Gate: simulated Loss raises exactly one toast; badge appears and clears as specified.

### Phase 10 — Export

- Settings page button "Export JSONL". Modal: scope picker (single source / all). File save dialog.
- Rust streams rows via `serde_json::to_writer` line-by-line so big libraries do not blow memory.

Gate: 50k-row export completes under fixed memory; output is valid JSONL.

### Phase 11 — Tests

- Rust: `cargo llvm-cov --fail-under-lines 100 --fail-under-branches 100` on `auth`, `spotify`, `store`, `sync`, `commands`.
- TS: `vitest run --coverage` thresholds — 100 on `lib/` and `hooks/`, 80 on `components/`.
- E2E: Playwright spec — boots app, mocks Spotify HTTP via fixture server, runs full first-run flow, asserts row + removed-icon visible.

Gate: all coverage gates green in CI.

### Phase 12 — CI / dist

- GitHub Actions matrix: ubuntu-latest, macos-latest, windows-latest.
- Steps: cache cargo + bun, `bun install`, `cargo fmt --check`, `cargo clippy -- -D warnings`, oxlint, vitest, llvm-cov, `bun run tauri build`, upload artifacts.
- `tauri-plugin-updater` configured but no signing keys yet — slot in for distribution.

Gate: green build on all three OSes.

## Out-of-band prerequisites

- Spotify Developer dashboard: register application, add redirect URI `http://127.0.0.1:*`, capture client ID. No client secret — PKCE.
- Choose app name. Cannot include "Spotify" per Spotify Developer Terms. Candidates: Archivist, Curator, Vinyl.
