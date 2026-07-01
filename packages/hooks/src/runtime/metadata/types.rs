use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookModuleMetadata {
    pub hooks: Vec<WasmHookEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WasmHookEndpoint {
    Http {
        id: String,
        method: String,
        path: String,
        entry: String,
    },
    Cron {
        id: String,
        schedule: String,
        entry: String,
    },
}

impl<'de> Deserialize<'de> for WasmHookEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        RawEndpoint::deserialize(deserializer).map(Into::into)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawEndpoint {
    Tagged(TaggedEndpoint),
    Legacy {
        id: String,
        method: String,
        path: String,
        entry: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TaggedEndpoint {
    Http {
        id: String,
        method: String,
        path: String,
        entry: String,
    },
    Cron {
        id: String,
        schedule: String,
        entry: String,
    },
}

impl From<RawEndpoint> for WasmHookEndpoint {
    fn from(value: RawEndpoint) -> Self {
        match value {
            RawEndpoint::Tagged(TaggedEndpoint::Http {
                id,
                method,
                path,
                entry,
            })
            | RawEndpoint::Legacy {
                id,
                method,
                path,
                entry,
            } => Self::Http {
                id,
                method,
                path,
                entry,
            },
            RawEndpoint::Tagged(TaggedEndpoint::Cron {
                id,
                schedule,
                entry,
            }) => Self::Cron {
                id,
                schedule,
                entry,
            },
        }
    }
}
