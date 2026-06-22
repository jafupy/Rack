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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_targets_by_service_host() {
        let table = TargetTable::new([target("api", 3000)]);

        assert_eq!(table.resolve("api"), Some(&target("api", 3000)));
        assert_eq!(table.resolve("web"), None);
    }

    #[test]
    fn formats_loopback_target_urls() {
        assert_eq!(target("api", 5173).loopback_url(), "http://127.0.0.1:5173");
    }

    fn target(host: &str, port: u16) -> ServiceTarget {
        ServiceTarget {
            service_id: host.to_string(),
            host: host.to_string(),
            port,
        }
    }
}
