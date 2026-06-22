use serde::{Deserialize, Serialize};

use crate::snapshot::Snapshot;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    List,
    Start,
    Stop,
    Restart,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
    pub command: Command,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    pub ok: bool,
    pub snapshot: Option<Snapshot>,
    pub log: Option<String>,
    pub error: Option<String>,
}

impl Response {
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            snapshot: None,
            log: None,
            error: Some(error.into()),
        }
    }
}
