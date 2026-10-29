//! Output paths and atomic PNG writes.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use cosnip_core::output::unique_path;

use crate::PlatformError;

pub fn pictures_dir() -> Result<PathBuf, PlatformError> {
    if let Some(path) = directories::UserDirs::new()
        .and_then(|dirs| dirs.picture_dir().map(std::path::Path::to_path_buf))
    {
        return Ok(path);
    }
    let home = std::env::var_os("HOME").ok_or(PlatformError::NoPicturesDir)?;
    Ok(PathBuf::from(home).join("Pictures"))
}

pub fn default_save_path(timestamp: &str) -> Result<PathBuf, PlatformError> {
    let dir = pictures_dir()?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| PlatformError::CreateDir(format!("{}: {e}", dir.display())))?;
    }
    let name = cosnip_core::output::default_file_name(timestamp);
    Ok(unique_path(&dir, &name))
}

/// Write bytes through a restrictive temporary file and rename into place.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), PlatformError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    if !parent.is_dir() {
        return Err(PlatformError::Write(format!(
            "output directory does not exist: {}",
            parent.display()
        )));
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| PlatformError::Write("output path has no file name".into()))?;
    let temp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)
        .map_err(|e| PlatformError::Write(format!("{}: {e}", temp.display())))?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(PlatformError::Write(format!("{}: {e}", temp.display())));
    }
    drop(file);
    if let Err(e) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(PlatformError::Write(format!("{}: {e}", path.display())));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::atomic_write;

    #[test]
    fn writes_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.png");
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        assert!(std::fs::read_dir(dir.path()).unwrap().all(|e| {
            let name = e.unwrap().file_name();
            let name = name.to_string_lossy();
            name.starts_with("out.png") || name.starts_with(".out.png")
        }));
    }

    #[test]
    fn refuses_missing_directory() {
        let path = std::path::Path::new("/definitely/not/here/out.png");
        assert!(atomic_write(path, b"x").is_err());
    }
}
