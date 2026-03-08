use std::path::PathBuf;
use std::time::Duration;

use kwybars_common::cli;
use kwybars_common::config::DaemonConfig;
use kwybars_common::notify::notify_error_with_cooldown;

fn main() {
    let config_path = match resolve_cli_config_path() {
        Ok(path) => path,
        Err(exit_code) => std::process::exit(exit_code),
    };

    if let Err(err) = kwybars_common::logging::init_logging("daemon") {
        eprintln!("kwybars-daemon logging init failed: {err}");
    }

    if let Err(err) = kwybars_daemon::run(config_path) {
        tracing::error!("kwybars-daemon failed: {err}");
        let defaults = DaemonConfig::default();
        notify_error_with_cooldown(
            "daemon.fatal",
            "Kwybars Daemon Error",
            &format!("{err}"),
            defaults.notify_on_error,
            Duration::from_secs(defaults.notify_cooldown_seconds),
        );
        std::process::exit(1);
    }
}

fn resolve_cli_config_path() -> Result<PathBuf, i32> {
    let options = match cli::parse_standard_cli() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("kwybars-daemon: {}", err.message());
            eprintln!("{}", cli::usage("kwybars-daemon"));
            return Err(2);
        }
    };

    if options.show_help {
        println!("{}", cli::usage("kwybars-daemon"));
        return Err(0);
    }

    Ok(options
        .config_path
        .unwrap_or_else(kwybars_common::config::default_config_path))
}
