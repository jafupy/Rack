mod build;
mod init;
mod install;
mod support;
mod test;
mod types;

use crate::Result;

pub(crate) fn cmd_function(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("add") => {
            let path = args
                .iter()
                .skip(1)
                .find(|arg| !matches!(arg.as_str(), "--replace"))
                .map(String::as_str);
            build::cmd_function_add(path)
        }
        Some("build" | "compile") => build::cmd_function_build(args.get(1).map(String::as_str)),
        Some("init") => init::cmd_function_init(args.get(1).map(String::as_str)),
        Some("test") => test::cmd_function_test(&args[1..]),
        Some("install") => {
            let replace = args.iter().any(|arg| arg == "--replace");
            let link = args.iter().any(|arg| arg == "--link");
            let path = args
                .iter()
                .skip(1)
                .find(|arg| !matches!(arg.as_str(), "--replace" | "--link"))
                .map(String::as_str);
            install::cmd_function_install(path, replace, link)
        }
        Some("ls" | "list") => install::cmd_function_ls(),
        Some("rm" | "remove" | "uninstall") => {
            let name = args.get(1).ok_or("Usage: rack fn rm <name>")?;
            install::cmd_function_remove(name)
        }
        _ => install::cmd_function_install(args.first().map(String::as_str), false, false),
    }
}
