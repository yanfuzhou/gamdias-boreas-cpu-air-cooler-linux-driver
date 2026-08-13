// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 BOREAS Linux Project Contributors

mod configuration;
mod data;
mod device;
mod protocol;
mod sensor;

use std::{
    env,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process, 
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    thread,  
    time::{Duration, Instant}
};

use signal_hook::{
    consts::signal::{SIGINT, SIGTERM},
    iterator::Signals
};

use crate::configuration::{
    BoreasConfig, 
    DisplayItem, 
    DisplayMode
};
use crate::device::BoreasDevice;
use crate::protocol::boreas_protocol::celsius_to_fahrenheit;
use crate::sensor::SensorReader;

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Some(first) = args.first() {
        match first.to_ascii_lowercase().as_str() {
            "--help" | "-h" => {
                print_help();
                return 0;
            }
            "--list-sensors" => {
                list_sensors();
                return 0;
            }
            "--generate-config" => {
                let path = args.get(1).map(String::as_str).unwrap_or("config.json");
                return generate_config(path);
            }
            "--test" => return run_test(),
            _ => {}
        }
    }

    let requested_config = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/boreas/config.json"));

    let config_file = match resolve_config_file(&requested_config) {
        Some(path) => path,
        None => {
            eprintln!("Configuration file not found: {}", requested_config.display());
            eprintln!("Run with --generate-config to create a sample configuration.");
            return 1;
        }
    };

    let config = match BoreasConfig::load(&config_file) {
        Ok(config) => {
            println!("Loaded configuration from: {}", config_file.display());
            config
        }
        Err(err) => {
            eprintln!("Failed to load configuration: {err}");
            return 1;
        }
    };

    let running = Arc::new(AtomicBool::new(true));
    if let Err(err) = install_signal_handlers(Arc::clone(&running)) {
        eprintln!("Failed to install signal handlers: {err}");
        return 1;
    }

    run_daemon(config, &running)
}

fn resolve_config_file(requested: &Path) -> Option<PathBuf> {
    if requested.exists() {
        return Some(requested.to_path_buf());
    }

    let local_config = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("config.json")));

    local_config.filter(|path| path.exists())
}

fn install_signal_handlers(running: Arc<AtomicBool>) -> io::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM])?;

    thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            match signal {
                SIGINT => println!("Shutdown requested..."),
                SIGTERM => println!("SIGTERM received, shutting down..."),
                _ => {}
            }
            running.store(false, Ordering::SeqCst);
        }
    });

    Ok(())
}

fn run_daemon(config: BoreasConfig, running: &AtomicBool) -> i32 {
    let sensors = SensorReader::new(
        config.sensors.cpu_temp_path.clone().map(PathBuf::from),
        config.sensors.cpu_fan_path.clone().map(PathBuf::from),
    );

    println!(
        "CPU Temp sensor: {}",
        sensors
            .cpu_temp_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not found".to_owned())
    );
    println!(
        "CPU Fan sensor: {}",
        sensors
            .cpu_fan_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not found".to_owned())
    );

    let mut device = BoreasDevice::new();
    println!("Connecting to BOREAS display...");

    let mut retry_count = 0u64;
    while !device.connect() && running.load(Ordering::SeqCst) {
        retry_count += 1;
        if retry_count % 10 == 1 {
            println!("Device not found, waiting...");
        }
        thread::sleep(Duration::from_secs(1));
    }

    if !running.load(Ordering::SeqCst) {
        return 0;
    }

    println!("Connected to BOREAS display");
    let _ = device.initialize();

    let mut current_display_index = 0usize;
    let mut last_rotation = Instant::now();
    println!("Starting display loop...");

    while running.load(Ordering::SeqCst) {
        if config.display.len() > 1 {
            let duration_seconds = config.display[current_display_index].duration_seconds;
            if last_rotation.elapsed() >= Duration::from_secs(duration_seconds) {
                current_display_index = (current_display_index + 1) % config.display.len();
                last_rotation = Instant::now();
            }
        }

        let display_item = config
            .display
            .get(current_display_index)
            .cloned()
            .unwrap_or_else(DisplayItem::default);

        let sent = match display_item.mode {
            DisplayMode::CpuTempCelsius => sensors
                .read_cpu_temperature()
                .map(|temp| device.display_temperature(temp, true, false))
                .unwrap_or(true),

            DisplayMode::CpuTempFahrenheit => sensors
                .read_cpu_temperature()
                .map(|temp| {
                    device.display_temperature(
                        celsius_to_fahrenheit(temp),
                        false,
                        false,
                    )
                })
                .unwrap_or(true),

            DisplayMode::CpuFanSpeed => sensors
                .read_cpu_fan_speed()
                .map(|rpm| device.display_fan_speed(rpm, false))
                .unwrap_or(true),
        };

        if !sent && !device.is_connected() {
            println!("Reconnecting...");
            let _ = device.connect();
        }

        thread::sleep(Duration::from_millis(config.update_interval_ms));
    }

    println!("Daemon stopped.");
    0
}

fn print_help() {
    println!(
        "BOREAS Display Daemon - Linux driver for GAMDIAS BOREAS M2-51D\n\
         \n\
         Usage: boreas [OPTIONS] [CONFIG_FILE]\n\
         \n\
         Arguments:\n\
           CONFIG_FILE          Path to config file (default: /etc/boreas/config.json)\n\
         \n\
         Options:\n\
           -h, --help           Show this help message\n\
           --list-sensors       List available sensors on this system\n\
           --generate-config    Generate a sample configuration file\n\
           --test               Run a quick display test\n\
         \n\
         Examples:\n\
           boreas                           Run with default config\n\
           boreas ./config.json             Run with custom config\n\
           boreas --generate-config         Create sample config.json\n\
           boreas --list-sensors            Show available sensors"
    );
}

fn list_sensors() {
    println!("Available sensors:\n");

    for sensor in SensorReader::list_available_sensors() {
        let value = fs::read_to_string(&sensor.path)
            .ok()
            .and_then(|content| content.trim().parse::<i64>().ok())
            .map(|raw| {
                if sensor.sensor_type == "temperature" {
                    format!("{:.1}°C", raw as f64 / 1000.0)
                } else {
                    format!("{raw} RPM")
                }
            })
            .unwrap_or_else(|| "?".to_owned());

        println!(
            "  [{:<12}] {:<15} {}",
            sensor.sensor_type,
            sensor.name,
            sensor.path.display()
        );
        println!("               Current value: {value}");
    }
}

fn generate_config(path: &str) -> i32 {
    let path = Path::new(path);

    if path.exists() {
        print!("File {} exists. Overwrite? [y/N] ", path.display());
        let _ = io::stdout().flush();

        let mut answer = String::new();
        if io::stdin().read_line(&mut answer).is_err()
            || !answer.trim().eq_ignore_ascii_case("y")
        {
            println!("Cancelled.");
            return 0;
        }
    }

    match BoreasConfig::save_sample(path) {
        Ok(()) => {
            println!("Sample configuration written to: {}", path.display());
            0
        }
        Err(err) => {
            eprintln!("Failed to write sample configuration: {err}");
            1
        }
    }
}

fn run_test() -> i32 {
    println!("BOREAS Display Test\n===================");

    let mut device = BoreasDevice::new();

    if !device.connect() {
        eprintln!("Failed to connect to device.");
        return 1;
    }

    println!("Connected!");
    let _ = device.initialize();

    println!("Test 1: Celsius 42.5°C");
    let _ = device.display_temperature(42.5, true, false);
    thread::sleep(Duration::from_secs(2));

    println!("Test 2: Fahrenheit 98.6°F");
    let _ = device.display_temperature(98.6, false, false);
    thread::sleep(Duration::from_secs(2));

    println!("Test 3: Fan 1234 RPM");
    let _ = device.display_fan_speed(1234, false);
    thread::sleep(Duration::from_secs(2));

    println!("Test 4: Live sensors...");
    let sensors = SensorReader::new(None, None);

    if let Some(temp) = sensors.read_cpu_temperature() {
        println!("  CPU Temp: {temp:.1}°C");
        let _ = device.display_temperature(temp, true, false);
        thread::sleep(Duration::from_secs(2));
    }

    if let Some(fan) = sensors.read_cpu_fan_speed() {
        println!("  CPU Fan: {fan} RPM");
        let _ = device.display_fan_speed(fan, false);
        thread::sleep(Duration::from_secs(2));
    }

    println!("Test complete!");
    0
}
