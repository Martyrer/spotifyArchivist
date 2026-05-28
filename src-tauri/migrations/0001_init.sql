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
    artists TEXT NOT NULL,
    album TEXT NOT NULL,
    first_seen_at TEXT NOT NULL
);

CREATE TABLE memberships (
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL REFERENCES tracks(id),
    added_at TEXT NOT NULL,
    position INTEGER NOT NULL,
    is_removed INTEGER NOT NULL DEFAULT 0 CHECK (is_removed IN (0,1)),
    pending_vanish INTEGER NOT NULL DEFAULT 0 CHECK (pending_vanish IN (0,1)),
    PRIMARY KEY (source_id, track_id)
);
CREATE INDEX idx_memberships_source ON memberships(source_id, position);
CREATE INDEX idx_memberships_removed ON memberships(source_id, is_removed);

CREATE TABLE syncs (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES sources(id),
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL CHECK (status IN ('running','ok','failed')),
    error TEXT
);
CREATE INDEX idx_syncs_source ON syncs(source_id, started_at);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO settings (key, value) VALUES
    ('sync_interval_hours', '6'),
    ('consecutive_failures', '0');
