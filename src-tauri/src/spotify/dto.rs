use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PageEnvelope<T> {
    pub items: Vec<T>,
    pub next: Option<String>,
    #[serde(default)]
    pub total: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SavedTrack {
    pub added_at: String,
    pub track: Option<SpotifyTrack>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlaylistItem {
    pub added_at: Option<String>,
    pub track: Option<SpotifyTrack>,
    #[serde(default)]
    pub is_local: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SpotifyTrack {
    pub id: Option<String>,
    pub uri: Option<String>,
    pub name: String,
    pub artists: Vec<TrackArtist>,
    pub album: TrackAlbum,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TrackArtist {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TrackAlbum {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SimplePlaylist {
    pub id: String,
    pub name: String,
    pub owner: PlaylistOwner,
    #[serde(default)]
    pub tracks: Option<PlaylistTracksRef>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlaylistOwner {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlaylistTracksRef {
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CurrentUser {
    pub id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchedItem {
    Track {
        added_at: String,
        track: SpotifyTrack,
    },
    Tombstone {
        added_at: String,
    },
}
