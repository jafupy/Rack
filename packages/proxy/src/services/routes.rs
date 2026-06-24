use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use super::{Destination, ServiceTarget};

#[derive(Debug, Clone, Default)]
pub struct TargetTable {
    by_origin: HashMap<String, ServiceTarget>,
}

impl TargetTable {
    pub fn new(targets: impl IntoIterator<Item = ServiceTarget>) -> Self {
        let by_origin = targets
            .into_iter()
            .map(|target| (target.host.clone(), target))
            .collect();

        Self { by_origin }
    }

    pub fn resolve(&self, origin: &str) -> Option<&ServiceTarget> {
        self.by_origin.get(origin)
    }

    pub fn is_empty(&self) -> bool {
        self.by_origin.is_empty()
    }
}

#[derive(Clone)]
pub struct ServiceRoutes(Arc<RwLock<TargetTable>>);

impl ServiceRoutes {
    pub fn new(routes: TargetTable) -> Self {
        Self(Arc::new(RwLock::new(routes)))
    }

    pub fn set(&self, origin: impl Into<String>, destination: Destination) {
        let origin = origin.into();
        let Destination::Loopback { port } = destination;
        self.0
            .write()
            .expect("proxy service route lock poisoned")
            .by_origin
            .insert(
                origin.clone(),
                ServiceTarget {
                    service_id: origin.clone(),
                    host: origin,
                    port,
                },
            );
    }

    pub fn remove(&self, origin: &str) {
        self.0
            .write()
            .expect("proxy service route lock poisoned")
            .by_origin
            .remove(origin);
    }

    pub fn update(&self, routes: TargetTable) {
        *self.0.write().expect("proxy service route lock poisoned") = routes;
    }

    pub(crate) fn destination_for(&self, origin: &str) -> Option<Destination> {
        self.0
            .read()
            .expect("proxy service route lock poisoned")
            .resolve(origin)
            .map(ServiceTarget::destination)
    }
}

pub type SharedTargets = ServiceRoutes;
