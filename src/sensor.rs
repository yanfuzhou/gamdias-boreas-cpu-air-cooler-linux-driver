// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 BOREAS Linux Project Contributors

use std::path::{
    Path, 
    PathBuf
};
use std::fs::{
    read_dir, 
    read_to_string
};

use crate::data::{
    CPU_TEM0, 
    CPU_TEM1, 
    CPU_FAN0
};

pub struct SensorReader {
    cpu_temp_path: Option<PathBuf>,
    cpu_fan_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SensorInfo {
    pub path: PathBuf,
    pub name: String,
    pub sensor_type: String,
}

impl SensorReader {
    pub fn new(
        cpu_temp_path: Option<PathBuf>,
        cpu_fan_path: Option<PathBuf>,
    ) -> Self {
        Self {
            cpu_temp_path: cpu_temp_path.or_else(Self::detect_cpu_temp_path),
            cpu_fan_path: cpu_fan_path.or_else(Self::detect_cpu_fan_path),
        }
    }

    pub fn cpu_temp_path(&self) -> Option<&Path> {
        self.cpu_temp_path.as_deref()
    }

    pub fn cpu_fan_path(&self) -> Option<&Path> {
        self.cpu_fan_path.as_deref()
    }

    pub fn read_cpu_temperature(&self) -> Option<f64> {
        let path = self.cpu_temp_path.as_ref()?;

        if !path.exists() {
            return None;
        }

        let content = read_to_string(path).ok()?;
        let milli_celsius: i64 = content.trim().parse().ok()?;

        Some(milli_celsius as f64 / 1000.0)
    }

    pub fn read_cpu_fan_speed(&self) -> Option<i32> {
        let path = self.cpu_fan_path.as_ref()?;

        if !path.exists() {
            return None;
        }

        let content = read_to_string(path).ok()?;
        content.trim().parse::<i32>().ok()
    }

    fn detect_cpu_temp_path() -> Option<PathBuf> {
        let hwmon = match find_sensor_path(&CPU_TEM0) {
            Some(p) => p,
            None => match find_sensor_path(&CPU_TEM1) {
                Some(p) => p,
                None => return None
            }
        };

        validate_path(hwmon, "temp1_input")
    }

    fn detect_cpu_fan_path() -> Option<PathBuf> {
        let hwmon = match find_sensor_path(&CPU_FAN0) {
            Some(p) => p,
            None => return None
        };

        validate_path(hwmon, "fan1_input")
    }

    pub fn list_available_sensors() -> Vec<SensorInfo> {
        let mut sensors = Vec::new();

        if let Ok(entries) = read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon = entry.path();

                if !hwmon.is_dir() {
                    continue;
                }

                let name = read_to_string(hwmon.join("name"))
                    .map(|s| s.trim().to_owned())
                    .unwrap_or_else(|_| "unknown".to_owned());

                if let Ok(files) = read_dir(&hwmon) {
                    for file in files.flatten() {
                        let path = file.path();

                        let Some(filename) = path.file_name().and_then(|f| f.to_str()) else { continue; };

                        if is_temp_input(filename) {
                            sensors.push(SensorInfo {
                                path,
                                name: name.clone(),
                                sensor_type: "temperature".to_owned(),
                            });
                        } else if is_fan_input(filename) {
                            sensors.push(SensorInfo {
                                path,
                                name: name.clone(),
                                sensor_type: "fan".to_owned(),
                            });
                        }
                    }
                }
            }
        }

        if let Ok(entries) = read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let zone = entry.path();

                if !zone.is_dir() {
                    continue;
                }

                let Some(dirname) = zone.file_name().and_then(|name| name.to_str()) else { continue; };

                if !dirname.starts_with("thermal_zone") {
                    continue;
                }

                let temp_path = zone.join("temp");

                if !temp_path.exists() {
                    continue;
                }

                let sensor_name = read_to_string(zone.join("type"))
                    .map(|s| s.trim().to_owned())
                    .unwrap_or_else(|_| "thermal_zone".to_owned());

                sensors.push(SensorInfo {
                    path: temp_path,
                    name: sensor_name,
                    sensor_type: "temperature".to_owned(),
                });
            }
        }

        sensors
    }
}

fn is_temp_input(filename: &str) -> bool {
    filename.starts_with("temp") && filename.ends_with("_input")
}

fn is_fan_input(filename: &str) -> bool {
    filename.starts_with("fan") && filename.ends_with("_input")
}

fn find_sensor_path(names: &[&str]) -> Option<PathBuf> {
    if let Ok(entries) = read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let hwmon = entry.path();

            if !hwmon.is_dir() {
                continue;
            }

            let name_path = hwmon.join("name");

            let Ok(name) = read_to_string(&name_path) else {
                continue;
            };

            let name = name.trim().to_lowercase();

            if names
                .iter()
                .any(|sensor_name| name.contains(sensor_name))
            {
                return Some(hwmon);
            }
        }
    }
    None
}

fn validate_path(hwmon: PathBuf, sensor_input: &str) -> Option<PathBuf> {
    let path = hwmon.join(sensor_input);

    if path.exists() {
        let Ok(content) = read_to_string(&path) else {
            return None;
        };

        if let Ok(v) = content.trim().parse::<i32>() {
            if is_fan_input(sensor_input) {
                if v > 0 {
                    return Some(path);
                }
            } else if is_temp_input(sensor_input) {
                return Some(path);
            } else {
                return None;
            }
        }
    }

    None
}