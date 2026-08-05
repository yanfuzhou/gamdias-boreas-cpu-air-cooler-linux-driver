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

fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Opens file relative to the project root directory
    let mut file = File::open("config.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: Config = serde_json::from_str(&contents)?;

    Ok(config)
}