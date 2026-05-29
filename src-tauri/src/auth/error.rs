use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("token endpoint returned {status}: {body}")]
    TokenEndpoint { status: u16, body: String },

    #[error("missing field in response: {0}")]
    MissingField(&'static str),

    #[error("keyring error: {0}")]
    Keyring(String),

    #[error("auth flow cancelled")]
    Cancelled,
}

impl From<keyring_core::Error> for AuthError {
    fn from(e: keyring_core::Error) -> Self {
        AuthError::Keyring(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AuthError>;
