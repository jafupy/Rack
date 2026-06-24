use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookEndpoint {
    pub id: String,
    pub method: String,
    pub path: String,
}

impl HookEndpoint {
    pub fn new(id: impl Into<String>, method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            method: method.into().to_ascii_uppercase(),
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HookRegistry {
    endpoints: Arc<RwLock<Vec<HookEndpoint>>>,
}

impl HookRegistry {
    pub fn new(endpoints: impl IntoIterator<Item = HookEndpoint>) -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(endpoints.into_iter().collect())),
        }
    }

    pub fn register(&self, endpoint: HookEndpoint) {
        self.endpoints
            .write()
            .expect("hook registry lock poisoned")
            .push(endpoint);
    }

    pub fn remove(&self, id: &str) {
        self.endpoints
            .write()
            .expect("hook registry lock poisoned")
            .retain(|endpoint| endpoint.id != id);
    }

    pub fn endpoints(&self) -> Vec<HookEndpoint> {
        self.endpoints
            .read()
            .expect("hook registry lock poisoned")
            .clone()
    }
}
