use std::sync::{Arc, RwLock};

use crate::runtime::{HookRuntime, RuntimeError};
use crate::{normalize_path, HookRequest, HookResponse};

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
            path: normalize_path(&path.into()),
        }
    }
}

#[derive(Clone)]
pub struct HookRegistry {
    endpoints: Arc<RwLock<Vec<HookEndpoint>>>,
    runtime: Arc<RwLock<HookRuntime>>,
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new([])
    }
}

impl HookRegistry {
    pub fn new(endpoints: impl IntoIterator<Item = HookEndpoint>) -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(endpoints.into_iter().collect())),
            runtime: Arc::new(RwLock::new(HookRuntime::new())),
        }
    }

    pub fn register(&self, endpoint: HookEndpoint) {
        self.endpoints
            .write()
            .expect("hook registry lock poisoned")
            .push(endpoint);
    }

    pub fn register_wasm(&self, wasm: &[u8]) -> Result<Vec<HookEndpoint>, RuntimeError> {
        let endpoints = self
            .runtime
            .write()
            .expect("hook runtime lock poisoned")
            .load_module(wasm)?;
        self.endpoints
            .write()
            .expect("hook registry lock poisoned")
            .extend(endpoints.clone());
        Ok(endpoints)
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

    pub fn run(
        &self,
        endpoint: &HookEndpoint,
        request: &HookRequest,
    ) -> Result<HookResponse, RuntimeError> {
        self.runtime
            .read()
            .expect("hook runtime lock poisoned")
            .run(endpoint, request)
    }
}
