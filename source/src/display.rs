use crate::{config::{Config, Input}, Result};
use ddc::Ddc;
use ddc_macos::Monitor;

pub fn list() -> Result<()> {
    let monitors = Monitor::enumerate()?;
    if monitors.is_empty() { return Err("No DDC-capable external display found. Check the cable and the monitor's DDC/CI setting.".into()); }
    for m in monitors {
        println!("{} | serial: {}", m.description(), m.serial_number().unwrap_or_else(|| "unavailable".into()));
    }
    Ok(())
}

fn select(config: &Config) -> Result<Monitor> {
    let monitors = Monitor::enumerate()?;
    let names: Vec<_> = monitors.iter().map(Monitor::description).collect();
    let mut matches: Vec<_> = monitors.into_iter().filter(|m| {
        config.display.trim().is_empty() || m.description().eq_ignore_ascii_case(config.display.trim())
            || m.serial_number().as_deref() == Some(config.display.trim())
    }).collect();
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(format!("Target monitor unavailable. Connected DDC displays: {names:?}. Check the connection and DDC/CI.").into()),
        _ => Err("More than one monitor matches. Set display to an exact name or unique serial in the config.".into()),
    }
}

pub fn current(config: &Config) -> Result<()> {
    let mut monitor = select(config)?;
    let value = monitor.get_vcp_feature(0x60)?.value();
    let name = Input::ALL.into_iter().find(|i| config.binding(*i).value == value).map(Input::name).unwrap_or("unconfigured input");
    println!("{}: {name} (0x{value:02x})", monitor.description());
    Ok(())
}

pub fn switch(config: &Config, input: Input) -> Result<()> {
    let mut monitor = select(config)?;
    let value = config.binding(input).value;
    // Send directly: inactive inputs may permit writes but not current-input reads.
    monitor.set_vcp_feature(0x60, value)?;
    println!("Sent {} (0x{value:02x}) to {}", input.name(), monitor.description());
    Ok(())
}
