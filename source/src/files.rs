use crate::Result;
use std::{fs, io::Write, path::Path};

/// Replace complete contents atomically; preserve existing permissions and symlinks.
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let target = if path.exists() { fs::canonicalize(path)? } else { path.to_path_buf() };
    let parent = target.parent().ok_or("File has no parent directory")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".monitor-keys-{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
    let result = (|| -> Result<()> {
        if let Ok(metadata) = fs::metadata(&target) { file.set_permissions(metadata.permissions())?; }
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, target)?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(temporary); }
    result
}
