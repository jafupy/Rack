use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceTarget {
    pub service_id: String,
    pub host: String,
    pub port: u16,
}

impl ServiceTarget {
    pub fn loopback_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TargetTable {
    by_host: HashMap<String, ServiceTarget>,
}

impl TargetTable {
    pub fn new(targets: impl IntoIterator<Item = ServiceTarget>) -> Self {
        let by_host = targets
            .into_iter()
            .map(|target| (target.host.clone(), target))
            .collect();

        Self { by_host }
    }

    pub fn resolve(&self, host: &str) -> Option<&ServiceTarget> {
        self.by_host.get(host)
    }

    pub fn is_empty(&self) -> bool {
        self.by_host.is_empty()
    }
}
