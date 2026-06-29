use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cron {
    pub schedule: String,
}

impl Cron {
    pub fn new(schedule: impl Into<String>) -> Self {
        Self {
            schedule: schedule.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronEvent {
    pub package: String,
    pub hook: String,
    pub schedule: String,
    pub scheduled_at_unix: i64,
}
