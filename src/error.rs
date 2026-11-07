//! Shared error type for `ziziphus`.
//!
//! `ZiziphusError` covers the application's common failure modes.
//!
//! In `main`, a `ZiziphusError` is printed as `error: <message>` and
//! the process exits with code 1.

use thiserror::Error;

/// Unified error type for `ziziphus`.
///
/// Library errors retain their original source so callers can inspect
/// the underlying failure when necessary.
#[derive(Debug, Error)]
pub enum ZiziphusError {
    /// Generic application error.
    #[error("{0}")]
    Msg(String),

    /// I/O error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// SQLite error.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON serialization / deserialization error.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// Anyhow error.
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl ZiziphusError {
    /// Construct a generic message error.
    pub fn msg<S: Into<String>>(message: S) -> Self {
        Self::Msg(message.into())
    }
}
