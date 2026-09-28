mod config;
mod display;
mod files;
mod listener;
mod service;
mod shell;
mod shortcuts;

use clap::{Parser, Subcommand};
use config::{Config, Input};
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
#[command(version, about = "Global F-key monitor input switching. Apple Silicon, no GUI.")]
struct Cli {
    /// Alternative config file (diagnostics/foreground use only)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Install for this user and start automatically at login
    Install,
    /// Stop and remove the installed utility, keeping configuration and logs
    Uninstall,
    /// Start the installed background service
    Start,
    /// Stop until explicitly started or next login
    Stop,
    /// Reload configuration by restarting the service
    Restart,
    /// Report service status and registered bindings
    Status,
    /// Show the last 80 log lines
    Logs,
    /// Create the default configuration if missing
    Init,
    /// Print the configuration file location
    ConfigPath,
    /// Validate configuration and briefly register/release global F-keys
    Check,
    /// Print detected DDC-capable monitors
    Displays,
    /// Read the current input (some monitors cannot read while inactive)
    Current,
    /// Switch video input immediately
    Switch { #[arg(value_enum, ignore_case = true)] input: Input },
    /// View or change F-key shortcuts; changes apply automatically when running
    #[command(alias = "shortcut")]
    Shortcuts { #[command(subcommand)] command: Option<shortcuts::Command> },
    /// Run invisibly in the foreground (normally managed by login startup)
    Run,
    /// Test global key delivery without switching the monitor (stop service first)
    Listen { #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u64).range(1..=300))] seconds: u64 },
    #[command(hide = true)]
    SendInput { #[arg(value_enum, ignore_case = true)] input: Input },
}

fn execute() -> Result<()> {
    let cli = Cli::parse();
    let paths = service::Paths::new()?;
    if cli.config.is_some() && matches!(cli.command, Action::Install | Action::Uninstall | Action::Start | Action::Stop | Action::Restart | Action::Status | Action::Logs) {
        return Err("Service commands use the installed configuration; --config is for foreground commands only.".into());
    }
    let installed = cli.config.is_none();
    let path = cli.config.unwrap_or_else(|| paths.config.clone());
    match cli.command {
        Action::Install => service::install(&paths),
        Action::Uninstall => service::uninstall(&paths),
        Action::Start | Action::Restart => service::start(&paths),
        Action::Stop => { service::stop(&paths)?; println!("Stopped. Will start again at next login."); Ok(()) },
        Action::Status => { if service::status(&paths)? { Ok(()) } else { Err("Service is not running.".into()) } },
        Action::Logs => service::logs(&paths.log),
        Action::Init => { config::init(&path)?; println!("{}", path.display()); Ok(()) },
        Action::ConfigPath => { println!("{}", path.display()); Ok(()) },
        Action::Displays => display::list(),
        Action::Shortcuts { command } => shortcuts::execute(command, &path, &paths, installed),
        Action::Current => display::current(&Config::load(&path)?),
        Action::Switch { input } => { Config::load(&path)?; listener::bounded_switch(&path, input) },
        Action::SendInput { input } => display::switch(&Config::load(&path)?, input),
        Action::Run => listener::run(&path, Config::load(&path)?, &paths, false, None),
        Action::Check => listener::run(&path, Config::load(&path)?, &paths, true, None),
        Action::Listen { seconds } => listener::run(&path, Config::load(&path)?, &paths, false, Some(seconds)),
    }
}

fn main() {
    if let Err(error) = execute() { eprintln!("Monitor Keys: {error}"); std::process::exit(1); }
}
