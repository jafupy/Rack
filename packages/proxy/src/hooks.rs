use pingora::{http::ResponseHeader, proxy::Session, Result};
use rack_hooks::{HookRegistry, HookRequest, HookResponse};

pub fn is_hooks_host(host: &str) -> bool {
    normalize_host(host).is_some_and(|host| host == "rack.local")
}

pub(crate) async fn dispatch(
    session: &mut Session,
    registry: &HookRegistry,
    host: &str,
) -> Result<bool> {
    let request = build_request(session, host).await?;
    let response = rack_hooks::dispatch(registry, &request);
    write_response(session, response).await?;
    Ok(true)
}

async fn build_request(session: &mut Session, host: &str) -> Result<HookRequest> {
    let header = session.req_header();
    let mut request = HookRequest::new(header.method.as_str(), header.uri.path(), host)
        .query(header.uri.query().unwrap_or_default().to_string());

    for (name, value) in header.headers.iter() {
        request = request.header(name.as_str(), value.to_str().unwrap_or_default());
    }

    let body = if request_has_body(session.req_header()) {
        read_body(session).await?
    } else {
        Vec::new()
    };

    Ok(request.body(body))
}

async fn read_body(session: &mut Session) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let no_body_expected = session.as_downstream_mut().is_body_done();
        let Some(chunk) = session
            .as_downstream_mut()
            .read_body_or_idle(no_body_expected)
            .await?
        else {
            break;
        };
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn request_has_body(header: &pingora::http::RequestHeader) -> bool {
    let content_length = header
        .headers
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default();

    content_length > 0 || header.headers.contains_key("transfer-encoding")
}

async fn write_response(session: &mut Session, response: HookResponse) -> Result<()> {
    let body_len = response.body.len();
    let mut header = ResponseHeader::build(response.status, Some(body_len))?;
    header.insert_header("content-length", body_len.to_string())?;
    for (name, value) in response.headers {
        header.insert_header(name, value)?;
    }
    session
        .write_response_header(Box::new(header), false)
        .await?;
    session
        .write_response_body(Some(response.body.into()), true)
        .await
}

fn normalize_host(host: &str) -> Option<String> {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }

    Some(strip_port(&host).trim_end_matches('.').to_string())
}

fn strip_port(host: &str) -> &str {
    if host.starts_with('[') {
        return host;
    }

    match host.rsplit_once(':') {
        Some((name, port)) if port.parse::<u16>().is_ok() => name,
        _ => host,
    }
}
