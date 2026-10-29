//! Output choices and default file naming.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutputRequest {
    pub copy: bool,
    pub save: Option<PathBuf>,
    pub pin: bool,
}

impl OutputRequest {
    #[must_use]
    pub fn copy_only() -> Self {
        Self {
            copy: true,
            save: None,
            pin: false,
        }
    }

    #[must_use]
    pub fn has_output(&self) -> bool {
        self.copy || self.save.is_some() || self.pin
    }
}

#[must_use]
pub fn default_file_name(timestamp: &str) -> String {
    format!("Cosnip_{timestamp}.png")
}

/// Return a path that does not exist yet, appending `-1`, `-2`, ...
#[must_use]
pub fn unique_path(directory: &Path, file_name: &str) -> PathBuf {
    let candidate = directory.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Cosnip");
    let extension = Path::new(file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    for index in 1..10_000u32 {
        let name = format!("{stem}-{index}.{extension}");
        let candidate = directory.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(format!("{stem}-{}.{}", std::process::id(), extension))
}

#[cfg(test)]
mod tests {
    use super::{default_file_name, unique_path};

    #[test]
    fn names_are_stable() {
        assert_eq!(
            default_file_name("2026-10-29_10-00-00"),
            "Cosnip_2026-10-29_10-00-00.png"
        );
    }

    #[test]
    fn unique_path_appends_index() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cosnip_x.png"), b"x").unwrap();
        let p = unique_path(dir.path(), "Cosnip_x.png");
        assert_eq!(p.file_name().unwrap(), "Cosnip_x-1.png");
    }
}
