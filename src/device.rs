// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 BOREAS Linux Project Contributors

use chrono::Local;
use hidapi::{
    HidApi, 
    HidDevice
};
use notify_rust::Notification;

use crate::data::PRODUCTS;
use crate::protocol::boreas_protocol::{
    build_init_packet, 
    build_fan_packet, 
    build_temperature_packet
};

pub struct BoreasDevice {
    vendor_id: u16,
    product_id: u16,
    device: Option<HidDevice>
}

impl BoreasDevice {
    pub fn new() -> Self {
        Self {
            vendor_id: 0,
            product_id: 0,
            device: None
        }
    }

    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    pub fn connect(&mut self) -> bool {
        if self.device.is_some() {
            return true;
        }

        let api = match HidApi::new() {
            Ok(api) => api,
            Err(_) => return false,
        };

        for device_info in api.device_list() {
            if device_info.vendor_id() == 0x1b80 
                && PRODUCTS.contains(&device_info.product_id()) 
                && device_info.interface_number() == 0 {
                    // Attempt to open the device using its path
                    if let Ok(dev) = device_info.open_device(&api) {
                        self.vendor_id = device_info.vendor_id();
                        self.product_id = device_info.product_id();
                        self.device = Some(dev);
                        return true;
                    }
            }
        }

        false
    }

    pub fn disconnect(&mut self) {
        self.device = None;
    }

    pub fn send_packet(&self, packet: &[u8; 64]) -> bool {
        let Some(ref dev) = self.device else { return false; };
        dev.write(packet).is_ok()
    }

    pub fn initialize(&self) -> bool {
        self.send_packet(&build_init_packet())
    }

    pub fn display_temperature(&self, temperature: f64, celsius: bool, flashing: bool) -> bool {
        let mut triggered: bool = false;
        if !triggered {
            trigger_alarm(temperature, celsius, &mut triggered);
        }
        self.send_packet(&build_temperature_packet(temperature, celsius, flashing))
    }

    pub fn display_fan_speed(&self, rpm: i32, flashing: bool) -> bool {
        self.send_packet(&build_fan_packet(rpm, flashing))
    }
}

impl Drop for BoreasDevice {
    fn drop(&mut self) {
        self.disconnect();
    }
}

fn trigger_alarm(temperature: f64, celsius: bool, triggered: &mut bool) {
    if celsius {
        if temperature > 95.0 { 
            spawn_notification(
                "CPU Temperature Alarm Triggered!", 
                &format!("The CPU Temperature is above 95.0°C at {}.", Local::now()), 
                "dialog-warning", 
                0
            );
            *triggered = true;
        } else {
            *triggered = false;
        }
    } else {
        if temperature > 203.0 {
            spawn_notification(
                "CPU Temperature Alarm Triggered!", 
                &format!("The CPU Temperature is above 203.0°F at {}.", Local::now()), 
                "dialog-warning", 
                0
            );
            *triggered = true;
        } else {
            *triggered = false;
        }
    }
}

fn spawn_notification(summary: &str, body: &str, icon: &str, timeout: i32) {
    Notification::new()
        .summary(summary)
        .body(body)
        .icon(icon)
        .timeout(timeout)
        .show()
        .expect("Error showing the CPU Temperature Alarm notification.");
}