mod destination;
mod routes;

use tokio::{io::AsyncWriteExt, net::TcpStream};

pub use destination::{Destination, ServiceTarget};
pub use routes::{ServiceRoutes, SharedTargets, TargetTable};

const LOCALHOST_SUFFIX: &str = ".localhost";

pub fn origin_from_host(host: &str) -> Option<String> {
    let host = normalize_host(host)?;
    let origin = host.strip_suffix(LOCALHOST_SUFFIX)?;
    (!origin.is_empty() && !origin.contains('.')).then(|| origin.to_string())
}

pub(crate) async fn forward(
    client: &mut TcpStream,
    routes: &ServiceRoutes,
    origin: &str,
    request: &[u8],
) -> Result<(), ForwardError> {
    let destination = routes
        .destination_for(origin)
        .ok_or(ForwardError::MissingDestination)?;
    let mut backend = TcpStream::connect(("127.0.0.1", destination.port()))
        .await
        .map_err(|_| ForwardError::Unavailable)?;

    backend
        .write_all(request)
        .await
        .map_err(|_| ForwardError::WriteFailed)?;
    let _ = tokio::io::copy_bidirectional(client, &mut backend).await;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ForwardError {
    MissingDestination,
    Unavailable,
    WriteFailed,
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
