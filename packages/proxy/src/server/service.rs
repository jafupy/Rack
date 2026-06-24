use tokio::net::TcpStream;

use crate::{
    hooks,
    services::{self, ForwardError, ServiceRoutes},
};

use super::response::{host_header, read_request_head, write_response};

pub(super) async fn handle_client(mut client: TcpStream, routes: ServiceRoutes) {
    let Ok(request) = read_request_head(&mut client).await else {
        return;
    };
    let Some(host) = host_header(&request) else {
        let _ = write_response(&mut client, 400, "missing Host header").await;
        return;
    };

    if hooks::is_hooks_host(host) {
        let _ = write_response(&mut client, 501, "rack.local hooks are not wired yet").await;
        return;
    }

    let Some(origin) = services::origin_from_host(host) else {
        let _ = write_response(
            &mut client,
            404,
            &format!("unsupported Host header `{host}`"),
        )
        .await;
        return;
    };

    match services::forward(&mut client, &routes, &origin, &request).await {
        Ok(()) => {}
        Err(ForwardError::MissingDestination) => {
            let _ = write_response(&mut client, 502, "service destination is not running").await;
        }
        Err(ForwardError::Unavailable) => {
            let _ = write_response(&mut client, 502, "service destination is unavailable").await;
        }
        Err(ForwardError::WriteFailed) => {
            let _ = write_response(&mut client, 502, "failed to forward request").await;
        }
    }
}
