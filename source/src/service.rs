use crate::{config, Result};
use std::{env, fs, path::{Path, PathBuf}, process::Command, thread, time::Duration};

pub const LABEL: &str = "local.monitor-keys.agent";

pub struct Paths {
    pub data: PathBuf,
    pub binary: PathBuf,
    pub config: PathBuf,
    pub plist: PathBuf,
    pub log: PathBuf,
    pub ready: PathBuf,
}
impl Paths {
    pub fn new() -> Result<Self> {
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME is unavailable")?);
        let data = home.join("Library/Application Support/Monitor Keys");
        Ok(Self {
            binary: data.join("monitor-keys"), config: data.join("config.toml"), ready: data.join("ready.txt"),
            plist: home.join(format!("Library/LaunchAgents/{LABEL}.plist")),
            log: home.join("Library/Logs/Monitor Keys/monitor-keys.log"), data,
        })
    }
}
fn domain() -> String { format!("gui/{}", unsafe { libc::getuid() }) }
fn target() -> String { format!("{}/{LABEL}", domain()) }

pub fn xml(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}
fn agent_plist(paths: &Paths) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>{LABEL}</string>
<key>ProgramArguments</key><array><string>{}</string><string>run</string></array>
<key>RunAtLoad</key><true/>
<key>ProcessType</key><string>Interactive</string>
<key>StandardOutPath</key><string>{}</string>
<key>StandardErrorPath</key><string>{}</string>
</dict></plist>
"#, xml(&paths.binary.to_string_lossy()), xml(&paths.log.to_string_lossy()), xml(&paths.log.to_string_lossy()))
}

fn launch(args: &[&str]) -> Result<()> {
    let output = Command::new("/bin/launchctl").args(args).output()?;
    if !output.status.success() {
        return Err(format!("launchctl {} failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr).trim()).into());
    }
    Ok(())
}

fn agent_state() -> Result<Option<String>> {
    let result = Command::new("/bin/launchctl").args(["print", &target()]).output()?;
    Ok(result.status.success().then(|| String::from_utf8_lossy(&result.stdout).into_owned()))
}

pub fn stop(paths: &Paths) -> Result<()> {
    if agent_state()?.is_some() {
        if let Err(error) = launch(&["bootout", &target()]) {
            // launchd can remove the job yet report ESRCH when its process has
            // already exited. Only ignore the error if the job is now absent.
            if agent_state()?.is_some() { return Err(error); }
        }
    }
    if paths.ready.exists() { fs::remove_file(&paths.ready)?; }
    Ok(())
}

pub fn status(paths: &Paths) -> Result<bool> {
    let ready = fs::read_to_string(&paths.ready).unwrap_or_default();
    let running = is_running(paths)?;
    println!("{}", if running { "Running — global shortcuts are registered." } else { "Stopped or failed to register shortcuts." });
    if running { for line in ready.lines().skip(1) { println!("{line}"); } }
    println!("Config: {}\nLog: {}", paths.config.display(), paths.log.display());
    Ok(running)
}

pub fn is_running(paths: &Paths) -> Result<bool> {
    let state = agent_state()?.unwrap_or_default();
    let ready = fs::read_to_string(&paths.ready).unwrap_or_default();
    let pid = ready.lines().next().unwrap_or("");
    Ok(!pid.is_empty() && state.lines().any(|line| line.trim() == format!("pid = {pid}")))
}

pub fn start(paths: &Paths) -> Result<()> {
    config::Config::load(&paths.config)?;
    if !paths.binary.exists() || !paths.plist.exists() { return Err("Not installed. Run `monitor-keys install`.".into()); }
    stop(paths)?;
    fs::create_dir_all(paths.log.parent().unwrap())?;
    if fs::metadata(&paths.log).is_ok_and(|m| m.len() > 1_048_576) {
        fs::rename(&paths.log, paths.log.with_extension("log.previous"))?;
    }
    launch(&["enable", &target()])?;
    launch(&["bootstrap", &domain(), paths.plist.to_str().ok_or("Invalid launch agent path")?])?;
    for _ in 0..60 {
        if paths.ready.exists() { return if status(paths)? { Ok(()) } else { Err("Startup failed. See `monitor-keys logs`.".into()) }; }
        thread::sleep(Duration::from_millis(50));
    }
    Err(format!("The service did not become ready. Run `monitor-keys logs` to see any shortcut conflict. Log: {}", paths.log.display()).into())
}

pub fn install(paths: &Paths) -> Result<()> {
    if unsafe { libc::getuid() } == 0 { return Err("Install as your normal user, without sudo.".into()); }
    config::init(&paths.config)?;
    let original = fs::read_to_string(&paths.config)?;
    let updated = config::edited_text(&original, &config::Config::load(&paths.config)?)?;
    stop(paths)?;
    if updated != original {
        let backup = paths.data.join("config.before-upgrade.toml");
        if !backup.exists() { fs::write(backup, &original)?; }
        crate::files::write_atomic(&paths.config, &updated)?;
    }
    fs::create_dir_all(paths.plist.parent().unwrap())?;
    let source = env::current_exe()?;
    if source != paths.binary {
        let next = paths.data.join("monitor-keys.next");
        fs::copy(source, &next)?;
        fs::rename(next, &paths.binary)?;
    }
    fs::write(&paths.plist, agent_plist(paths))?;
    start(paths)?;
    if let Err(error) = crate::shell::install(paths) { eprintln!("Installed listener, but permanent mk setup needs attention: {error}"); }
    println!("Installed. Login startup is enabled.\nControl command: \"{}\"", paths.binary.display());
    Ok(())
}

pub fn uninstall(paths: &Paths) -> Result<()> {
    stop(paths)?;
    if let Err(error) = crate::shell::uninstall(paths) { eprintln!("Shell cleanup: {error}"); }
    for path in [&paths.plist, &paths.binary] {
        if path.exists() { fs::remove_file(path)?; }
    }
    println!("Uninstalled and disabled login startup. Configuration and logs kept at:\n{}\n{}", paths.config.display(), paths.log.display());
    Ok(())
}

pub fn logs(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let lines: Vec<_> = text.lines().collect();
    for line in &lines[lines.len().saturating_sub(80)..] { println!("{line}"); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_launch_agent_paths() {
        assert_eq!(xml("/Users/A&B/<work>\"'"), "/Users/A&amp;B/&lt;work&gt;&quot;&apos;");
    }
}
