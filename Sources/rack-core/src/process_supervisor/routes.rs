use crate::process::LaunchPlan;
use crate::routes::{register_pending_route, unregister_route};

pub(super) fn prepare_route(plan: &LaunchPlan) -> Result<(), String> {
    let _ = std::fs::remove_file(&plan.socket_path);
    if let Some(parent) = std::path::Path::new(&plan.socket_path).parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    register_pending_route(&plan.subdomain, &plan.working_directory)
}

pub(super) fn unregister_process_route(plan: &LaunchPlan) {
    let _ = unregister_route(&plan.subdomain);
    let _ = std::fs::remove_file(&plan.socket_path);
}
