use std::path::{Path, PathBuf};
use std::fs::{read_to_string, File};
use std::io::{Error, ErrorKind, Read};

use regex::Regex;
use walkdir::WalkDir;
use serde::{Deserialize, Serialize};

use crate::data::{CPU_TEM0, CPU_TEM1, CPU_FAN0};

pub const VERBOSE: bool = false; // Set to true to enable verbose logging

#[derive(Debug, Deserialize, Serialize)]
struct Sensors {
    cpu_temp_path: Option<String>,
    cpu_fan_path: Option<String>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Mode {
    pub mode: String,
    pub duration_seconds: u32
}

#[derive(Debug, Deserialize)]
struct Config {
    update_interval_ms: u32,
    sensors: Sensors,
    display: Vec<Mode>,
}

fn find_sensor_path(lines: &[String]) -> Option<PathBuf> {
    let root = "/sys/class/hwmon";

    // Verify existence safely and it is a directory
    if Path::new(root).is_dir() {
        let pattern = Regex::new(r"^hwmon[0-9]$").unwrap();

        // Read the main hwmon class directory
        for entry in WalkDir::new(root)
        .min_depth(1).max_depth(1)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok()) {
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy();
                if pattern.is_match(&name) {
                    let path = entry.into_path();
                    let name_path = path.join("name");

                    // Read the "name" file inside the hwmon folder to check the driver
                    if let Ok(name) = read_to_string(name_path) {
                        let search_name = name.trim();
                        if lines.contains(&String::from(search_name)) {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_cpu_fan_speed_sensor(lines: &[String]) -> Result<PathBuf, Error> {
    // Find the correct hwmon directory dynamically
    let hwmon_dir = find_sensor_path(&lines)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "no chip driver found in hwmon"))?;

    // Construct the path to fan1_input
    let fan_path = hwmon_dir.join("fan1_input");

    Ok(fan_path)
}

fn get_cpu_temp_sensor(lines: &[String]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Find the correct hwmon directory dynamically
    let hwmon_dir = find_sensor_path(&lines)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "no cpu driver found in hwmon"))?;
    
    // Construct the path to temp1_input
    let temp_path = hwmon_dir.join("temp1_input");

    Ok(temp_path)
}

fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Opens file relative to the project root directory
    let mut file = File::open("config.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: Config = serde_json::from_str(&contents)?;

    Ok(config)
}

pub fn read_temp_rpm(temp_path: &PathBuf, fan_path: &PathBuf) -> (f32, i32) {
        // Read and parse the raw temperature string
        let raw_temp = read_to_string(temp_path).unwrap_or(String::from("-1"));
        let milli_celsius: i32 = raw_temp.trim().parse().unwrap_or(-1);
        let temp = milli_celsius as f32 / 1000.0;

        // Read and parse the raw RPM string
        let raw_speed = read_to_string(fan_path).unwrap_or(String::from("-1"));
        let rpm: i32 = raw_speed.trim().parse().unwrap_or(-1);

        (temp, rpm)
}

pub fn get_run_parameters() -> (u32, Vec<Mode>, PathBuf, PathBuf) {
    // Opens file relative to the project root directory
    let config = read_config().unwrap_or(
        Config { 
            update_interval_ms: 500, 
            sensors: Sensors { 
                cpu_temp_path: None, 
                cpu_fan_path: None 
            }, 
            display: vec![
                Mode {
                    mode: "CpuTempCelsius".to_string(), 
                    duration_seconds: 10
                }, 
                Mode {
                    mode: "CpuFanSpeed".to_string(), 
                    duration_seconds: 6
                }
                ]
            }
    );
    println!("Configuration loaded: {:?}", config);
    let update_interval_ms = config.update_interval_ms;
    let boreas_display = config.display;
    let cpu_sensors = config.sensors;

    let temp_path: PathBuf;

    if cpu_sensors.cpu_temp_path.is_none() {
        let cpu_tem0: Vec<String> = CPU_TEM0.iter().map(|&s| s.to_string()).collect();
        let cpu_tem1: Vec<String> = CPU_TEM1.iter().map(|&s| s.to_string()).collect();
        temp_path = get_cpu_temp_sensor(&cpu_tem0)
            .unwrap_or(get_cpu_temp_sensor(&cpu_tem1)
            .expect("Failed to find CPU temperature sensor"));
    } else {
        temp_path = PathBuf::from(cpu_sensors.cpu_temp_path.unwrap());
    }

    let fan_path: PathBuf;

    if cpu_sensors.cpu_fan_path.is_none() {
        let cpu_fan0: Vec<String> = CPU_FAN0.iter().map(|&s| s.to_string()).collect();
        fan_path = get_cpu_fan_speed_sensor(&cpu_fan0)
            .expect("Failed to find CPU fan speed sensor");
    } else {
        fan_path = PathBuf::from(cpu_sensors.cpu_fan_path.unwrap());
    }

    (update_interval_ms, boreas_display, temp_path, fan_path)
}
