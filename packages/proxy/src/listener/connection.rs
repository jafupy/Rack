use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::{route_host, HostRoute, SharedTargets};

use super::http::{host_header, read_request_head, write_response};

pub(super) async fn handle_client(mut client: TcpStream, targets: SharedTargets) {
    let Ok(request) = read_request_head(&mut client).await else {
        return;
    };
    let Some(host) = host_header(&request) else {
        let _ = write_response(&mut client, 400, "missing Host header").await;
        return;
    };

    let route = match route_host(host) {
        Ok(route) => route,
        Err(error) => {
            let _ = write_response(&mut client, 404, &error.to_string()).await;
            return;
        }
    };

    let HostRoute::Service { host } = route else {
        let _ = write_response(&mut client, 501, "rack.local hooks are not wired yet").await;
        return;
    };

    let Some(target) = targets.resolve(&host) else {
        let _ = write_response(&mut client, 502, "service target is not running").await;
        return;
    };

    let Ok(mut backend) = TcpStream::connect(("127.0.0.1", target.port)).await else {
        let _ = write_response(&mut client, 502, "service target is unavailable").await;
        return;
    };

    if backend.write_all(&request).await.is_err() {
        let _ = write_response(&mut client, 502, "failed to forward request").await;
        return;
    }

    let _ = tokio::io::copy_bidirectional(&mut client, &mut backend).await;
}
