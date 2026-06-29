use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub host: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Vec<u8>,
}

impl Request {
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

    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}
