mod data;
mod configuration;
mod device;
mod protocol;
mod sensor;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

use device::BoreasDevice;
use configuration::ConfigManager;

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Some(first_arg) = args.first() {
        match first_arg.to_lowercase().as_str() {
            "--help" | "-h" => {
                print_help();
                return 0;
            }

            "--list-sensors" => {
                list_sensors();
                return 0;
            }

            "--generate-config" => {
                let path = args
                    .get(1)
                    .map(String::as_str)
                    .unwrap_or("config.json");

                generate_config(path);
                return 0;
            }

            "--test" => {
                return run_test();
            }

            _ => {}
        }
    }

    let requested_config = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/boreas/config.json"));

    let config_file = match find_config_file(requested_config) {
        Some(path) => path,
        None => return 1,
    };

    let config = match ConfigManager::load(&config_file) {
        Ok(config) => {
            println!(
                "Loaded configuration from: {}",
                config_file.display()
            );

            config
        }

        Err(err) => {
            eprintln!("Failed to load configuration: {err}");
            return 1;
        }
    };

    let running = Arc::new(AtomicBool::new(true));

    install_signal_handler(Arc::clone(&running));

    run_daemon(config, running)
}

fn find_config_file(config_file: PathBuf) -> Option<PathBuf> {
    if config_file.exists() {
        return Some(config_file);
    }

    //
    // Equivalent to AppContext.BaseDirectory/config.json
    //
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let local_config = exe_dir.join("config.json");

            if local_config.exists() {
                return Some(local_config);
            }
        }
    }

    eprintln!(
        "Configuration file not found: {}",
        config_file.display()
    );

    eprintln!(
        "Run with --generate-config to create a sample configuration."
    );

    None
}

fn install_signal_handler(running: Arc<AtomicBool>) {
    if let Err(err) = ctrlc::set_handler(move || {
        if running.swap(false, Ordering::SeqCst) {
            println!("Shutdown requested...");
        }
    }) {
        eprintln!("Warning: failed to install signal handler: {err}");
    }
}

fn run_daemon(
    config: BoreasConfig,
    running: Arc<AtomicBool>,
) -> i32 {
    let sensors = sensor::new(
        config.sensors.cpu_temp_path.clone(),
        config.sensors.cpu_fan_path.clone(),
    );

    match sensors.cpu_temp_path() {
        Some(path) => {
            println!("CPU Temp sensor: {}", path.display());
        }

        None => {
            println!("CPU Temp sensor: not found");
        }
    }

    match sensors.cpu_fan_path() {
        Some(path) => {
            println!("CPU Fan sensor: {}", path.display());
        }

        None => {
            println!("CPU Fan sensor: not found");
        }
    }

    let mut device = DeviceManager::new();

    println!("Connecting to BOREAS display...");

    let mut retry_count = 0_u64;

    while running.load(Ordering::SeqCst) {
        if device.connect() {
            break;
        }

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

    if let Err(err) = device.initialize() {
        eprintln!("Failed to initialize device: {err}");
        return 1;
    }

    let mut current_display_index = 0_usize;
    let mut last_rotation = Instant::now();

    println!("Starting display loop...");

    while running.load(Ordering::SeqCst) {
        let result = update_display(
            &config,
            &sensors,
            &mut device,
            &mut current_display_index,
            &mut last_rotation,
        );

        if let Err(err) = result {
            eprintln!("Error: {err}");

            thread::sleep(Duration::from_secs(1));

            if !device.is_connected() {
                println!("Reconnecting...");

                if let Err(err) = device.connect_result() {
                    eprintln!("Reconnect failed: {err}");
                }
            }
        }

        thread::sleep(Duration::from_millis(
            config.update_interval_ms,
        ));
    }

    println!("Daemon stopped.");

    0
}

fn update_display(
    config: &BoreasConfig,
    sensors: &sensor,
    device: &mut device,
    current_display_index: &mut usize,
    last_rotation: &mut Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.display.len() > 1 {
        let display_item = &config.display[*current_display_index];

        if last_rotation.elapsed()
            >= Duration::from_secs(display_item.duration_seconds)
        {
            *current_display_index =
                (*current_display_index + 1) % config.display.len();

            *last_rotation = Instant::now();
        }
    }

    let default_item = DisplayItem::default();

    let display_item = if config.display.is_empty() {
        &default_item
    } else {
        &config.display[*current_display_index]
    };

    match display_item.mode {
        DisplayMode::CpuTempCelsius => {
            if let Some(temp_c) = sensors.read_cpu_temperature() {
                device.display_temperature(temp_c, true)?;
            }
        }

        DisplayMode::CpuTempFahrenheit => {
            if let Some(temp_c) = sensors.read_cpu_temperature() {
                let temp_f =
                    protocol::celsius_to_fahrenheit(temp_c);

                device.display_temperature(temp_f, false)?;
            }
        }

        DisplayMode::CpuFanSpeed => {
            if let Some(rpm) = sensors.read_cpu_fan_speed() {
                device.display_fan_speed(rpm)?;
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"BOREAS Display Daemon - Linux driver for GAMDIAS BOREAS M2-51D

Usage: boreas [OPTIONS] [CONFIG_FILE]

Arguments:
  CONFIG_FILE          Path to config file (default: /etc/boreas/config.json)

Options:
  -h, --help           Show this help message
  --list-sensors       List available sensors on this system
  --generate-config    Generate a sample configuration file
  --test               Run a quick display test

Examples:
  boreas                           Run with default config
  boreas ./config.json             Run with custom config
  boreas --generate-config         Create sample config.json
  boreas --list-sensors            Show available sensors"#
    );
}

fn list_sensors() {
    println!("Available sensors:\n");

    for sensor in sensor::list_available_sensors() {
        let value = read_sensor_display_value(
            &sensor.path,
            &sensor.sensor_type,
        );

        println!(
            "  [{:<12}] {:<15} {}",
            sensor.sensor_type,
            sensor.name,
            sensor.path.display()
        );

        println!(
            "               Current value: {}",
            value
        );
    }
}

fn read_sensor_display_value(
    path: &Path,
    sensor_type: &str,
) -> String {
    let Ok(content) = fs::read_to_string(path) else {
        return "?".to_owned();
    };

    let Ok(raw) = content.trim().parse::<i64>() else {
        return "?".to_owned();
    };

    if sensor_type == "temperature" {
        format!("{:.1}°C", raw as f64 / 1000.0)
    } else {
        format!("{raw} RPM")
    }
}

fn generate_config(path: &str) {
    let path = Path::new(path);

    if path.exists() {
        print!(
            "File {} exists. Overwrite? [y/N] ",
            path.display()
        );

        if let Err(err) = io::stdout().flush() {
            eprintln!("Failed to flush stdout: {err}");
        }

        let mut input = String::new();

        if io::stdin().read_line(&mut input).is_err()
            || input.trim().to_lowercase() != "y"
        {
            println!("Cancelled.");
            return;
        }
    }

    match BoreasConfig::save_sample(path) {
        Ok(()) => {
            println!(
                "Sample configuration written to: {}",
                path.display()
            );
        }

        Err(err) => {
            eprintln!("Failed to create configuration: {err}");
        }
    }
}

fn run_test() -> i32 {
    println!(
        "BOREAS Display Test\n\
         ==================="
    );

    let mut device = device::new();

    if !device.connect() {
        eprintln!("Failed to connect to device.");
        return 1;
    }

    println!("Connected!");

    if let Err(err) = device.initialize() {
        eprintln!("Failed to initialize device: {err}");
        return 1;
    }

    println!("Test 1: Celsius 42.5°C");

    if let Err(err) = device.display_temperature(42.5, true) {
        eprintln!("Display error: {err}");
        return 1;
    }

    thread::sleep(Duration::from_secs(2));

    println!("Test 2: Fahrenheit 98.6°F");

    if let Err(err) = device.display_temperature(98.6, false) {
        eprintln!("Display error: {err}");
        return 1;
    }

    thread::sleep(Duration::from_secs(2));

    println!("Test 3: Fan 1234 RPM");

    if let Err(err) = device.display_fan_speed(1234) {
        eprintln!("Display error: {err}");
        return 1;
    }

    thread::sleep(Duration::from_secs(2));

    println!("Test 4: Live sensors...");

    let sensors = sensor::new(None, None);

    if let Some(temp) = sensors.read_cpu_temperature() {
        println!("  CPU Temp: {:.1}°C", temp);

        if let Err(err) = device.display_temperature(temp, true) {
            eprintln!("Display error: {err}");
        }

        thread::sleep(Duration::from_secs(2));
    }

    if let Some(fan) = sensors.read_cpu_fan_speed() {
        println!("  CPU Fan: {fan} RPM");

        if let Err(err) = device.display_fan_speed(fan) {
            eprintln!("Display error: {err}");
        }

        thread::sleep(Duration::from_secs(2));
    }

    println!("Test complete!");

    0
}