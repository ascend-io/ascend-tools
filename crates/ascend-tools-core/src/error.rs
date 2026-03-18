use std::time::SystemTimeError;

/// Result type for the Ascend SDK.
pub type Result<T> = std::result::Result<T, Error>;

/// Public error type for the Ascend SDK.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("{field} is required, set {env_var} or pass --{flag}")]
    MissingConfig {
        field: String,
        env_var: String,
        flag: String,
    },

    #[error("failed to decode service account key from base64")]
    InvalidServiceAccountKeyEncoding,

    #[error("service account key must be 32 bytes (Ed25519 seed), got {got}")]
    InvalidServiceAccountKeyLength { got: usize },

    #[error("expected 32-byte Ed25519 seed, got {got} bytes")]
    InvalidEd25519SeedLength { got: usize },

    #[error("failed to sign JWT")]
    JwtSignFailed {
        #[source]
        source: jsonwebtoken::errors::Error,
    },

    #[error(
        "internal synchronization error: {name} mutex poisoned, client state may be inconsistent — recreate client"
    )]
    MutexPoisoned { name: &'static str },

    #[error("system clock before Unix epoch")]
    SystemClockBeforeUnixEpoch {
        #[source]
        source: SystemTimeError,
    },

    #[error("{context}: {source}")]
    RequestFailed {
        context: String,
        #[source]
        source: ureq::Error,
    },

    #[error("failed to read response body for {context}: {source}")]
    ResponseReadFailed {
        context: String,
        #[source]
        source: ureq::Error,
    },

    #[error("failed to parse JSON for {context}: {source}")]
    JsonParseFailed {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to serialize JSON for {context}: {source}")]
    JsonSerializeFailed {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("missing `{field}` in {context}")]
    MissingField {
        context: &'static str,
        field: &'static str,
    },

    #[error("API error (HTTP {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error(
        "workspace/deployment is paused, use --resume (CLI) or resume=True (SDK) to resume before running"
    )]
    RuntimePaused,

    #[error("workspace/deployment is starting, not yet ready to accept flow runs")]
    RuntimeStarting,

    #[error("workspace/deployment is in error state and cannot run flows")]
    RuntimeInErrorState,

    #[error("workspace/deployment health is '{health}', expected 'running'")]
    RuntimeUnexpectedHealth { health: String },

    #[error("workspace/deployment has no health status, it may be initializing")]
    RuntimeHealthMissing,

    #[error("no {kind} found with title '{title}'")]
    NotFound { kind: String, title: String },

    #[error("multiple {kind}s found with title '{title}', use --uuid to specify one: {}", .matches.iter().map(|(uuid, title)| format!("{uuid} ({title})")).collect::<Vec<_>>().join(", "))]
    AmbiguousTitle {
        kind: String,
        title: String,
        matches: Vec<(String, String)>,
    },

    #[error("SSE stream error: {context}")]
    SseParseError { context: String },
}

impl Error {
    /// Returns the HTTP status code if this is an API error, or `None` otherwise.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            Self::ApiError { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns `true` if this is an HTTP 401 Unauthorized error.
    pub fn is_unauthorized(&self) -> bool {
        self.http_status() == Some(401)
    }

    /// Returns `true` if this is an HTTP 403 Forbidden error.
    pub fn is_forbidden(&self) -> bool {
        self.http_status() == Some(403)
    }

    /// Returns `true` if this is an HTTP 404 Not Found error or a title-based `NotFound`.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. }) || self.http_status() == Some(404)
    }
}

pub(crate) trait UreqResultExt<T> {
    fn with_request_context(self, context: impl Into<String>) -> Result<T>;
    fn with_response_read_context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> UreqResultExt<T> for std::result::Result<T, ureq::Error> {
    fn with_request_context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|source| Error::RequestFailed {
            context: context.into(),
            source,
        })
    }

    fn with_response_read_context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|source| Error::ResponseReadFailed {
            context: context.into(),
            source,
        })
    }
}

pub(crate) trait JsonResultExt<T> {
    fn with_json_parse_context(self, context: impl Into<String>) -> Result<T>;
    fn with_json_serialize_context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> JsonResultExt<T> for std::result::Result<T, serde_json::Error> {
    fn with_json_parse_context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|source| Error::JsonParseFailed {
            context: context.into(),
            source,
        })
    }

    fn with_json_serialize_context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|source| Error::JsonSerializeFailed {
            context: context.into(),
            source,
        })
    }
}
