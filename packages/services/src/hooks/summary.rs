use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookSummary {
    pub name: String,
    pub routes: Vec<HookRouteSummary>,
    pub crons: Vec<HookCronSummary>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookRouteSummary {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookCronSummary {
    pub schedule: String,
    pub hook: String,
}
