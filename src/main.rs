use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn read_lines_simple<P>(filename: P) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let content = fs::read_to_string(filename)?;
    
    // Split the content by lines and convert each line to an owned String
    let lines = content.lines().map(|line| line.to_string()).collect();
    
    Ok(lines)
}

fn find_chip_hwmon_path(lines: &[String]) -> Option<PathBuf> {
    // Read the main hwmon class directory
    let paths = fs::read_dir("/sys/class/hwmon").ok()?;
    let pattern = "hwmon";

    for entry in paths.flatten() {
        let path = entry.path();
        let name_path = path.join("name");

        // Read the "name" file inside the hwmon folder to check the driver
        if let Ok(name) = fs::read_to_string(name_path) {
            let search_name = name.trim();
            if lines.contains(&String::from(search_name)) {
                return Some(path);
            }
        }
    }
    None
}

fn read_fan1_speed(lines: &[String]) -> Result<u32, std::io::Error> {
    // Find the correct hwmon directory dynamically
    let hwmon_dir = find_chip_hwmon_path(&lines)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no chip driver found in hwmon"))?;

    // Construct the path to fan1_input
    let fan_path = hwmon_dir.join("fan1_input");

    // Read and parse the raw RPM string
    let raw_speed = fs::read_to_string(fan_path)?;
    let rpm: u32 = raw_speed
        .trim()
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(rpm)
}

fn main() {
    match read_lines_simple("chips") {
        Ok(lines) => {
            match read_fan1_speed(&lines) {
                Ok(rpm) => println!("Fan 1 Speed: {} RPM", rpm),
                Err(e) => eprintln!("Failed to read fan speed: {}", e),
            }
        }
        Err(e) => eprintln!("Error reading file: {}", e),
    }
}
