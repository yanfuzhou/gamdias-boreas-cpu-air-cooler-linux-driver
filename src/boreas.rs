mod cpu_fan_data;
mod configuration;

use std::path::PathBuf;
use std::time::Duration;
use std::fs::read_to_string;
use std::io::{Error, ErrorKind};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use configuration::{get_run_parameters, Mode};

fn main() {
    let (update_interval_ms, boreas_display, temp_path, fan_path) = get_run_parameters();

    println!("update_interval_ms: {:?}", update_interval_ms);
    println!("display: {:?}", boreas_display);

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
