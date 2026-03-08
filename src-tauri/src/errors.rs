use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SSH error: {0}")]
    Ssh(#[from] ssh2::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Session not found")]
    SessionNotFound,
    #[error("Terminal not found")]
    TerminalNotFound,
    #[error("Authentication failed")]
    AuthFailed,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn state_lock_poisoned() -> AppError {
    AppError::InvalidInput("state lock poisoned".to_string())
}
