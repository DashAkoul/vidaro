//! Error types shared across the application.

use std::fmt;

/// Unified error type. Serialises to a plain string so Tauri commands can
/// return it directly to the frontend.
#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Network(String),
    NotFound(String),
    Invalid(String),
    Tool(String),
    Cancelled,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "{e}"),
            AppError::Json(e) => write!(f, "invalid JSON: {e}"),
            AppError::Network(m) => write!(f, "network error: {m}"),
            AppError::NotFound(m) => write!(f, "not found: {m}"),
            AppError::Invalid(m) => write!(f, "{m}"),
            AppError::Tool(m) => write!(f, "{m}"),
            AppError::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::error::Error for AppError {}

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::Tool(format!("archive error: {e}"))
    }
}

impl From<String> for AppError {
    fn from(e: String) -> Self {
        AppError::Tool(e)
    }
}

impl From<&str> for AppError {
    fn from(e: &str) -> Self {
        AppError::Tool(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
