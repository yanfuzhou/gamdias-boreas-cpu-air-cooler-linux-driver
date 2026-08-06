const HEADER1: u8 = 0x3A;
const HEADER2: u8 = 0xB5;
const COMMAND_DISPLAY: u8 = 0x01;
const COMMAND_INIT: u8 = 0x20;
const MODE_TEMPERATURE: u8 = 0x00;
const MODE_FAN: u8 = 0x01;
const UNIT_CELSIUS: u8 = 0x01;
const UNIT_FAHRENHEIT: u8 = 0x00;
const BLANK_DIGIT: u8 = 0x20;

const PACKET_SIZE: usize = 64;

fn calculate_checksum(packet: [u8; 64]) -> u8 {
    let mut sum = 0;
    for i in 1..12 {
        sum += packet[i]
    };
    sum
}

fn extract_digits(value: i32, blank_leading_zeros: i32) -> (u8, u8, u8, u8) {
    let mut d1 = (value / 1000) as u8;
    let mut d2 = ((value / 100) % 10) as u8;
    let mut d3 = ((value / 10) % 10) as u8;
    let d4 = (value % 10) as u8;

    if blank_leading_zeros >= 1 && d1 == 0 {
        d1 = BLANK_DIGIT;
        if blank_leading_zeros >= 2 && d2 == 0 {
            d2 = BLANK_DIGIT;
            if blank_leading_zeros >= 3 && d3 == 0 {
                d3 = BLANK_DIGIT;
            }
        }
    }

    (d1, d2, d3, d4)
}

fn build_packet(d1: u8, d2: u8, d3: u8, d4: u8, has_decimal: bool, unit: u8, show_cpu_icon: bool, mode: u8, flashing: bool) -> [u8; 64] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = HEADER1;
    packet[1] = HEADER2;
    packet[2] = COMMAND_DISPLAY;
    packet[3] = d1;
    packet[4] = d2;
    packet[5] = d3;
    packet[6] = d4;
    packet[7] = if has_decimal { 0x01 } else { 0x00 };
    packet[8] = unit;
    packet[9] = if show_cpu_icon { 0x01 } else { 0x00 };
    packet[10] = mode;
    packet[11] = if flashing { 0x01 } else { 0x00 };
    packet[12] = calculate_checksum(packet);
    packet
}

pub fn build_init_packet() -> [u8; 64] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = HEADER1;
    packet[1] = HEADER2;
    packet[2] = COMMAND_INIT;
    packet[12] = calculate_checksum(packet);
    packet
}

pub fn build_temperature_packet(temp: f32, celsius: bool, flashing: bool) -> [u8; 64] {
    let value = ((temp * 10.0) as i32).clamp(0, 9999);
    let (d1, d2, d3, d4) = extract_digits(value, 2);
    let packet = build_packet(d1, d2, d3, d4, true, if celsius { UNIT_CELSIUS } else { UNIT_FAHRENHEIT }, true, MODE_TEMPERATURE, flashing);
    packet
}

pub fn build_fan_packet(rpm: i32, flashing: bool) -> [u8; 64] {
    let value = rpm.clamp(0, 9999);
    let (d1, d2, d3, d4) = extract_digits(value, 3);
    let packet = build_packet(d1, d2, d3, d4, false, 0x00, false, MODE_FAN, flashing);
    packet
}