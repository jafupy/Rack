use super::manifest::{FunctionRoute, FunctionRouteMatch};
use super::routing::{route_match_request, route_matches, route_specificity};
use super::runtime::parse_function_response;
use std::path::PathBuf;

#[test]
fn parses_structured_http_function_response() {
    let payload = parse_function_response(
        r#"{"status":201,"headers":{"Content-Type":"application/json"},"body":"{\"ok\":true}"}"#,
    )
    .unwrap();

    assert_eq!(payload["status"], 201);
    assert_eq!(payload["headers"]["content-type"], "application/json");
    assert_eq!(payload["body"], r#"{"ok":true}"#);
}

#[test]
fn matches_recursive_glob_routes() {
    assert!(route_matches("/assets/**/*.js", "/assets/app/main.js").unwrap());
    assert!(!route_matches("/assets/**/*.js", "/assets/app/main.css").unwrap());
    assert!(!route_matches("/assets/*.js", "/assets/app/main.js").unwrap());
}

#[test]
fn exact_routes_are_more_specific_than_globs() {
    assert!(route_specificity("/gcse") > route_specificity("/*"));
    assert!(route_specificity("/assets/images/*") > route_specificity("/assets/**"));
}

#[test]
fn route_match_request_adds_route_metadata() {
    let route_match = FunctionRouteMatch {
        route: FunctionRoute {
            package: "pkg".to_string(),
            id: "assets".to_string(),
            path: "/assets/**".to_string(),
            method: "GET".to_string(),
            function: "serve".to_string(),
            wasm_path: PathBuf::from("functions.wasm"),
        },
        request_path: "/assets/app/main.js".to_string(),
    };
    let request = serde_json::json!({
        "method": "GET",
        "path": "/assets/app/main.js",
        "uri": "/assets/app/main.js?debug=1",
        "headers": {},
        "body": "",
    });

    let request = route_match_request(&route_match, &request);

    assert_eq!(request["route"]["package"], "pkg");
    assert_eq!(request["route"]["id"], "assets");
    assert_eq!(request["route"]["pattern"], "/assets/**");
    assert_eq!(request["route"]["matched_path"], "/assets/app/main.js");
    assert_eq!(request["route"]["is_glob"], true);
}
