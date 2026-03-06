/// Unified error types for the Kraken CLI.
///
/// Error categories cover API responses, authentication failures, network
/// issues, rate-limiting, validation, configuration, WebSocket errors,
/// I/O, and response parsing problems.
use std::fmt;

/// Top-level error categories used in JSON error envelopes and exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Api,
    Auth,
    Network,
    RateLimit,
    Validation,
    Config,
    WebSocket,
    Io,
    Parse,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Api => "api",
            Self::Auth => "auth",
            Self::Network => "network",
            Self::RateLimit => "rate_limit",
            Self::Validation => "validation",
            Self::Config => "config",
            Self::WebSocket => "websocket",
            Self::Io => "io",
            Self::Parse => "parse",
        };
        f.write_str(s)
    }
}

/// The primary error type for all CLI operations.
#[derive(Debug, thiserror::Error)]
pub enum KrakenError {
    #[error("{message}")]
    Api {
        category: ErrorCategory,
        message: String,
    },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl KrakenError {
    /// Returns the error category for JSON envelope output.
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Api { category, .. } => *category,
            Self::Auth(_) => ErrorCategory::Auth,
            Self::Network(_) => ErrorCategory::Network,
            Self::RateLimit(_) => ErrorCategory::RateLimit,
            Self::Validation(_) => ErrorCategory::Validation,
            Self::Config(_) => ErrorCategory::Config,
            Self::WebSocket(_) => ErrorCategory::WebSocket,
            Self::Io(_) => ErrorCategory::Io,
            Self::Parse(_) => ErrorCategory::Parse,
            Self::Other(_) => ErrorCategory::Api,
        }
    }

    /// Constructs an API error from a Kraken error string (e.g. "EAPI:Invalid key").
    pub fn from_kraken_error(msg: &str) -> Self {
        if msg.starts_with("EAPI:Rate limit") || msg.starts_with("EService:Throttled") {
            return Self::RateLimit(msg.to_string());
        }
        if msg.starts_with("EGeneral:Permission") || msg.starts_with("EAPI:Invalid key") {
            return Self::Auth(msg.to_string());
        }
        Self::Api {
            category: ErrorCategory::Api,
            message: msg.to_string(),
        }
    }

    /// Builds the JSON error envelope: `{"error":"<category>","message":"<detail>"}`.
    pub fn to_json_envelope(&self) -> serde_json::Value {
        serde_json::json!({
            "error": self.category().to_string(),
            "message": self.to_string(),
        })
    }
}

impl From<reqwest::Error> for KrakenError {
    fn from(err: reqwest::Error) -> Self {
        Self::Network(err.to_string())
    }
}

impl From<serde_json::Error> for KrakenError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<url::ParseError> for KrakenError {
    fn from(err: url::ParseError) -> Self {
        Self::Validation(format!("Invalid URL: {err}"))
    }
}

impl From<toml::de::Error> for KrakenError {
    fn from(err: toml::de::Error) -> Self {
        Self::Config(format!("TOML parse error: {err}"))
    }
}

impl From<base64::DecodeError> for KrakenError {
    fn from(err: base64::DecodeError) -> Self {
        Self::Auth(format!("Base64 decode error: {err}"))
    }
}

pub type Result<T> = std::result::Result<T, KrakenError>;
