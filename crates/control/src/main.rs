use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use kwybars_common::config;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    SwitchConfig {
        target: PathBuf,
        active: Option<PathBuf>,
    },
    Help,
}

#[derive(Debug)]
enum ControlError {
    Usage(String),
    Io(std::io::Error),
    InvalidTarget(String),
}

impl Display for ControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::Io(err) => write!(f, "{err}"),
            Self::InvalidTarget(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<std::io::Error> for ControlError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

fn main() {
    match run() {
        Ok(Some(message)) => println!("{message}"),
        Ok(None) => {}
        Err(ControlError::Usage(message)) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
        Err(err) => {
            eprintln!("kwybarsctl: {err}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Option<String>, ControlError> {
    match parse_args(std::env::args_os().skip(1))? {
        Command::Help => Ok(Some(usage())),
        Command::SwitchConfig { target, active } => {
            let active_path = active.unwrap_or_else(config::default_config_path);
            let target_path = validate_target(&target)?;
            let message = switch_config(&active_path, &target_path)?;
            Ok(Some(message))
        }
    }
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Command::Help);
    };

    match command.to_string_lossy().as_ref() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "switch-config" => parse_switch_config(args),
        other => Err(ControlError::Usage(format!(
            "unknown command: {other}\n\n{}",
            usage()
        ))),
    }
}

fn parse_switch_config(args: impl IntoIterator<Item = OsString>) -> Result<Command, ControlError> {
    let mut args = args.into_iter();
    let mut active = None;
    let mut target = None;

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => return Ok(Command::Help),
            "-a" | "--active" => {
                let Some(value) = args.next() else {
                    return Err(ControlError::Usage(format!(
                        "missing value for --active\n\n{}",
                        usage()
                    )));
                };
                active = Some(PathBuf::from(value));
            }
            value if value.starts_with("--active=") => {
                let path = &value["--active=".len()..];
                if path.is_empty() {
                    return Err(ControlError::Usage(format!(
                        "missing value for --active\n\n{}",
                        usage()
                    )));
                }
                active = Some(PathBuf::from(path));
            }
            other => {
                if target.is_some() {
                    return Err(ControlError::Usage(format!(
                        "unexpected extra argument: {other}\n\n{}",
                        usage()
                    )));
                }
                target = Some(PathBuf::from(other));
            }
        }
    }

    let Some(target) = target else {
        return Err(ControlError::Usage(format!(
            "missing target config path\n\n{}",
            usage()
        )));
    };

    Ok(Command::SwitchConfig { target, active })
}

fn validate_target(path: &Path) -> Result<PathBuf, ControlError> {
    let canonical = fs::canonicalize(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ControlError::InvalidTarget(format!("target config does not exist: {}", path.display()))
        } else {
            ControlError::Io(err)
        }
    })?;

    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() {
        return Err(ControlError::InvalidTarget(format!(
            "target config is not a file: {}",
            canonical.display()
        )));
    }

    Ok(canonical)
}

fn switch_config(active_path: &Path, target_path: &Path) -> Result<String, ControlError> {
    if paths_match(active_path, target_path) {
        return Ok(format!(
            "active config already points to {}",
            target_path.display()
        ));
    }

    let Some(parent) = active_path.parent() else {
        return Err(ControlError::InvalidTarget(format!(
            "active config path has no parent directory: {}",
            active_path.display()
        )));
    };
    fs::create_dir_all(parent)?;

    maybe_backup_regular_file(active_path)?;

    let temp_link = parent.join(format!(".kwybarsctl-{}.tmp", std::process::id()));
    if temp_link.exists() {
        let _ = fs::remove_file(&temp_link);
    }

    create_symlink(target_path, &temp_link)?;
    fs::rename(&temp_link, active_path)?;

    Ok(format!(
        "switched active config {} -> {}",
        active_path.display(),
        target_path.display()
    ))
}

fn maybe_backup_regular_file(active_path: &Path) -> Result<(), ControlError> {
    let Ok(metadata) = fs::symlink_metadata(active_path) else {
        return Ok(());
    };

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Ok(());
    }

    let backup_path = active_path.with_file_name(format!(
        "{}.bak",
        active_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config.toml")
    ));
    if backup_path.exists() {
        return Ok(());
    }

    fs::copy(active_path, backup_path)?;
    Ok(())
}

fn paths_match(active_path: &Path, target_path: &Path) -> bool {
    fs::canonicalize(active_path)
        .ok()
        .is_some_and(|current| current == target_path)
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), ControlError> {
    use std::os::unix::fs::symlink;
    symlink(target, link)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<(), ControlError> {
    Err(ControlError::InvalidTarget(
        "kwybarsctl switch-config requires Unix symlink support".to_owned(),
    ))
}

fn usage() -> String {
    "Usage:\n  kwybarsctl switch-config [--active <path>] <target-config.toml>\n  kwybarsctl --help\n\nCommands:\n  switch-config         Atomically switch the watched config path to another config file\n\nOptions:\n  -a, --active <path>   Active config path to update (default: normal Kwybars config path)\n  -h, --help            Show this help message"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Command, parse_args};
    use std::path::PathBuf;

    #[test]
    fn parses_switch_config_command() {
        let parsed = parse_args([
            "switch-config".into(),
            "--active".into(),
            "/tmp/current.toml".into(),
            "/tmp/alt.toml".into(),
        ]);

        let Ok(Command::SwitchConfig { target, active }) = parsed else {
            panic!("expected switch-config command");
        };
        assert_eq!(target, PathBuf::from("/tmp/alt.toml"));
        assert_eq!(active, Some(PathBuf::from("/tmp/current.toml")));
    }

    #[test]
    fn parses_help_command() {
        let parsed = parse_args(["--help".into()]);
        assert!(matches!(parsed, Ok(Command::Help)));
    }
}
