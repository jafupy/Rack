use super::manifest::load_functions;
use super::runtime::run_cron;
use crate::schedule::{next_after, parse_schedule};
use chrono::{DateTime, Local};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

pub(crate) fn start_scheduler(
    stop: Arc<AtomicBool>,
    callback: Option<crate::EventCallback>,
    context: usize,
) {
    std::thread::spawn(move || {
        let mut next_runs: BTreeMap<String, DateTime<Local>> = BTreeMap::new();
        let mut reported_invalid_schedules: BTreeSet<String> = BTreeSet::new();

        while !stop.load(Ordering::Relaxed) {
            let now = Local::now();
            for package in load_functions() {
                if !package.errors.is_empty() {
                    continue;
                }

                for cron in package.crons {
                    let key = format!("{}:{}", cron.package, cron.id);
                    let schedule = match parse_schedule(&cron.schedule) {
                        Ok(schedule) => {
                            reported_invalid_schedules.remove(&key);
                            schedule
                        }
                        Err(message) => {
                            if reported_invalid_schedules.insert(key.clone()) {
                                if let Some(callback) = callback {
                                    crate::emit(
                                        callback,
                                        context,
                                        &serde_json::json!({
                                            "type": "cron.error",
                                            "payload": {
                                                "package": cron.package,
                                                "id": cron.id,
                                                "schedule": cron.schedule,
                                                "error": message,
                                            }
                                        })
                                        .to_string(),
                                    );
                                }
                            }
                            continue;
                        }
                    };

                    let due_at = *next_runs
                        .entry(key.clone())
                        .or_insert_with(|| next_after(&schedule, now).unwrap_or(now));
                    if due_at > now {
                        continue;
                    }

                    if let Some(callback) = callback {
                        crate::emit(
                            callback,
                            context,
                            &serde_json::json!({
                                "type": "cron.started",
                                "payload": {
                                    "package": cron.package,
                                    "id": cron.id,
                                    "schedule": cron.schedule,
                                    "scheduled_at": due_at.to_rfc3339(),
                                }
                            })
                            .to_string(),
                        );
                    }

                    let result = run_cron(&cron, due_at);

                    if let Some(callback) = callback {
                        crate::emit(
                            callback,
                            context,
                            &serde_json::json!({
                                "type": "cron.finished",
                                "payload": {
                                    "package": cron.package,
                                    "id": cron.id,
                                    "schedule": cron.schedule,
                                    "scheduled_at": due_at.to_rfc3339(),
                                    "result": result,
                                }
                            })
                            .to_string(),
                        );
                    }

                    if let Some(next) = next_after(&schedule, now) {
                        next_runs.insert(key, next);
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }
    });
}
