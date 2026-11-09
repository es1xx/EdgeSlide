use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Disabled,
    Volume,
    Brightness,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Edge {
    pub enabled: bool,
    pub action: Action,
    /// Fraction of full touchpad width.
    pub width: f64,
    /// Fraction of full touchpad height before activation.
    pub threshold: f64,
    pub sensitivity: f64,
    pub invert: bool,
}
impl Default for Edge {
    fn default() -> Self {
        Self {
            enabled: true,
            action: Action::Volume,
            width: 0.07,
            threshold: 0.04,
            sensitivity: 1.0,
            invert: false,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    /// Empty selects the first udev-classified touchpad; otherwise exact device name.
    pub touchpad: String,
    pub left: Edge,
    pub right: Edge,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_speed: f64,
    pub brightness_min: f64,
    pub brightness_speed: f64,
    /// Optional short-lived, click-through Wayland layer-shell indicator.
    pub indicator: bool,
    pub indicator_position: IndicatorPosition,
    pub indicator_accent: IndicatorAccent,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            touchpad: String::new(),
            left: Edge::default(),
            right: Edge {
                action: Action::Brightness,
                ..Edge::default()
            },
            volume_min: 0.0,
            volume_max: 1.0,
            volume_speed: 1.0,
            brightness_min: 0.10,
            brightness_speed: 1.0,
            indicator: false,
            indicator_position: IndicatorPosition::Bottom,
            indicator_accent: IndicatorAccent::Blue,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        for edge in [&self.left, &self.right] {
            range("edge width", edge.width, 0.01, 0.2)?;
            range("threshold", edge.threshold, 0.005, 0.3)?;
            range("sensitivity", edge.sensitivity, 0.1, 5.0)?;
        }
        range("volume minimum", self.volume_min, 0.0, 1.0)?;
        range(
            "volume maximum",
            self.volume_max,
            self.volume_min.max(0.01),
            1.5,
        )?;
        range("volume speed", self.volume_speed, 0.1, 5.0)?;
        range("brightness minimum", self.brightness_min, 0.05, 0.8)?;
        range("brightness speed", self.brightness_speed, 0.1, 5.0)
    }
    pub fn to_toml(&self) -> Result<String> {
        self.validate()?;
        Ok(toml::to_string_pretty(self)?)
    }
    pub fn parse(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text)?;
        config.validate()?;
        Ok(config)
    }
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text)
                .with_context(|| format!("Invalid configuration: {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save(&self) -> Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        self.validate()?;
        let path = config_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        let result = (|| -> Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)?;
            file.write_all(toml::to_string_pretty(self)?.as_bytes())?;
            file.sync_all()?;
            std::fs::rename(&temp, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }
}
pub fn config_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .context("HOME and XDG_CONFIG_HOME are unset")?;
    Ok(base.join("edgeslide/config.toml"))
}
fn range(name: &str, value: f64, min: f64, max: f64) -> Result<()> {
    if !value.is_finite() || !(min..=max).contains(&value) {
        bail!("{name} must be between {min} and {max}")
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_roundtrip() {
        let c = Config::default();
        Config::parse(&toml::to_string(&c).unwrap()).unwrap();
    }
    #[test]
    fn rejects_unsafe_values() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let c = Config {
                brightness_min: value,
                ..Config::default()
            };
            assert!(c.validate().is_err());
        }
    }
    #[test]
    fn rejects_unknown_fields() {
        assert!(Config::parse("volum_max = 10").is_err());
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorPosition {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorAccent {
    #[default]
    Blue,
    Amber,
    Green,
}
