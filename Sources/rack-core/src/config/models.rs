use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct EnvironmentVariable {
    #[serde(default = "new_id")]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) value: String,
}

impl Default for EnvironmentVariable {
    fn default() -> Self {
        Self {
            id: new_id(),
            key: String::new(),
            value: String::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ServerConfiguration {
    #[serde(default = "new_id")]
    pub(crate) id: String,
    #[serde(default = "default_server_name")]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) arguments: String,
    #[serde(default, rename = "workingDirectory")]
    pub(crate) working_directory: String,
    #[serde(default, rename = "autoStart")]
    pub(crate) auto_start: bool,
    #[serde(default, rename = "customDomain")]
    pub(crate) custom_domain: String,
    #[serde(default)]
    pub(crate) environment: Vec<EnvironmentVariable>,
    #[serde(default)]
    pub(crate) port: Option<u16>,
    #[serde(default, rename = "portFlag")]
    pub(crate) port_flag: Option<String>,
}

impl Default for ServerConfiguration {
    fn default() -> Self {
        Self {
            id: new_id(),
            name: default_server_name(),
            command: String::new(),
            arguments: String::new(),
            working_directory: String::new(),
            auto_start: false,
            custom_domain: String::new(),
            environment: Vec::new(),
            port: None,
            port_flag: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PersistedConfiguration {
    #[serde(default)]
    pub(crate) servers: Vec<ServerConfiguration>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ServerRouteInfo {
    #[serde(rename = "routeSubdomain")]
    pub(crate) route_subdomain: String,
    #[serde(rename = "localURL")]
    pub(crate) local_url: String,
}

pub(crate) fn route_info_command(payload: &Value) -> Result<ServerRouteInfo, String> {
    let server: ServerConfiguration = serde_json::from_value(
        payload
            .get("config")
            .cloned()
            .unwrap_or_else(|| payload.clone()),
    )
    .map_err(|error| error.to_string())?;
    let context = payload.get("context").unwrap_or(&Value::Null);

    Ok(ServerRouteInfo {
        route_subdomain: route_subdomain(&server),
        local_url: local_url(&server, context),
    })
}

pub(crate) fn local_url(server: &ServerConfiguration, context: &Value) -> String {
    let subdomain = route_subdomain(server);
    if context
        .get("standardPortsEnabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return format!("http://{subdomain}.localhost");
    }
    let port = context
        .get("boundPort")
        .and_then(Value::as_u64)
        .unwrap_or(1355);
    format!("http://{subdomain}.localhost:{port}")
}

pub(crate) fn route_subdomain(server: &ServerConfiguration) -> String {
    let raw = if server.custom_domain.is_empty() {
        &server.name
    } else {
        &server.custom_domain
    };
    let trimmed = raw.trim().to_lowercase().replace(' ', "-");
    trimmed
        .strip_suffix(".localhost")
        .unwrap_or(&trimmed)
        .to_string()
}

pub(super) fn new_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id() & 0xffff;
    format!(
        "00000000-0000-4000-8000-{process_id:04x}{:08x}",
        (timestamp as u32).wrapping_add(counter)
    )
}

fn default_server_name() -> String {
    "New Server".to_string()
}
