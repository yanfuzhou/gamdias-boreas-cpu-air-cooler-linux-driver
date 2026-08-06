mod data;
mod config;
mod protocol;

use std::thread;
use std::path::PathBuf;
use std::time::Duration;
use std::io::{self, Write};
use std::fs::read_to_string;

use chrono::Utc;
use rusb::{Context, DeviceHandle, UsbContext};

use config::get_run_parameters;
use protocol::{build_init_packet, build_temperature_packet, build_fan_packet};

use crate::config::Mode;

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

fn get_device_handle() -> DeviceHandle<Context> {
    // Search device by vender id and product id
    let device = match get_device() {
        Some(v) => v,
        None => {
            println!("Device not found. Exiting now.");
            std::process::exit(0);
        }
    };

    // connect to display
    let vid = device.vendor_id();
    let pid = device.product_id();
    println!("ID {:04x}:{:04x}", vid, pid);

    // 1. Initialize the USB context
    let context = Context::new().expect("Can't initialize the device with deamon.");

    // 2. Open device by VID and PID
    let handle = context.open_device_with_vid_pid(vid, pid)
        .ok_or_else(|| rusb::Error::NoDevice).expect("Can't open the device with deamon.");

    // 3. Detach kernel driver (required on Linux if a driver is active)
    let interface_num = 0;
    if handle.kernel_driver_active(interface_num).unwrap_or(false) {
        handle.detach_kernel_driver(interface_num).expect("Can't detach the kernel driver. Please make sure the driver is inactive.");
    }

    // 4. Claim the interface
    handle.claim_interface(interface_num).expect("Can't claim the device interface.");

    handle
}

fn send_packet(handle: DeviceHandle<Context>, packet: &[u8; 64]) {
    let endpoint_out = 0x01; 
    let timeout = Duration::from_secs(1);
    let bytes_written = handle.write_bulk(endpoint_out, packet, timeout).expect("Unknown error while write byte array buffer to the device.");
    println!("Successfully sent {} bytes.", bytes_written);
}

fn init_device(handle: DeviceHandle<Context>) {
    let packet = build_init_packet();
    send_packet(handle, &packet);
}

fn display_cpu_temp(temp_c: f32, celsius: bool) {
    if celsius {
        print!("CPU Temperature: {} °C", temp_c);
        build_temperature_packet(temp_c, celsius, false);
    } else {
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;
        print!("CPU Temperature: {} °F", temp_f);
        build_temperature_packet(temp_c, celsius, false);
    }
}

fn display_cpu_fan(rpm: i32) {
    print!("Fan 1 Speed: {} RPM", rpm);
    build_fan_packet(rpm, false);
}

// Todo: make main() function take argument from console, move device, config to class if needed
fn main() -> rusb::Result<()> {

    let handle = get_device_handle();
    init_device(handle);

    let (update_interval_ms, boreas_display, temp_path, fan_path) = get_run_parameters();

    // Your conditional variable for the while loop
    let running = true;
    let mut current_display_index = 0;
    let mut last_rotation = Utc::now();
    let display_count = boreas_display.len();

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
            "CpuTempCelsius" => display_cpu_temp(temp, true),
            "CpuTempFahrenheit" => display_cpu_temp(temp, false),
            "CpuFanSpeed" => display_cpu_fan(rpm),
            _ => print!("Unknown display mode"), // The mandatory default case
        }

        // Rust buffers stdout, so you must flush it manually
        io::stdout().flush().unwrap();

        thread::sleep(Duration::from_millis(update_interval_ms as u64));
    }

    Ok(())
}
