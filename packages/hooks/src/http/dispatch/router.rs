use std::fmt;

use super::{HookEndpoint, HookRegistry, HookRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    Conflict { method: String, path: String },
    ReservedPath(String),
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict { method, path } => write!(f, "conflicting hook route: {method} {path}"),
            Self::ReservedPath(path) => write!(f, "reserved hook route: {path}"),
        }
    }
}

impl std::error::Error for RouteError {}

pub fn route(
    registry: &HookRegistry,
    request: &HookRequest,
) -> Result<Option<HookEndpoint>, RouteError> {
    let method = request.method.to_ascii_uppercase();
    let path = normalize_path(&request.path);
    if is_reserved_path(&path) {
        return Err(RouteError::ReservedPath(path));
    }

    let matches: Vec<_> = registry
        .endpoints()
        .into_iter()
        .filter(|endpoint| endpoint.method == method && endpoint.path == path)
        .collect();

    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => Err(RouteError::Conflict { method, path }),
    }
}

pub fn normalize_path(path: &str) -> String {
    let without_query = path.split_once('?').map_or(path, |(path, _)| path);
    if without_query.starts_with('/') {
        without_query.to_string()
    } else {
        format!("/{without_query}")
    }
}

pub fn is_reserved_path(path: &str) -> bool {
    path == "/" || path.starts_with("/_")
}
