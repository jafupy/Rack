use std::sync::{Arc, RwLock};

use crate::{ServiceTarget, TargetTable};

#[derive(Clone)]
pub struct SharedTargets(Arc<RwLock<TargetTable>>);

impl SharedTargets {
    pub fn new(targets: TargetTable) -> Self {
        Self(Arc::new(RwLock::new(targets)))
    }

    pub fn update(&self, targets: TargetTable) {
        *self.0.write().expect("proxy target lock poisoned") = targets;
    }

    pub(crate) fn resolve(&self, host: &str) -> Option<ServiceTarget> {
        self.0
            .read()
            .expect("proxy target lock poisoned")
            .resolve(host)
            .cloned()
    }
}
