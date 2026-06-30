use anyhow::Result;
use rack_hooks::HookRegistry;

pub fn run() -> Result<()> {
    let summaries = rack_services::hooks::load_deployed(&HookRegistry::default()).summaries;

    if summaries.is_empty() {
        println!("No hooks deployed");
        return Ok(());
    }

    for summary in summaries {
        println!("{}", summary.name);

        for route in &summary.routes {
            println!("  route\t{}\t{}", route.method, route.path);
        }

        for cron in &summary.crons {
            println!("  cron\t{}\t{}", cron.schedule, cron.hook);
        }

        for error in &summary.errors {
            println!("  error\t{error}");
        }
    }

    Ok(())
}
