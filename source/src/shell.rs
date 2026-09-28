use crate::{files::write_atomic, service::Paths, Result};
use std::{env, fs, path::PathBuf};

const START: &str = "# >>> Monitor Keys: managed mk command >>>";
const END: &str = "# <<< Monitor Keys: managed mk command <<<";
const BLOCK: &str = "# >>> Monitor Keys: managed mk command >>>\nfunction mk {\n  \"$HOME/Library/Application Support/Monitor Keys/monitor-keys\" \"$@\"\n}\n# <<< Monitor Keys: managed mk command <<<\n";

fn bounds(text: &str) -> Result<Option<(usize, usize)>> {
    let starts: Vec<_> = text.match_indices(START).collect();
    let ends: Vec<_> = text.match_indices(END).collect();
    if starts.is_empty() && ends.is_empty() { return Ok(None); }
    if starts.len() != 1 || ends.len() != 1 || starts[0].0 >= ends[0].0 {
        return Err("The managed mk block in your shell configuration is malformed; it was left unchanged.".into());
    }
    let start = starts[0].0;
    let mut end = ends[0].0 + END.len();
    if text.as_bytes().get(end) == Some(&b'\n') { end += 1; }
    Ok(Some((start, end)))
}

fn install_text(original: &str) -> Result<String> {
    if let Some((start, end)) = bounds(original)? {
        if &original[start..end] != BLOCK {
            return Err("The managed mk function was edited; it was left unchanged.".into());
        }
        return Ok(original.to_owned());
    }
    for line in original.lines().map(str::trim).filter(|line| !line.starts_with('#')) {
        let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.starts_with("mk()") || compact.starts_with("functionmk{") || compact.starts_with("functionmk()") || compact.starts_with("aliasmk=") {
            return Err("Your shell configuration already defines mk. It was left unchanged; rename that definition before installing the managed mk command.".into());
        }
    }
    let separator = if original.is_empty() || original.ends_with('\n') { "" } else { "\n" };
    Ok(format!("{original}{separator}{BLOCK}"))
}

fn path(paths: &Paths) -> Result<PathBuf> {
    let record = paths.data.join("shell-config-path.txt");
    if record.exists() { return Ok(PathBuf::from(fs::read_to_string(record)?)); }
    let root = env::var_os("ZDOTDIR").filter(|value| !value.is_empty()).or_else(|| env::var_os("HOME")).ok_or("HOME is unavailable")?;
    Ok(PathBuf::from(root).join(".zshrc"))
}

pub fn install(paths: &Paths) -> Result<()> {
    let file = path(paths)?;
    let original = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let updated = install_text(&original)?;
    if updated != original { write_atomic(&file, &updated)?; }
    write_atomic(&paths.data.join("shell-config-path.txt"), file.to_str().ok_or("Non-UTF8 shell configuration path")?)?;
    println!("Permanent mk command configured. Open a new Terminal tab, or reload {}.", file.display());
    Ok(())
}

pub fn uninstall(paths: &Paths) -> Result<()> {
    let file = path(paths)?;
    if !file.exists() { return Ok(()); }
    let original = fs::read_to_string(&file)?;
    if let Some((start, end)) = bounds(&original)? {
        if &original[start..end] != BLOCK { return Err("Edited mk block preserved in shell configuration.".into()); }
        let updated = format!("{}{}", &original[..start], &original[end..]);
        write_atomic(&file, &updated)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_user_content_and_installs_only_once() {
        let original = "# user settings\nexport EDITOR=vim\n";
        let updated = install_text(original).unwrap();
        assert!(updated.starts_with(original));
        assert_eq!(install_text(&updated).unwrap(), updated);
        let (start, end) = bounds(&updated).unwrap().unwrap();
        assert_eq!(format!("{}{}", &updated[..start], &updated[end..]), original);
    }
    #[test]
    fn refuses_custom_functions_and_malformed_blocks() {
        for text in ["mk() { echo custom; }", "function mk { echo custom; }", "alias mk=make", START, END] {
            assert!(install_text(text).is_err(), "{text}");
        }
        assert!(install_text(&BLOCK.replace("function mk", "function custom")).is_err());
    }
}
