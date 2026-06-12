pub(in crate::functions) fn parse_function_response(stdout: &str) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let status = value
        .get("status")
        .and_then(|value| value.as_u64())
        .filter(|status| (100..=599).contains(status))
        .unwrap_or(200);
    let headers = value
        .get("headers")
        .and_then(|headers| headers.as_object())
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| {
                        (
                            key.to_ascii_lowercase(),
                            serde_json::Value::String(value.to_string()),
                        )
                    })
                })
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_else(|| {
            let mut headers = serde_json::Map::new();
            headers.insert(
                "content-type".to_string(),
                serde_json::Value::String("text/plain".to_string()),
            );
            headers
        });
    let body = value
        .get("body")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    Some(serde_json::json!({
        "status": status,
        "headers": headers,
        "body": body,
    }))
}
