use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpotifyError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("API returned {status}: {body}")]
    Api { status: u16, body: String },

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::AuthError),

    #[error("rate limit exceeded after {tries} retries")]
    RateLimited { tries: u32 },
}

pub type Result<T> = std::result::Result<T, SpotifyError>;
