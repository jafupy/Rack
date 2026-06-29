use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronEvent {
    pub package: String,
    pub hook: String,
    pub schedule: String,
    pub scheduled_at_unix: i64,
}

impl CronEvent {
    pub fn new(
        package: impl Into<String>,
        hook: impl Into<String>,
        schedule: impl Into<String>,
        scheduled_at_unix: i64,
    ) -> Self {
        Self {
            package: package.into(),
            hook: hook.into(),
            schedule: schedule.into(),
            scheduled_at_unix,
        }
    }
}
