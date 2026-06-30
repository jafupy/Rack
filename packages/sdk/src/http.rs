use serde::{Deserialize, Serialize};

use crate::{Payload, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request<T = ()> {
    pub method: String,
    pub path: String,
    pub host: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Vec<u8>,
    #[serde(skip)]
    payload: T,
}

impl Request<()> {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        host: impl Into<String>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            host: host.into(),
            query: String::new(),
            headers: Vec::new(),
            body: Vec::new(),
            payload: (),
        }
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
}

impl<T> Request<T> {
    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn into_payload(self) -> T {
        self.payload
    }

    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

impl Request<()> {
    pub fn parse_payload<T: Payload>(self) -> Result<Request<T>> {
        let payload = T::from_body(&self.body)?;
        Ok(Request {
            method: self.method,
            path: self.path,
            host: self.host,
            query: self.query,
            headers: self.headers,
            body: self.body,
            payload,
        })
    }
}
