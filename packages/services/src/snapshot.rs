use std::collections::HashMap;

use rack_core::config::Service as ServiceConfig;
use serde::{Deserialize, Serialize};

use crate::registry::{ServiceState, ServiceView};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub proxy_port: Option<u16>,
    pub services: Vec<ServiceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceSnapshot {
    pub id: String,
    pub name: String,
    pub host: String,
    pub run: String,
    pub working_dir: String,
    pub auto_start: bool,
    pub state: StateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StateSnapshot {
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
}

pub fn snapshot_service(
    view: ServiceView,
    configs: &HashMap<String, ServiceConfig>,
) -> ServiceSnapshot {
    let config = configs.get(&view.id);
    ServiceSnapshot {
        id: view.id,
        name: view.name,
        host: view.host,
        run: config.map(|config| config.run.clone()).unwrap_or_default(),
        working_dir: config
            .map(|config| config.working_dir.clone())
            .unwrap_or_else(|| "~".to_string()),
        auto_start: config.map(|config| config.auto_start).unwrap_or_default(),
        state: snapshot_state(view.state),
    }
}

fn snapshot_state(state: ServiceState) -> StateSnapshot {
    match state {
        ServiceState::Stopped => StateSnapshot::Stopped,
        ServiceState::Starting { pid, pgid } => StateSnapshot::Starting { pid, pgid },
        ServiceState::Running { pid, pgid, ports } => StateSnapshot::Running { pid, pgid, ports },
    }
}
