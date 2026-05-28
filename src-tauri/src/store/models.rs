use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    LikedSongs,
    Playlist,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct Source {
    pub id: i64,
    pub kind: SourceKind,
    pub spotify_id: Option<String>,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub first_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct Membership {
    pub source_id: i64,
    pub track_id: String,
    pub added_at: String,
    pub position: i64,
    pub is_removed: bool,
    pub pending_vanish: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Running,
    Ok,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct SyncRecord {
    pub id: i64,
    pub source_id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: SyncStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MembershipFilter {
    All,
    Present,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct Row {
    pub source_id: i64,
    pub track_id: String,
    pub uri: String,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub added_at: String,
    pub position: i64,
    pub is_removed: bool,
    pub pending_vanish: bool,
}
