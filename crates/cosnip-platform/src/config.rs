//! XDG TOML configuration.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{PlatformError, atomic_write, paths};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub overlay: OverlayConfig,
    pub annotations: AnnotationsConfig,
    pub pin: PinConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub default_output: DefaultOutput,
    /// Empty means the XDG Pictures directory.
    pub save_dir: String,
    pub filename: String,
    pub delay_ms: u64,
    /// Save/pin scaled monitors at their native pixel dimensions instead of the XWayland canvas.
    pub native_output: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_output: DefaultOutput::Copy,
            save_dir: String::new(),
            filename: "Cosnip_%Y-%m-%d_%H-%M-%S.png".into(),
            delay_ms: 0,
            native_output: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultOutput {
    #[default]
    Copy,
    Save,
    Pin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayConfig {
    pub cursor_mode: CursorMode,
    pub magnifier: bool,
    pub show_tips: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            cursor_mode: CursorMode::Auto,
            magnifier: true,
            show_tips: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorMode {
    #[default]
    Auto,
    Always,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnnotationsConfig {
    pub color: String,
    pub line_width: u32,
    pub font_size: u32,
    pub text_shadow: bool,
}

impl Default for AnnotationsConfig {
    fn default() -> Self {
        Self {
            color: "#FF4040".into(),
            line_width: 3,
            font_size: 20,
            text_shadow: true,
        }
    }
}

impl AnnotationsConfig {
    #[must_use]
    pub fn color_rgba(&self) -> [u8; 4] {
        parse_color(&self.color).unwrap_or([255, 64, 64, 255])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PinConfig {
    /// Multiplier applied to the wheel delta. 0.002 is the current Cosnip default.
    pub zoom_step: f32,
    /// How pins are displayed. `auto` uses the GNOME Shell extension when it is running.
    pub backend: PinBackend,
}

impl Default for PinConfig {
    fn default() -> Self {
        Self {
            zoom_step: 0.002,
            backend: PinBackend::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PinBackend {
    /// Use the Shell extension when its D-Bus service is present, otherwise a window.
    #[default]
    Auto,
    /// Always use the XWayland pin window.
    Window,
    /// Require the GNOME Shell extension and fail if it is not running.
    Shell,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(ConfigError::Read(error.to_string())),
        };
        let config: Self =
            toml::from_str(&text).map_err(|error| ConfigError::Parse(error.to_string()))?;
        Ok(config.normalized())
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| ConfigError::Write(error.to_string()))?;
        }
        let text =
            toml::to_string_pretty(self).map_err(|error| ConfigError::Write(error.to_string()))?;
        atomic_write(path, text.as_bytes()).map_err(|error| ConfigError::Write(error.to_string()))
    }

    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.annotations.line_width = self.annotations.line_width.clamp(1, 32);
        self.annotations.font_size = self.annotations.font_size.clamp(6, 96);
        self.pin.zoom_step = self.pin.zoom_step.clamp(0.0001, 0.05);
        if self.general.filename.trim().is_empty() {
            self.general.filename = GeneralConfig::default().filename;
        }
        self
    }

    #[must_use]
    pub fn save_dir(&self) -> Option<PathBuf> {
        let path = self.general.save_dir.trim();
        (!path.is_empty()).then(|| PathBuf::from(path))
    }

    /// Format the configured file name for `now`, ensuring a `.png` suffix.
    #[must_use]
    pub fn file_name(&self, now: &jiff::Zoned) -> String {
        format_file_name(&self.general.filename, now)
    }

    pub fn save_path(&self) -> Result<PathBuf, PlatformError> {
        let now = jiff::Zoned::now();
        paths::save_path(self.save_dir().as_deref(), &self.file_name(&now))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config: {0}")]
    Read(String),
    #[error("cannot parse config: {0}")]
    Parse(String),
    #[error("cannot write config: {0}")]
    Write(String),
}

impl From<PlatformError> for ConfigError {
    fn from(error: PlatformError) -> Self {
        Self::Write(error.to_string())
    }
}

#[must_use]
pub fn config_path() -> PathBuf {
    paths::config_path()
}

/// Format a file-name pattern through `strftime`, strip path separators, and add `.png`.
#[must_use]
pub fn format_file_name(pattern: &str, now: &jiff::Zoned) -> String {
    let name = if pattern.contains('%') {
        now.strftime(pattern).to_string()
    } else {
        pattern.to_string()
    };
    let name = name.replace(['/', '\\'], "-");
    if name.to_ascii_lowercase().ends_with(".png") {
        name
    } else {
        format!("{name}.png")
    }
}

fn parse_color(value: &str) -> Option<[u8; 4]> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    match value.len() {
        6 => {
            let red = u8::from_str_radix(&value[0..2], 16).ok()?;
            let green = u8::from_str_radix(&value[2..4], 16).ok()?;
            let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
            Some([red, green, blue, 255])
        }
        8 => {
            let red = u8::from_str_radix(&value[0..2], 16).ok()?;
            let green = u8::from_str_radix(&value[2..4], 16).ok()?;
            let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
            let alpha = u8::from_str_radix(&value[6..8], 16).ok()?;
            Some([red, green, blue, alpha])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, DefaultOutput, PinBackend};

    #[test]
    fn defaults_are_valid() {
        let config = Config::default().normalized();
        assert_eq!(config.general.default_output, DefaultOutput::Copy);
        assert_eq!(config.annotations.color_rgba(), [255, 64, 64, 255]);
        assert_eq!(config.annotations.line_width, 3);
        assert_eq!(config.pin.backend, PinBackend::Auto);
    }

    #[test]
    fn roundtrip_preserves_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = Config::default();
        config.general.default_output = DefaultOutput::Save;
        config.general.save_dir = "/tmp/pictures".into();
        config.annotations.color = "#112233".into();
        config.annotations.font_size = 28;
        config.pin.backend = PinBackend::Shell;
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.general.default_output, DefaultOutput::Save);
        assert_eq!(loaded.general.save_dir, "/tmp/pictures");
        assert_eq!(loaded.annotations.color_rgba(), [0x11, 0x22, 0x33, 255]);
        assert_eq!(loaded.annotations.font_size, 28);
        assert_eq!(loaded.pin.backend, PinBackend::Shell);
    }

    #[test]
    fn missing_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(&dir.path().join("missing.toml")).unwrap();
        assert_eq!(config.general.default_output, DefaultOutput::Copy);
    }

    #[test]
    fn invalid_toml_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "not = [valid").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn values_are_clamped() {
        let mut config = Config::default();
        config.annotations.line_width = 0;
        config.annotations.font_size = 999;
        config.pin.zoom_step = 5.0;
        let config = config.normalized();
        assert_eq!(config.annotations.line_width, 1);
        assert_eq!(config.annotations.font_size, 96);
        assert_eq!(config.pin.zoom_step, 0.05);
    }
}
