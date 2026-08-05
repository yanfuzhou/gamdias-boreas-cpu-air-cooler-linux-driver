mod superio;
mod configuration;

use std::time::Duration;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::io::{Error, ErrorKind};

use regex::Regex;
use walkdir::WalkDir;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use superio::{CPU_TEM0, CPU_TEM1, CPU_FAN0};
use configuration::{Sensors, Mode, Config};

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

fn get_cpu_temp_sensor(lines: &[String])  -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Find the correct hwmon directory dynamically
    let hwmon_dir = find_sensor_path(&lines)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "no cpu driver found in hwmon"))?;
    
    // Construct the path to temp1_input
    let temp_path = hwmon_dir.join("temp1_input");

    Ok(temp_path)
}

fn read_config() -> Result<(), Box<dyn std::error::Error>> {
    // Opens file relative to the project root directory
    let mut file = File::open("config.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: Config = serde_json::from_str(&contents)?;

    Ok(config)
}

fn main() {
    // Opens file relative to the project root directory
    let config = match read_config() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error reading configuration: {}", e);
            return;
        }
    };
    println!("update_interval_ms: {:?}", config.update_interval_ms);
    println!("sensors: {:?}", config.sensors);
    println!("display: {:?}", config.display);

    let cpu_tem0: Vec<String> = CPU_TEM0.iter().map(|&s| s.to_string()).collect();
    let temp_path = match get_cpu_temp_sensor(&cpu_tem0) {
        Ok(v) => v,
        Err(e1) => {
            eprintln!("Error finding CPU temperature sensor #0: {}", e1);
            let cpu_tem1: Vec<String> = CPU_TEM1.iter().map(|&s| s.to_string()).collect();
            match get_cpu_temp_sensor(&cpu_tem1) {
                Ok(v) => v,
                Err(e2) => {
                    eprintln!("Error finding CPU temperature sensor #1: {}", e2);
                    return;
                }
            }
        }
    };

    let cpu_fan0: Vec<String> = CPU_FAN0.iter().map(|&s| s.to_string()).collect();
    let fan_path = match get_cpu_fan_speed_sensor(&cpu_fan0) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error finding CPU fan speed sensor #0: {}", e);
            return;
        }
    };

    // 1. Enable raw mode to read keys immediately without waiting for Enter
    enable_raw_mode()?;
    println!("Loop started. Press 'q' or 'Esc' to interrupt...\r");

    // Your conditional variable for the while loop
    let mut running = true;

    while running {
        // 2. Perform your background loop tasks here

        // Read and parse the raw RPM string
        let raw_speed = match read_to_string(fan_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error reading CPU raw speed: {}", e);
                return;
            }
        };
        let rpm: u32 = match raw_speed
            .trim()
            .parse()
            .map_err(|e| Error::new(ErrorKind::InvalidData, e)) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error converting CPU raw speed to rpm: {}", e);
                    return;
                }            
            };
        
        // Read and parse the raw temperature string
        let raw_temp = match read_to_string(temp_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error reading CPU raw temperature: {}", e);
                return;
            }
        };
        let milli_celsius: i32 = match raw_temp.trim().parse() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error converting CPU raw temperature to celsius: {}", e);
                return;
            }
        };
        let temp = milli_celsius as f32 / 1000.0;

        println!("CPU Temperature: {} °C", temp);
        println!("Fan 1 Speed: {} RPM", rpm);

        // 3. Poll for keyboard events for 50 milliseconds (Non-blocking)
        if event::poll(Duration::from_millis(50))? {
            // 4. Read the available event
            if let Event::Key(key) = event::read()? {
                // Filter out key release events (mainly relevant on Windows)
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // 5. Check if the interrupt key was pressed
                        KeyCode::Char('q') | KeyCode::Esc => {
                            println!("\nInterrupt received! Exiting loop...\r");
                            running = false; 
                        }
                        _ => {}
                    }
                }
            }
        }

        // Throttle the loop slightly if your task doesn't take time
        std::thread::sleep(Duration::from_millis(200));
    }

    // 6. Always disable raw mode before exiting to restore standard terminal behavior
    disable_raw_mode()?;
    println!("Program finished successfully.");
}
