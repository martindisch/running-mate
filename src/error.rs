//! Error types.

use std::{error, fmt};

/// The error type wrapping various library errors.
#[derive(Debug)]
pub enum InternalError {
    /// Tokio's `JoinError`.
    Tokio(tokio::task::JoinError),
    /// MongoDB's `Error`.
    Mongo(mongodb::error::Error),
}

impl From<tokio::task::JoinError> for InternalError {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::Tokio(e)
    }
}

impl From<mongodb::error::Error> for InternalError {
    fn from(e: mongodb::error::Error) -> Self {
        Self::Mongo(e)
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Tokio(e) => e.fmt(f),
            Self::Mongo(e) => e.fmt(f),
        }
    }
}

impl error::Error for InternalError {}
