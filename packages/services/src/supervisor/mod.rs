mod error;
mod message;
mod runtime;

use std::{
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
};

use rack_core::config::Service as ServiceConfig;

pub use error::SupervisorError;
use message::{Message, Reply};
use runtime::run;

use crate::registry::{Registry, ServiceState, ServiceView};

pub struct Supervisor {
    commands: Sender<Message>,
    thread: Option<JoinHandle<()>>,
}

impl Supervisor {
    pub fn start(registry: Registry) -> Self {
        let (commands, receiver) = mpsc::channel();
        let thread = thread::spawn(move || run(registry, receiver));
        Self {
            commands,
            thread: Some(thread),
        }
    }

    pub fn register(&self, config: ServiceConfig) -> Result<(), SupervisorError> {
        self.request(|reply| Message::Register { config, reply })
    }

    pub fn list(&self) -> Result<Vec<ServiceView>, SupervisorError> {
        self.request(|reply| Message::List { reply })
    }

    pub fn status(&self, id: impl Into<String>) -> Result<ServiceState, SupervisorError> {
        self.request(|reply| Message::Status {
            id: id.into(),
            reply,
        })
    }

    pub fn start_service(&self, id: impl Into<String>) -> Result<(), SupervisorError> {
        self.request(|reply| Message::Start {
            id: id.into(),
            reply,
        })
    }

    pub fn stop_service(&self, id: impl Into<String>) -> Result<(), SupervisorError> {
        self.request(|reply| Message::Stop {
            id: id.into(),
            reply,
        })
    }

    pub fn shutdown(mut self) -> thread::Result<()> {
        self.stop_thread()
    }

    fn request<T>(&self, message: impl FnOnce(Reply<T>) -> Message) -> Result<T, SupervisorError> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(message(reply))
            .map_err(|_| SupervisorError::Stopped)?;
        response.recv().map_err(|_| SupervisorError::Stopped)?
    }

    fn stop_thread(&mut self) -> thread::Result<()> {
        let _ = self.commands.send(Message::Shutdown);

        if let Some(thread) = self.thread.take() {
            thread.join()
        } else {
            Ok(())
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.stop_thread();
    }
}
