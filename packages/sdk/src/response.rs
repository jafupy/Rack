use std::{error, fmt};

use serde::{Deserialize, Serialize};

pub type Result<T = Response> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidStatus(u16),
    Json(String),
    Message(String),
}

impl Error {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    fn invalid_status(status: u16) -> Self {
        Self::InvalidStatus(status)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStatus(status) => write!(f, "invalid HTTP status: {status}"),
            Self::Json(error) => write!(f, "JSON error: {error}"),
            Self::Message(message) => f.write_str(message),
        }
    }
}

impl error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self::try_new(status, body).expect("response status must be a valid HTTP status")
    }

    pub fn try_new(status: u16, body: impl Into<Vec<u8>>) -> Result<Self> {
        validate_status(status)?;
        Ok(Self {
            status,
            headers: Vec::new(),
            body: body.into(),
        })
    }

    pub fn ok(body: impl Into<String>) -> Self {
        Self::text(body)
    }

    pub fn created(body: impl Into<String>) -> Self {
        Self::text(body).status(201)
    }

    pub fn bad_request(body: impl Into<String>) -> Self {
        Self::text(body).status(400)
    }

    pub fn teapot(body: impl Into<String>) -> Self {
        Self::text(body).status(418)
    }

    pub fn text(body: impl Into<String>) -> Self {
        Self::new(200, body.into()).header("content-type", "text/plain; charset=utf-8")
    }

    pub fn html(body: impl Into<String>) -> Self {
        Self::new(200, body.into()).header("content-type", "text/html; charset=utf-8")
    }

    pub fn json(value: impl Serialize) -> Result<Self> {
        let body = serde_json::to_vec(&value)?;
        Ok(Self::new(200, body).header("content-type", "application/json"))
    }

    pub fn csv(body: impl Into<String>) -> Self {
        Self::new(200, body.into()).header("content-type", "text/csv; charset=utf-8")
    }

    pub fn bytes(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, body)
    }

    pub fn empty(status: u16) -> Self {
        Self::new(status, Vec::new())
    }

    pub fn no_content() -> Self {
        Self::empty(204)
    }

    pub fn not_found(body: impl Into<String>) -> Self {
        Self::text(body).status(404)
    }

    pub fn internal_server_error(body: impl Into<String>) -> Self {
        Self::text(body).status(500)
    }

    pub fn status(mut self, status: u16) -> Self {
        self.status = validate_status(status).expect("response status must be a valid HTTP status");
        self
    }

    pub fn try_status(mut self, status: u16) -> Result<Self> {
        self.status = validate_status(status)?;
        Ok(self)
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl<T, E> IntoResponse for std::result::Result<T, E>
where
    T: IntoResponse,
    E: fmt::Display,
{
    fn into_response(self) -> Response {
        match self {
            Ok(response) => response.into_response(),
            Err(error) => Response::internal_server_error(error.to_string()),
        }
    }
}

fn validate_status(status: u16) -> Result<u16> {
    if (100..=599).contains(&status) {
        Ok(status)
    } else {
        Err(Error::invalid_status(status))
    }
}
