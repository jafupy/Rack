mod service;

use std::{net::SocketAddr, thread::JoinHandle};

use pingora::{
    proxy::http_proxy_service,
    server::{configuration::ServerConf, RunArgs, Server, ShutdownSignal, ShutdownSignalWatch},
};
use rack_hooks::HookRegistry;
use thiserror::Error;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
    time::{sleep, Duration, Instant},
};

use crate::services::{ServiceRoutes, TargetTable};
use service::RackProxy;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("failed to bind proxy listener at {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        source: std::io::Error,
    },

    #[error("failed to start pingora proxy: {0}")]
    Start(String),

    #[error("proxy thread failed")]
    Task,
}

pub struct ProxyServer {
    addr: SocketAddr,
    services: ServiceRoutes,
    hooks: HookRegistry,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl ProxyServer {
    pub async fn bind(addr: SocketAddr, targets: TargetTable) -> Result<Self, ProxyError> {
        let addr = reserve_addr(addr).await?;
        let services = ServiceRoutes::new(targets);
        let hooks = HookRegistry::default();
        let (shutdown, stop) = watch::channel(false);
        let task = run_pingora(addr, services.clone(), hooks.clone(), stop)?;
        wait_until_ready(addr).await?;

        Ok(Self {
            addr,
            services,
            hooks,
            shutdown,
            task,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn services(&self) -> ServiceRoutes {
        self.services.clone()
    }

    pub fn targets(&self) -> ServiceRoutes {
        self.services()
    }

    pub fn hooks(&self) -> HookRegistry {
        self.hooks.clone()
    }

    pub async fn shutdown(self) -> Result<(), ProxyError> {
        let _ = self.shutdown.send(true);
        tokio::task::spawn_blocking(move || self.task.join())
            .await
            .map_err(|_| ProxyError::Task)?
            .map_err(|_| ProxyError::Task)
    }
}

async fn wait_until_ready(addr: SocketAddr) -> Result<(), ProxyError> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(10)).await;
    }

    Err(ProxyError::Start(format!("proxy did not listen at {addr}")))
}

async fn reserve_addr(addr: SocketAddr) -> Result<SocketAddr, ProxyError> {
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| ProxyError::Bind { addr, source })?;
    let bound = listener
        .local_addr()
        .map_err(|source| ProxyError::Bind { addr, source })?;
    drop(listener);
    Ok(bound)
}

fn run_pingora(
    addr: SocketAddr,
    services: ServiceRoutes,
    hooks: HookRegistry,
    stop: watch::Receiver<bool>,
) -> Result<JoinHandle<()>, ProxyError> {
    let proxy_port = addr.port();
    let addr = addr.to_string();
    let handle = std::thread::Builder::new()
        .name("rack-proxy".to_string())
        .spawn(move || {
            let mut server = Server::new_with_opt_and_conf(None, ServerConf::new().unwrap());
            server.bootstrap();

            let mut proxy = http_proxy_service(
                &server.configuration,
                RackProxy::new(services, hooks, proxy_port),
            );
            proxy.add_tcp(&addr);
            server.add_service(proxy);
            server.run(RunArgs {
                shutdown_signal: Box::new(ProxyShutdown { stop }),
            });
        })
        .map_err(|error| ProxyError::Start(error.to_string()))?;
    Ok(handle)
}

struct ProxyShutdown {
    stop: watch::Receiver<bool>,
}

#[async_trait::async_trait]
impl ShutdownSignalWatch for ProxyShutdown {
    async fn recv(&self) -> ShutdownSignal {
        let mut stop = self.stop.clone();
        let _ = stop.changed().await;
        ShutdownSignal::FastShutdown
    }
}
