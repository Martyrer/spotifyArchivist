use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("invalid setting value for {key}: {value}")]
    InvalidSetting { key: String, value: String },

    #[error("source not found: {0}")]
    SourceNotFound(i64),
}

pub type Result<T> = std::result::Result<T, StoreError>;
