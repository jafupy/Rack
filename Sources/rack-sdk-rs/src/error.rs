use std::fmt;
use std::io;

/// Error type used by SDK helpers and macro-generated wrappers.
///
/// Route and cron handlers normally return [`Response`] directly. Helper
/// functions can return `rack::Result<T>` so `?` can bubble failures into the
/// generated wrapper.
#[derive(Debug)]
pub struct Error {
    message: String,
}

/// SDK result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create an SDK error from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Self::new(error)
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Self::new(error)
    }
}
