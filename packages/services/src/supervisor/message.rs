use std::sync::mpsc::Sender;

use rack_core::config::Service as ServiceConfig;

use crate::registry::{ServiceState, ServiceView};

use super::SupervisorError;

pub(super) type Reply<T> = Sender<Result<T, SupervisorError>>;

pub(super) enum Message {
    Register {
        config: ServiceConfig,
        reply: Reply<()>,
    },
    List {
        reply: Reply<Vec<ServiceView>>,
    },
    Status {
        id: String,
        reply: Reply<ServiceState>,
    },
    Log {
        id: String,
        reply: Reply<String>,
    },
    Start {
        id: String,
        reply: Reply<()>,
    },
    Stop {
        id: String,
        reply: Reply<()>,
    },
    Restart {
        id: String,
        reply: Reply<()>,
    },
    Shutdown,
}
