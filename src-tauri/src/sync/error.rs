use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("store error: {0}")]
    Store(#[from] crate::store::StoreError),

    #[error("spotify error: {0}")]
    Spotify(#[from] crate::spotify::SpotifyError),

    #[error("source kind {0:?} not supported by sync engine")]
    UnsupportedSource(crate::store::SourceKind),
}

pub type Result<T> = std::result::Result<T, SyncError>;
