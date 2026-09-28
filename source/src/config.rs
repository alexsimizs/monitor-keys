use crate::Result;
use clap::ValueEnum;
use global_hotkey::hotkey::HotKey;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

pub const TEMPLATE: &str = r#"# Monitor Keys — restart after editing: monitor-keys restart
# Only plain F1–F20 keys are accepted. On MX Keys Mini, use Fn as needed.
# Empty display = the only DDC-capable external display.
# When using more displays, enter an exact name or serial from `displays`.
display = ""

[USB-C]
key = "F1"
value = 0x1b

[HDMI]
key = "F2"
value = 0x11

[DisplayPort]
key = "F3"
value = 0x0f
"#;

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum Input {
    #[value(name = "usb-c", alias = "usbc1", alias = "usbc")]
    Usbc1,
    #[value(name = "hdmi", alias = "hdmi1")]
    Hdmi1,
    #[value(name = "displayport", alias = "dp1", alias = "dp")]
    Dp1,
}
impl Input {
    pub const ALL: [Self; 3] = [Self::Usbc1, Self::Hdmi1, Self::Dp1];
    pub fn name(self) -> &'static str {
        match self { Self::Usbc1 => "USB-C", Self::Hdmi1 => "HDMI", Self::Dp1 => "DisplayPort" }
    }
    pub fn legacy_name(self) -> &'static str {
        match self { Self::Usbc1 => "usbc1", Self::Hdmi1 => "hdmi1", Self::Dp1 => "dp1" }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding { pub key: String, pub value: u16 }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub display: String,
    #[serde(rename = "USB-C", alias = "usbc1")]
    pub usbc1: Binding,
    #[serde(rename = "HDMI", alias = "hdmi1")]
    pub hdmi1: Binding,
    #[serde(rename = "DisplayPort", alias = "dp1")]
    pub dp1: Binding,
}

impl Config {
    pub fn set_key(&mut self, input: Input, key: &str) -> Result<()> {
        parse_key(key)?;
        let binding = match input { Input::Usbc1 => &mut self.usbc1, Input::Hdmi1 => &mut self.hdmi1, Input::Dp1 => &mut self.dp1 };
        binding.key = key.trim().to_ascii_uppercase();
        Ok(())
    }
    pub fn show_shortcuts(&self) {
        for input in Input::ALL { println!("{} → {}", self.binding(input).key, input.name()); }
    }
    pub fn binding(&self, input: Input) -> &Binding {
        match input { Input::Usbc1 => &self.usbc1, Input::Hdmi1 => &self.hdmi1, Input::Dp1 => &self.dp1 }
    }
    pub fn validate(&self) -> Result<()> {
        let mut keys = HashSet::new();
        let mut values = HashSet::new();
        for input in Input::ALL {
            let binding = self.binding(input);
            let key = parse_key(&binding.key)?;
            if !keys.insert(key.id()) { return Err(format!("{} is assigned more than once", binding.key).into()); }
            if binding.value == 0 || binding.value > 255 {
                return Err(format!("{} input code must be between 1 and 255", input.name()).into());
            }
            if !values.insert(binding.value) { return Err("Each input must have a different input code".into()); }
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {e}. Run `monitor-keys init` first.", path.display()))?;
        let config: Self = toml::from_str(&text)?;
        config.validate()?;
        Ok(config)
    }
}

pub fn edited_text(original: &str, config: &Config) -> Result<String> {
    config.validate()?;
    let mut doc: toml_edit::Document = original.parse()?;
    for input in Input::ALL {
        if let Some(item) = doc.remove(input.legacy_name()) { doc[input.name()] = item; }
        let item = &mut doc[input.name()]["key"];
        let decor = item.as_value().map(|value| value.decor().clone());
        *item = toml_edit::value(&config.binding(input).key);
        if let Some(decor) = decor { *item.as_value_mut().unwrap().decor_mut() = decor; }
    }
    let result = doc.to_string();
    let parsed: Config = toml::from_str(&result)?;
    parsed.validate()?;
    Ok(result)
}

pub fn parse_key(value: &str) -> Result<HotKey> {
    let canonical = value.trim().to_ascii_uppercase();
    let number = canonical.strip_prefix('F').and_then(|s| s.parse::<u8>().ok());
    match number {
        Some(n @ 1..=20) if canonical == format!("F{n}") => Ok(canonical.parse()?),
        _ => Err(format!("Invalid key {value:?}. Use one plain function key, F1 through F20.").into()),
    }
}

pub fn init(path: &Path) -> Result<()> {
    if path.exists() { Config::load(path)?; return Ok(()); }
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    use std::io::Write;
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(TEMPLATE.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_function_keys() {
        for n in 1..=20 { assert!(parse_key(&format!("F{n}")).is_ok()); }
        for key in ["F0", "F21", "F01", "3", "Fn+F3", "Command+F1", "Shift+F1", "A", ""] {
            assert!(parse_key(key).is_err(), "{key}");
        }
    }
    #[test]
    fn catches_ambiguous_and_invalid_config() {
        let mut c: Config = toml::from_str(TEMPLATE).unwrap();
        c.validate().unwrap();
        c.dp1.key = "f1".into();
        assert!(c.validate().is_err());
        c.dp1.key = "F3".into();
        c.dp1.value = c.hdmi1.value;
        assert!(c.validate().is_err());
        c.dp1.value = 256;
        assert!(c.validate().is_err());
        assert!(toml::from_str::<Config>(&TEMPLATE.replace("display =", "displaay =")).is_err());
    }
    #[test]
    fn migrates_legacy_config_and_preserves_input_codes_and_comments() {
        let old = TEMPLATE.replace("[USB-C]", "[usbc1]").replace("[HDMI]", "[hdmi1]").replace("[DisplayPort]", "[dp1]").replace("0x1b", "0x13 # monitor-specific");
        let mut c: Config = toml::from_str(&old).unwrap();
        c.set_key(Input::Usbc1, "f4").unwrap();
        let updated = edited_text(&old, &c).unwrap();
        assert!(updated.contains("[USB-C]") && updated.contains("[HDMI]") && updated.contains("[DisplayPort]"));
        assert!(!updated.contains("[usbc1]"));
        assert!(updated.contains("0x13 # monitor-specific"));
        let loaded: Config = toml::from_str(&updated).unwrap();
        assert_eq!(loaded.usbc1.key, "F4");
        assert_eq!(loaded.usbc1.value, 19);
        assert_eq!(edited_text(&updated, &loaded).unwrap(), updated);
    }
    #[test]
    fn validates_a_swap_as_a_single_change() {
        let mut c: Config = toml::from_str(TEMPLATE).unwrap();
        c.set_key(Input::Usbc1, "F3").unwrap();
        assert!(c.validate().is_err());
        c.set_key(Input::Dp1, "F1").unwrap();
        assert!(c.validate().is_ok());
    }
}
