extern crate hidapi;

mod data;
mod sensors;
mod protocol;
mod device;

use std::thread;
use std::time::Duration;

use chrono::Utc;

use sensors::{get_run_parameters, read_temp_rpm, Mode};
use device::{get_device, init_device, display_cpu_temp, display_cpu_fan};

// Todo: make main() function take argument from console, move device, config to class if needed
fn main() {

    let device = get_device().expect(" Failed to find the device. Make sure the device is connected and accessible.");
    init_device(&device);

    let (update_interval_ms, boreas_display, temp_path, fan_path) = get_run_parameters();

    // Your conditional variable for the while loop
    let running = true;
    let mut current_display_index = 0;
    let mut last_rotation = Utc::now();
    let display_count = boreas_display.len();

    println!("Starting display loop...");

    while running {

        if display_count > 1 {
            let elapsed = Utc::now() - last_rotation;
            if elapsed.num_seconds() as u32 >= boreas_display[current_display_index].duration_seconds {
                current_display_index = (current_display_index + 1) % display_count;
                last_rotation = Utc::now();
            }
        }

        let display_item = if display_count > 0 {
            &boreas_display[current_display_index]
        } else {
            &Mode {
                mode: "CpuTempCelsius".to_string(), 
                duration_seconds: 5
            }
        };

        let display_mode = display_item.mode.as_str();

        // 2. Perform your background loop tasks here
        let (temp, rpm) = read_temp_rpm(&temp_path, &fan_path);

        match display_mode {
            "CpuTempCelsius" => display_cpu_temp(&device, temp, true),
            "CpuTempFahrenheit" => display_cpu_temp(&device, temp, false),
            "CpuFanSpeed" => display_cpu_fan(&device, rpm),
            _ => eprintln!("Unknown display mode"), // The mandatory default case
        }

        thread::sleep(Duration::from_millis(update_interval_ms as u64));
    }

    println!("Daemon stopped.");
}
