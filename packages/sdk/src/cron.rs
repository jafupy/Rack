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
