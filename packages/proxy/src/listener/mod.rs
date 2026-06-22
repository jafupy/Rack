mod connection;
mod http;

use std::net::SocketAddr;

use thiserror::Error;
use tokio::{io, net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::{SharedTargets, TargetTable};
use connection::handle_client;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("failed to bind proxy listener at {addr}: {source}")]
    Bind { addr: SocketAddr, source: io::Error },

    #[error("proxy task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

pub struct ProxyServer {
    addr: SocketAddr,
    targets: SharedTargets,
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
        let targets = SharedTargets::new(targets);
        let (shutdown, stop) = oneshot::channel();
        let task = tokio::spawn(run(listener, targets.clone(), stop));

        Ok(Self {
            addr,
            targets,
            shutdown: Some(shutdown),
            task,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn targets(&self) -> SharedTargets {
        self.targets.clone()
    }

    pub async fn shutdown(mut self) -> Result<(), ProxyError> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.await?;
        Ok(())
    }
}

async fn run(listener: TcpListener, targets: SharedTargets, mut stop: oneshot::Receiver<()>) {
    loop {
        tokio::select! {
            _ = &mut stop => break,
            accepted = listener.accept() => {
                let Ok((client, _)) = accepted else { continue };
                tokio::spawn(handle_client(client, targets.clone()));
            }
        }
    }
}
