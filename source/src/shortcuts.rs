use crate::{config::{self, Config, Input}, files::write_atomic, service::{self, Paths}, Result};
use clap::Subcommand;
use std::{fs, os::fd::AsRawFd, path::Path};

#[derive(Subcommand)]
pub enum Command {
    /// Show configured F-key shortcuts
    Show,
    /// Change one input's key; use set-all when swapping existing keys
    Set {
        #[arg(value_enum, ignore_case = true)] input: Input,
        /// One plain F-key, such as F4
        key: String,
    },
    /// Set all three keys together (USB-C, HDMI, DisplayPort order)
    SetAll { usb_c: String, hdmi: String, displayport: String },
    /// Restore F1 = USB-C, F2 = HDMI, F3 = DisplayPort
    Reset,
}

pub fn execute(command: Option<Command>, path: &Path, paths: &Paths, installed: bool) -> Result<()> {
    if matches!(command, None | Some(Command::Show)) {
        Config::load(path)?.show_shortcuts();
        return Ok(());
    }
    let lock = fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path.with_extension("edit.lock"))?;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("Another shortcut change is in progress. Try again shortly.".into());
    }
    let original = fs::read_to_string(path)?;
    let mut config = Config::load(path)?;
    match command.unwrap() {
        Command::Set { input, key } => config.set_key(input, &key)?,
        Command::SetAll { usb_c, hdmi, displayport } => {
            config.set_key(Input::Usbc1, &usb_c)?;
            config.set_key(Input::Hdmi1, &hdmi)?;
            config.set_key(Input::Dp1, &displayport)?;
        }
        Command::Reset => {
            for (input, key) in Input::ALL.into_iter().zip(["F1", "F2", "F3"]) { config.set_key(input, key)?; }
        }
        Command::Show => unreachable!(),
    }
    let updated = config::edited_text(&original, &config)?;
    let running = installed && service::is_running(paths)?;
    apply_transaction(path, &original, &updated, running, || service::start(paths))?;
    println!("Shortcuts saved{}.", if running { " and applied" } else { " (listener is stopped; start it to apply)" });
    config.show_shortcuts();
    Ok(())
}

fn apply_transaction(path: &Path, original: &str, updated: &str, running: bool, mut restart: impl FnMut() -> Result<()>) -> Result<()> {
    write_atomic(path, updated)?;
    if running {
        if let Err(error) = restart() {
            write_atomic(path, original)?;
            let recovery = match restart() {
                Ok(()) => "Previous configuration restored and listener restarted.".to_owned(),
                Err(recovery_error) => format!("Previous configuration restored, but restarting failed: {recovery_error}. Run `mk logs`."),
            };
            return Err(format!("Could not apply shortcuts: {error}\n{recovery}").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_registration_restores_exact_config_and_restarts() {
        let dir = std::env::temp_dir().join(format!("monitor-keys-rollback-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("config.toml");
        fs::write(&file, "original with comments").unwrap();
        let mut calls = 0;
        let result = apply_transaction(&file, "original with comments", "new config", true, || {
            calls += 1;
            if calls == 1 { Err("key conflict".into()) } else { Ok(()) }
        });
        assert!(result.unwrap_err().to_string().contains("Previous configuration restored"));
        assert_eq!(calls, 2);
        assert_eq!(fs::read_to_string(&file).unwrap(), "original with comments");
        fs::remove_dir_all(dir).unwrap();
    }
}
