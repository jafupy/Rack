use super::manifest::{load_functions, normalize_route_path, FunctionRoute, FunctionRouteMatch};
use globset::GlobBuilder;

pub(super) fn route_has_glob(path: &str) -> bool {
    path.bytes()
        .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b']' | b'{' | b'}'))
}

pub(super) fn route_specificity(path: &str) -> usize {
    let literal_count = path
        .chars()
        .filter(|character| !matches!(character, '*' | '?' | '[' | ']' | '{' | '}' | ',' | '!'))
        .count();
    if route_has_glob(path) {
        literal_count
    } else {
        literal_count + 10_000
    }
}

pub(super) fn route_matches(route_path: &str, request_path: &str) -> Result<bool, String> {
    if !route_has_glob(route_path) {
        return Ok(route_path == request_path);
    }

    GlobBuilder::new(route_path)
        .literal_separator(true)
        .backslash_escape(true)
        .build()
        .map(|glob| glob.compile_matcher().is_match(request_path))
        .map_err(|error| format!("invalid route glob '{route_path}': {error}"))
}

pub(super) fn validate_route_path(route_path: &str) -> Result<(), String> {
    if route_has_glob(route_path) {
        GlobBuilder::new(route_path)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
            .map(|_| ())
            .map_err(|error| format!("uses invalid glob '{route_path}': {error}"))
    } else {
        Ok(())
    }
}

pub(super) fn find_route(method: &str, path: &str) -> Result<FunctionRouteMatch, String> {
    let normalized = normalize_route_path(path);
    if normalized == "/" || normalized.starts_with("/_") {
        return Err("reserved rack.local path".to_string());
    }

    let mut matched: Option<FunctionRoute> = None;
    let mut matched_score = 0usize;
    for package in load_functions() {
        if !package.errors.is_empty() {
            continue;
        }
        for route in package.routes {
            if route.method != method.to_uppercase() {
                continue;
            }

            if route_matches(&route.path, &normalized)? {
                let score = route_specificity(&route.path);
                if matched.is_some() && score == matched_score {
                    return Err(format!(
                        "route conflict for {} {}",
                        method.to_uppercase(),
                        normalized
                    ));
                }
                if score > matched_score {
                    matched = Some(route);
                    matched_score = score;
                }
            }
        }
    }

    matched
        .map(|route| FunctionRouteMatch {
            route,
            request_path: normalized.clone(),
        })
        .ok_or_else(|| {
            format!(
                "no function route for {} {}",
                method.to_uppercase(),
                normalized
            )
        })
}

pub(super) fn route_match_request(
    route_match: &FunctionRouteMatch,
    request: &serde_json::Value,
) -> serde_json::Value {
    let mut request = request.clone();
    let is_glob = route_has_glob(&route_match.route.path);
    let route = serde_json::json!({
        "package": route_match.route.package,
        "id": route_match.route.id,
        "path": route_match.route.path,
        "pattern": route_match.route.path,
        "method": route_match.route.method,
        "function": route_match.route.function,
        "is_glob": is_glob,
        "matched_path": route_match.request_path,
    });

    if let Some(object) = request.as_object_mut() {
        object.insert("route".to_string(), route);
    }
    request
}
