use std::io::Read;
use std::fs::File;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Sensors {
    cpu_temp_path: Option<String>,
    cpu_fan_path: Option<String>
}

#[derive(Debug, Deserialize, Serialize)]
struct Mode {
    mode: String,
    duration_seconds: u32
}

#[derive(Debug, Deserialize)]
struct Config {
    update_interval_ms: u32,
    sensors: Sensors,
    display: Vec<Mode>,
}