use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub host: String,
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
        }
    }
}
