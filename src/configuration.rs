// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 BOREAS Linux Project Contributors

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DisplayMode {
    CpuTempCelsius,
    CpuTempFahrenheit,
    CpuFanSpeed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayItem {
    #[serde(rename = "Mode", alias = "mode")]
    pub mode: DisplayMode,

    #[serde(rename = "DurationSeconds", alias = "durationSeconds")]
    pub duration_seconds: u64,
}

impl Default for DisplayItem {
    fn default() -> Self {
        Self {
            mode: DisplayMode::CpuTempCelsius,
            duration_seconds: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SensorConfig {
    #[serde(rename = "CpuTempPath", alias = "cpuTempPath")]
    pub cpu_temp_path: Option<String>,

    #[serde(rename = "CpuFanPath", alias = "cpuFanPath")]
    pub cpu_fan_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BoreasConfig {
    #[serde(rename = "UpdateIntervalMs", alias = "updateIntervalMs")]
    pub update_interval_ms: u64,

    #[serde(rename = "Sensors", alias = "sensors")]
    pub sensors: SensorConfig,

    #[serde(rename = "Display", alias = "display")]
    pub display: Vec<DisplayItem>,
}

impl Default for BoreasConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 500,
            sensors: SensorConfig::default(),
            display: vec![
                DisplayItem { mode: DisplayMode::CpuTempCelsius, duration_seconds: 10 }, 
                DisplayItem { mode: DisplayMode::CpuFanSpeed, duration_seconds: 6 }
            ]
        }
    }
}

impl BoreasConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::NotFound(
                path.display().to_string(),
            ));
        }

        let json = fs::read_to_string(path)?;

        let config: BoreasConfig = serde_json::from_str(&json).map_err(ConfigError::Parse)?;

        Ok(config)
    }

    pub fn save_sample<P: AsRef<Path>>(path: P) -> Result<(), ConfigError> {
        let sample = BoreasConfig::default();

        let json = serde_json::to_string_pretty(&sample).map_err(ConfigError::Parse)?;

        fs::write(path, json)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NotFound(String),
    Io(io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ConfigError::NotFound(path) => {
                write!(f, "Configuration file not found: {path}")
            }

            ConfigError::Io(err) => {
                write!(f, "I/O error: {err}")
            }

            ConfigError::Parse(err) => {
                write!(f, "Failed to parse configuration: {err}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}