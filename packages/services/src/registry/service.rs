use rack_core::config::Service as ServiceConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    Starting {
        pid: i32,
        pgid: i32,
    },
    Running {
        pid: i32,
        pgid: i32,
        ports: Vec<u16>,
    },
    Failed {
        pid: i32,
        pgid: i32,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceView {
    pub id: String,
    pub name: String,
    pub host: String,
    pub state: ServiceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub state: ServiceState,
    pub config: ServiceConfig,
}

impl Service {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            state: ServiceState::Stopped,
            config,
        }
    }

    pub fn view(&self) -> ServiceView {
        ServiceView {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            host: self.config.host.clone(),
            state: self.state.clone(),
        }
    }
}
