# Context — Spotify Archivist

Glossary for shared language. Implementation details belong in code/ADRs, not here.

## Core terms

### Liked Songs
Spotify's system-managed saved-tracks list. Populated via heart button. Accessed via Web API endpoint `/me/tracks`. Not a user playlist — has no playlist ID.

### User Playlist
Playlist created and owned by the authenticated user. Has a playlist ID. Accessed via `/playlists/{id}/tracks`.

### Tracked Source
A list the archivist has been told to watch. In scope: Liked Songs and User Playlists. Out of scope: followed playlists owned by others (Discover Weekly, Release Radar, friends' playlists).

### Sync
A single pass that fetches the current state of every Tracked Source and compares it against the last known state.

### Sync Schedule
The single global interval on which the app performs Syncs while running. Default 6 hours; configurable from 1h to 24h via the UI. Applies uniformly to every Tracked Source — there is no per-source cadence. The app runs as a system tray application: Syncs continue on schedule whenever the tray icon is alive, even with the main window closed. Closing the tray icon stops Syncs.

Triggers in addition to the interval: app start, OS wake-from-sleep (debounced to at most once per 10 minutes), and a manual Sync button in the UI. Offline failures are silently logged; only after 3 consecutive failures is the user notified.

### Disappearance
A track present in a Tracked Source on a prior Sync, no longer present in usable form on the current Sync. Two observable shapes:

- **Tombstone** — playlist item still occupies a slot but the `track` object is null or its metadata (title, artist) is stripped. Unambiguous Spotify-side removal.
- **Vanished** — item entirely absent from the source. Cause ambiguous; could be Spotify-side or user-initiated.

### Loss
A Disappearance attributed to Spotify, not the user. Tombstones are Losses immediately and unambiguously. Vanished items are promoted to Loss only after a second consecutive Sync also fails to find them — this absorbs transient API glitches and partial fetches. User-initiated removals are explicitly **not** Losses and are not surfaced.

### Pending Vanish
A track that was present on Sync N-1 and absent on Sync N, but has not yet been seen absent on a second consecutive Sync. Held in a quarantine state. Promoted to Loss on the next absence; cancelled if the track reappears.

### Archive
The local store of every track ever observed in any Tracked Source, including ones that have since disappeared. The archive is append-only in spirit — disappeared tracks are flagged, not deleted.

### Track Snapshot
Per-track captured fields: Spotify track ID, URI, track name, artist names, album name, `added_at` timestamp. Sufficient to manually re-find a Lost track via title + artist search. Audio features, ISRC, popularity, and album art are deliberately not captured.

The Snapshot is **global per Spotify track ID** — stored once, regardless of how many Tracked Sources contain that track. Membership rows reference the Snapshot. The Removal Flag lives on the membership, not the Snapshot, so a track Lost from one source while still present in another keeps its global metadata intact.

### Store
Single-layer storage. SQLite is the sole system of record — every Track Snapshot, every Sync, every Removal Flag lives there. There is no continuous JSON mirror.

### Export
A manual user action that dumps the current contents of the Store to a `.jsonl` file (one Track Snapshot per line, including the Removal Flag). The user chooses scope at export time: a single Tracked Source or all Tracked Sources combined. Exports are write-only artefacts — the app never reads them back; they exist for backup, git, and external tooling.

### Source View
The single primary UI surface. For a selected Tracked Source, displays its tracks as virtualized rows in a Spotify-like layout. Rows for tracks marked Lost are shown inline, visually de-emphasized, with a removed-icon. A filter toggle switches between "all", "present only", and "removed only". No separate Lost tab, no chronological feed, no per-album report.

### Auth Flow
OAuth 2.0 Authorization Code with PKCE. The app starts a transient HTTP listener on a random loopback port (`http://127.0.0.1:<port>/callback`) for the duration of the login. Spotify redirects there with the auth code; the app exchanges it for tokens and shuts the listener down. No client secret is embedded in the binary. Refresh tokens live in the OS keyring, never on disk.

### Loss Notification
Two-channel signal raised when a Sync confirms one or more new Losses. (1) A single OS toast per Sync, coalesced across all sources — never one toast per track. Clicking the toast opens the relevant Source View. (2) An unread badge on the tray icon, set when any unseen Loss exists, cleared once the user has opened a Source View that contains the unseen Losses.

### Removal Flag
A boolean per `(source, track)` pair. The Store does not persist when the removal happened — only that it did. This collapses Loss reporting to a single bit per row and makes time-based queries ("what disappeared this week") impossible by design.
