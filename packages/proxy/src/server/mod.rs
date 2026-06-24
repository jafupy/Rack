mod response;
mod service;

use std::net::SocketAddr;

use thiserror::Error;
use tokio::{io, net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::services::{ServiceRoutes, TargetTable};
use service::handle_client;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("failed to bind proxy listener at {addr}: {source}")]
    Bind { addr: SocketAddr, source: io::Error },

    #[error("proxy task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

pub struct ProxyServer {
    addr: SocketAddr,
    services: ServiceRoutes,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl ProxyServer {
    pub async fn bind(addr: SocketAddr, targets: TargetTable) -> Result<Self, ProxyError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|source| ProxyError::Bind { addr, source })?;
        let addr = listener
            .local_addr()
            .map_err(|source| ProxyError::Bind { addr, source })?;
        let services = ServiceRoutes::new(targets);
        let (shutdown, stop) = oneshot::channel();
        let task = tokio::spawn(run(listener, services.clone(), stop));

        Ok(Self {
            addr,
            services,
            shutdown: Some(shutdown),
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

    pub async fn shutdown(mut self) -> Result<(), ProxyError> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.await?;
        Ok(())
    }
}

async fn run(listener: TcpListener, services: ServiceRoutes, mut stop: oneshot::Receiver<()>) {
    loop {
        tokio::select! {
            _ = &mut stop => break,
            accepted = listener.accept() => {
                let Ok((client, _)) = accepted else { continue };
                tokio::spawn(handle_client(client, services.clone()));
            }
        }
    }
}
