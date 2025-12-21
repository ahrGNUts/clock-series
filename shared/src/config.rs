//! Configuration persistence utilities
//!
//! Provides functions for loading and saving clock configuration to disk.

use directories::ProjectDirs;
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

/// Error type for configuration operations
#[derive(Debug)]
pub enum ConfigError {
    /// Failed to determine config directory
    NoConfigDir,
    /// IO error while reading/writing config
    Io(io::Error),
    /// Failed to parse config file
    Parse(toml::de::Error),
    /// Failed to serialize config
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoConfigDir => write!(f, "Could not determine config directory"),
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Parse(e) => write!(f, "Parse error: {}", e),
            ConfigError::Serialize(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        ConfigError::Serialize(e)
    }
}

/// Get the base configuration directory for all clocks
pub fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "clock-series", "clocks")
        .map(|dirs| dirs.config_dir().to_path_buf())
}

/// Get the configuration file path for a specific clock
pub fn config_path(clock_name: &str) -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(format!("{}.toml", clock_name)))
}

/// Load configuration for a specific clock
///
/// Returns `None` if the config file doesn't exist yet.
/// Returns an error if the file exists but can't be parsed.
pub fn load_config<T: DeserializeOwned>(clock_name: &str) -> Result<Option<T>, ConfigError> {
    let path = config_path(clock_name).ok_or(ConfigError::NoConfigDir)?;
    
    if !path.exists() {
        return Ok(None);
    }
    
    let contents = fs::read_to_string(&path)?;
    let config: T = toml::from_str(&contents)?;
    Ok(Some(config))
}

/// Save configuration for a specific clock
pub fn save_config<T: Serialize>(clock_name: &str, config: &T) -> Result<(), ConfigError> {
    let path = config_path(clock_name).ok_or(ConfigError::NoConfigDir)?;
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}

/// Delete configuration for a specific clock
pub fn delete_config(clock_name: &str) -> Result<(), ConfigError> {
    let path = config_path(clock_name).ok_or(ConfigError::NoConfigDir)?;
    
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    #[test]
    fn test_config_dir() {
        let dir = config_dir();
        assert!(dir.is_some());
    }

    #[test]
    fn test_config_path() {
        let path = config_path("test_clock");
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("test_clock.toml"));
    }
}

