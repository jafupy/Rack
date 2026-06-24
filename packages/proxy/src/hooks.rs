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
    let request = HookRequest::new(
        session.req_header().method.as_str(),
        session.req_header().uri.path(),
        host,
    );
    let response = rack_hooks::dispatch(registry, &request);
    write_response(session, response).await?;
    Ok(true)
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
