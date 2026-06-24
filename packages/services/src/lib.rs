pub mod control;
pub mod ffi;
pub mod hooks;
pub mod process;
pub mod registry;
mod runtime;
pub mod snapshot;
pub mod supervisor;

use std::sync::Mutex;

use runtime::RackRuntime;

pub(crate) static RUNTIME: Mutex<Option<RackRuntime>> = Mutex::new(None);
