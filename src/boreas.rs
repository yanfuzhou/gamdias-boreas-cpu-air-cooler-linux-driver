mod data;
mod config;

use std::thread;
use std::path::PathBuf;
use std::time::Duration;
use std::fs::read_to_string;

use config::get_run_parameters;

fn read_temp_rpm(temp_path: &PathBuf, fan_path: &PathBuf) -> (f32, i32) {
        // Read and parse the raw temperature string
        let raw_temp = read_to_string(temp_path).unwrap_or(String::from("-1"));
        let milli_celsius: i32 = raw_temp.trim().parse().unwrap_or(-1);
        let temp = milli_celsius as f32 / 1000.0;

        // Read and parse the raw RPM string
        let raw_speed = read_to_string(fan_path).unwrap_or(String::from("-1"));
        let rpm: i32 = raw_speed.trim().parse().unwrap_or(-1);

        (temp, rpm)
}

fn get_device() -> Option<rusb::DeviceDescriptor> {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();

        if device_desc.vendor_id() == 0x1b80 {
            return Some(device_desc);
        }
    }
    None
}

// Todo: make main() function take argument from console
fn main() {
    // Search device by vender id and product id
    let device = get_device().expect("Device not found");
    let vendor_id = device.vendor_id();
    let product_id = device.product_id();
    println!("ID {:04x}:{:04x}", vendor_id, product_id);

    let (update_interval_ms, boreas_display, temp_path, fan_path) = get_run_parameters();

    for mode in boreas_display.iter() {
        println!("Mode: {}, Duration: {} seconds", mode.mode, mode.duration_seconds);
    }

    // Your conditional variable for the while loop
    let running = true;

    while running {
        // 2. Perform your background loop tasks here
        let (temp, rpm) = read_temp_rpm(&temp_path, &fan_path);

        println!("CPU Temperature: {} °C", temp);
        println!("Fan 1 Speed: {} RPM", rpm);

        thread::sleep(Duration::from_millis(update_interval_ms as u64));

        // Todo: 
        // connect to display
        // create protocol packet (can be moved to top)
        // send packet to display
    }
}
