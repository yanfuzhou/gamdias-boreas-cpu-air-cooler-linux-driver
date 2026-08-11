// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 BOREAS Linux Project Contributors

use std::fs;
use std::path::{Path, PathBuf};

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
    /// Creates a new SensorReader.
    ///
    /// If a path is not supplied, the corresponding sensor is automatically
    /// detected from Linux sysfs.
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

        let content = fs::read_to_string(path).ok()?;
        let milli_celsius: i64 = content.trim().parse().ok()?;

        Some(milli_celsius as f64 / 1000.0)
    }

    pub fn read_cpu_fan_speed(&self) -> Option<i32> {
        let path = self.cpu_fan_path.as_ref()?;

        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(path).ok()?;
        content.trim().parse::<i32>().ok()
    }

    fn detect_cpu_temp_path() -> Option<PathBuf> {
        const CPU_SENSOR_NAMES: &[&str] = &[
            "coretemp",
            "k10temp",
            "zenpower",
            "cpu_thermal",
            "acpitz",
        ];

        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon = entry.path();

                if !hwmon.is_dir() {
                    continue;
                }

                let name_path = hwmon.join("name");

                let Ok(name) = fs::read_to_string(&name_path) else {
                    continue;
                };

                let name = name.trim().to_lowercase();

                if CPU_SENSOR_NAMES
                    .iter()
                    .any(|sensor_name| name.contains(sensor_name))
                {
                    let temp_path = hwmon.join("temp1_input");

                    if temp_path.exists() {
                        return Some(temp_path);
                    }
                }
            }
        }

        let thermal_path =
            PathBuf::from("/sys/class/thermal/thermal_zone0/temp");

        if thermal_path.exists() {
            return Some(thermal_path);
        }

        None
    }

    fn detect_cpu_fan_path() -> Option<PathBuf> {
        const FAN_SENSOR_NAMES: &[&str] = &[
            "nct6687",
            "nct6798",
            "nct6775",
            "nct",
            "it87",
            "asus",
            "dell",
        ];

        let entries = fs::read_dir("/sys/class/hwmon").ok()?;

        for entry in entries.flatten() {
            let hwmon = entry.path();

            if !hwmon.is_dir() {
                continue;
            }

            let name_path = hwmon.join("name");

            let Ok(name) = fs::read_to_string(&name_path) else {
                continue;
            };

            let name = name.trim().to_lowercase();

            if !FAN_SENSOR_NAMES
                .iter()
                .any(|sensor_name| name.contains(sensor_name))
            {
                continue;
            }

            let fan_path = hwmon.join("fan1_input");

            if !fan_path.exists() {
                continue;
            }

            let Ok(content) = fs::read_to_string(&fan_path) else {
                continue;
            };

            if let Ok(rpm) = content.trim().parse::<i32>() {
                if rpm > 0 {
                    return Some(fan_path);
                }
            }
        }

        None
    }

    pub fn list_available_sensors() -> Vec<SensorInfo> {
        let mut sensors = Vec::new();

        //
        // /sys/class/hwmon
        //
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon = entry.path();

                if !hwmon.is_dir() {
                    continue;
                }

                let name = fs::read_to_string(hwmon.join("name"))
                    .map(|s| s.trim().to_owned())
                    .unwrap_or_else(|_| "unknown".to_owned());

                if let Ok(files) = fs::read_dir(&hwmon) {
                    for file in files.flatten() {
                        let path = file.path();

                        let Some(filename) =
                            path.file_name().and_then(|f| f.to_str())
                        else {
                            continue;
                        };

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

        //
        // /sys/class/thermal
        //
        if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let zone = entry.path();

                if !zone.is_dir() {
                    continue;
                }

                let Some(dirname) =
                    zone.file_name().and_then(|name| name.to_str())
                else {
                    continue;
                };

                if !dirname.starts_with("thermal_zone") {
                    continue;
                }

                let temp_path = zone.join("temp");

                if !temp_path.exists() {
                    continue;
                }

                let sensor_name = fs::read_to_string(zone.join("type"))
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