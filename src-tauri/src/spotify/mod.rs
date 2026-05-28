pub mod client;
pub mod dto;
pub mod error;

pub use client::{classify_playlist, classify_saved, SpotifyClient};
pub use dto::*;
pub use error::SpotifyError;
