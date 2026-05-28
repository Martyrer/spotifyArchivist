# Decision Log

Frozen outcomes from the design grilling session on 2026-05-28. Each row links to the artefact (CONTEXT.md term or ADR) that captures the *why*.

| # | Decision | Choice | Captured in |
|---|---|---|---|
| 1 | Tracked Source scope | Liked Songs + user-owned playlists only | [CONTEXT.md → Tracked Source](../CONTEXT.md) |
| 2 | Disappearance taxonomy | Tombstone vs Vanished | [CONTEXT.md → Disappearance](../CONTEXT.md) |
| 3 | Loss confirmation policy | Tombstone immediate, Vanished after 2 consecutive Syncs | [ADR 0002](./adr/0002-two-sync-loss-confirmation.md), [CONTEXT.md → Loss / Pending Vanish](../CONTEXT.md) |
| 4 | Track metadata depth | Minimal — id, uri, name, artists, album, added_at | [CONTEXT.md → Track Snapshot](../CONTEXT.md) |
| 5 | Runtime shape | Tray app, sync continues while tray alive | [CONTEXT.md → Sync Schedule](../CONTEXT.md) |
| 6 | Tech stack | Tauri 2 + React + TypeScript + Tailwind + Bun + Vite+ | this doc |
| 7 | Storage | SQLite is the sole system of record | [CONTEXT.md → Store](../CONTEXT.md) |
| 8 | Schema shape | Flat `memberships` with boolean flags, no `removed_at` | [ADR 0001 + amendment](./adr/0001-bitemporal-membership-store.md), [CONTEXT.md → Removal Flag](../CONTEXT.md) |
| 9 | Sync schedule | Global cadence, default 6h, configurable 1–24h | [CONTEXT.md → Sync Schedule](../CONTEXT.md) |
| 10 | Auth | OAuth 2.0 Authorization Code with PKCE via loopback redirect | [CONTEXT.md → Auth Flow](../CONTEXT.md) |
| 11 | UI surface | Single Source View, virtualized rows, removal-icon, filter pill | [CONTEXT.md → Source View](../CONTEXT.md) |
| 12 | Test coverage | 100% on Rust + TS non-component code, >80% on React components, E2E smoke gate | this doc |
| 13 | Export | Manual user action, JSONL, scope-pickable | [CONTEXT.md → Export](../CONTEXT.md) |
| 14 | Notifications | Coalesced OS toast per Sync + persistent tray badge | [CONTEXT.md → Loss Notification](../CONTEXT.md) |
| 15 | Cloud sync | Dropped — single-device personal tool | this doc |
| 16 | Onboarding flow | Linear: Welcome → Source picker → Source View | this doc |
| 17 | Track identity | Global per Spotify track ID, single `tracks` row across sources | [CONTEXT.md → Track Snapshot](../CONTEXT.md) |
| 18 | Artists column | JSON column on `tracks`, not a normalized join table | this doc |

## Stack details (decision 6)

| Layer | Pick |
|---|---|
| Shell | Tauri 2 |
| Frontend framework | React 19 + TypeScript |
| Build / test / lint / fmt | Vite+ (Vite, Vitest, Oxlint, Oxfmt, Rolldown, tsdown, Vite Task) |
| Package manager / runtime | Bun |
| Styling | Tailwind 4 + latest CSS (view transitions, container queries) |
| UI primitives | shadcn/ui (Radix + Tailwind copy-paste) |
| Icons | lucide-react |
| Routing | TanStack Router |
| Server state | TanStack Query |
| Client state | Zustand |
| Validation | Zod (IPC boundary) |
| Virtualization | TanStack Virtual |
| Dates | date-fns |
| Rust HTTP | reqwest with rustls-tls |
| Spotify SDK | rspotify |
| Async runtime | tokio |
| DB | sqlx (sqlite, runtime-tokio-rustls) |
| Scheduling | tokio-cron-scheduler |
| Token storage | keyring crate (OS keychain) |
| Logging | tracing + tracing-subscriber |
| Error types | thiserror (libs) + anyhow (boundaries) |
| IPC type-gen | specta + tauri-specta |
| Tauri plugins | sql, notification, autostart, single-instance, deep-link, log, store, updater |
| Test fixtures | wiremock crate, recorded Spotify responses |
| Coverage | cargo-llvm-cov (Rust), Vitest coverage (TS) |
| E2E | Playwright against built Tauri binary |

## Test coverage policy (decision 12)

- **Rust crates** (`auth`, `spotify`, `store`, `sync`, `commands`): 100% line + branch coverage. Mock Spotify at HTTP boundary via `wiremock` with committed fixture JSON. Use in-memory SQLite for store tests.
- **TS non-component code** (`lib/`, `hooks/`, formatters, reducers): 100% lines via Vitest.
- **React components**: >80% lines. 100% on visual components is brittle; not pursued.
- **E2E** (Playwright): smoke gate only — first-run flow, sync, removed-icon visible. Not counted in coverage thresholds.

## Onboarding flow (decision 16)

1. **Welcome** — single button "Login with Spotify" → kicks off PKCE loopback.
2. **Source picker** — list of user playlists with checkboxes. Liked Songs is auto-included and not togglable on first run.
3. **Source View** — first sync runs in background; user lands on the first selected source with a small "syncing…" indicator instead of a blocking modal.

Returning user with existing auth + sources skips screens 1–2 and lands directly on the most recently viewed source.

## Cloud sync (decision 15)

Dropped entirely. No native cloud client, no encryption layer, no settings hook. Users wanting offsite backup put `app_data_dir` under Dropbox/OneDrive/Syncthing themselves, or use the manual JSONL Export with a private git repo. Re-evaluating this decision means a new ADR.
