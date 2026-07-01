use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use rack_proxy::{ProxyServer, ServiceTarget, TargetTable};

use crate::registry::{ServiceState, ServiceView};

pub(super) fn bind_proxy(runtime: &tokio::runtime::Runtime) -> Result<ProxyServer, String> {
    for port in 1355..=1365 {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        match runtime.block_on(ProxyServer::bind(addr, TargetTable::default())) {
            Ok(proxy) => return Ok(proxy),
            Err(error) => eprintln!("failed to bind proxy at {addr}: {error}"),
        }
    }

    Err("failed to bind proxy on ports 1355 through 1365".to_string())
}

pub(super) fn target_table(services: &[ServiceView]) -> TargetTable {
    TargetTable::new(services.iter().filter_map(service_target))
}

fn service_target(service: &ServiceView) -> Option<ServiceTarget> {
    let ServiceState::Running { ports, .. } = &service.state else {
        return None;
    };

    Some(ServiceTarget {
        service_id: service.id.clone(),
        host: service.host.clone(),
        port: *ports.first()?,
    })
}
