use super::RackRuntime;
use crate::hooks::{self, HookScheduler};

impl RackRuntime {
    pub fn hooks_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.hooks).map_err(|error| error.to_string())
    }

    pub fn hook_path(name: &str) -> Result<String, String> {
        Ok(hooks::deployed_hook_path(name)?
            .to_string_lossy()
            .into_owned())
    }

    pub fn reload_hooks(&mut self) -> Result<(), String> {
        let Some(proxy) = &self.proxy else {
            return Err("proxy is not running".to_string());
        };
        proxy.hooks().clear();
        self.hook_scheduler.take();
        let deployed_hooks = hooks::load_deployed(&proxy.hooks());
        self.hook_scheduler = Some(HookScheduler::start(deployed_hooks.crons));
        self.hooks = deployed_hooks.summaries;
        Ok(())
    }

    pub fn remove_hook(&mut self, name: &str) -> Result<(), String> {
        hooks::remove_deployed(name)?;
        self.reload_hooks()
    }
}
