use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use rack_hooks::run_cron_wasm;

#[derive(Clone)]
pub struct CronHook {
    pub package: String,
    pub id: String,
    pub schedule: String,
    pub entry: String,
    pub wasm: Arc<Vec<u8>>,
}

pub struct HookScheduler {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl HookScheduler {
    pub fn start(crons: Vec<CronHook>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = thread::spawn(move || run(crons, thread_stop));
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for HookScheduler {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(crons: Vec<CronHook>, stop: Arc<AtomicBool>) {
    let mut next_runs = HashMap::new();

    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        for cron in &crons {
            let Ok(interval) = parse_interval(&cron.schedule) else {
                continue;
            };
            let key = format!("{}:{}", cron.package, cron.id);
            let due_at = next_runs.entry(key).or_insert_with(|| now + interval);
            if *due_at > now {
                continue;
            }

            if let Err(error) = run_cron_wasm(&cron.wasm, &cron.entry) {
                eprintln!("cron hook {}:{} failed: {error}", cron.package, cron.id);
            }
            *due_at = now + interval;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn parse_interval(schedule: &str) -> Result<Duration, String> {
    let normalized = schedule.trim().to_lowercase();
    let rest = normalized
        .strip_prefix("every ")
        .ok_or_else(|| "expected schedule to start with `every`".to_string())?;
    let mut parts = rest.split_whitespace();
    let first = parts.next().ok_or_else(|| "missing interval".to_string())?;

    let (amount, unit) = match first {
        "second" | "seconds" | "minute" | "minutes" | "hour" | "hours" | "day" | "days" => {
            (1.0, first)
        }
        value => {
            let amount = value
                .parse::<f64>()
                .map_err(|_| "invalid interval amount".to_string())?;
            let unit = parts
                .next()
                .ok_or_else(|| "missing interval unit".to_string())?;
            (amount, unit)
        }
    };

    if amount <= 0.0 {
        return Err("interval must be positive".to_string());
    }

    let seconds = match unit.trim_end_matches('s') {
        "second" => amount,
        "minute" => amount * 60.0,
        "hour" => amount * 60.0 * 60.0,
        "day" => amount * 60.0 * 60.0 * 24.0,
        _ => return Err(format!("unsupported interval unit `{unit}`")),
    };

    Ok(Duration::from_secs(seconds.round().max(1.0) as u64))
}
