use hidapi::HidApi;

use crate::data::PRODUCTS;
use crate::sensors::VERBOSE;
use crate::protocol::{build_init_packet, build_temperature_packet, build_fan_packet};

fn show_verbose_info(msg: String) {
    if VERBOSE {
        println!("{}", msg);
    }
}

fn send_packet(device: &hidapi::HidDevice, packet: &[u8; 64]) {
    let bytes_written = device.write(packet).expect("Failed to write packet to device");
    show_verbose_info(format!("Wrote {} bytes successfully", bytes_written));
}


pub fn get_device() -> Option<hidapi::HidDevice> {
    match HidApi::new() {
        Ok(api) => {
            for device in api.device_list() {
                if device.vendor_id() == 0x1b80 {
                    show_verbose_info(format!(
                        "Found device: {:04x}:{:04x}:{}", 
                        device.vendor_id(), 
                        device.product_id(), 
                        device.product_string().unwrap_or("Unknown")
                    ));
                    if PRODUCTS.contains(&device.product_id()) {
                        // Try to open the device and return the handle
                        if let Ok(handle) = api.open(device.vendor_id(), device.product_id()) {
                            println!("Successfully connected to device: {}", device.product_string().unwrap_or("Unknown"));
                            return Some(handle);
                        } else {
                            eprintln!(
                                "Failed to open HID device: {:04x}:{:04x}:{}",
                                device.vendor_id(),
                                device.product_id(),
                                device.product_string().unwrap_or("Unknown")
                            );
                            return None;
                        }
                    } else {
                        eprintln!(
                            "Device not supported: {:04x}:{:04x}:{}",
                            device.vendor_id(),
                            device.product_id(),
                            device.product_string().unwrap_or("Unknown")
                        );
                        return None;
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
        },
    }
    None
}

pub fn init_device(device: &hidapi::HidDevice) {
    let packet = build_init_packet();
    send_packet(device, &packet);
}

pub fn display_cpu_temp(device: &hidapi::HidDevice, temp_c: f32, celsius: bool) {
    if celsius {
        show_verbose_info(format!("Sending temperature packet: {} °C", temp_c));
        let packet = build_temperature_packet(temp_c, celsius, false);
        send_packet(device, &packet);
    } else {
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;
        show_verbose_info(format!("Sending temperature packet: {} °F", temp_f));
        let packet = build_temperature_packet(temp_f, celsius, false);
        send_packet(device, &packet);
    }
}

pub fn display_cpu_fan(device: &hidapi::HidDevice, rpm: i32) {
    show_verbose_info(format!("Sending fan speed packet: {} RPM", rpm));
    let packet = build_fan_packet(rpm, false);
    send_packet(device, &packet);
}