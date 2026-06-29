use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use chrono::Local;
use rack_hooks::{run_cron_wasm_with_event, CronEvent};

use super::schedule::parse_schedule;

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
    let schedules: Vec<_> = crons
        .into_iter()
        .filter_map(|cron| match parse_schedule(&cron.schedule) {
            Ok(schedule) => Some((cron, schedule)),
            Err(error) => {
                eprintln!(
                    "cron hook {}:{} has invalid schedule: {error}",
                    cron.package, cron.id
                );
                None
            }
        })
        .collect();
    let mut next_runs = HashMap::new();

    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        for (cron, schedule) in &schedules {
            let key = format!("{}:{}", cron.package, cron.id);
            let due_at = next_runs
                .entry(key)
                .or_insert_with(|| schedule.next_instant_after(Local::now(), now));
            if *due_at > now {
                continue;
            }

            run_cron(cron);
            *due_at = schedule.next_instant_after(Local::now(), Instant::now());
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn run_cron(cron: &CronHook) {
    let event = CronEvent::new(
        cron.package.clone(),
        cron.id.clone(),
        cron.schedule.clone(),
        Local::now().timestamp(),
    );
    if let Err(error) = run_cron_wasm_with_event(&cron.wasm, &cron.entry, &event) {
        eprintln!("cron hook {}:{} failed: {error}", cron.package, cron.id);
    }
}
