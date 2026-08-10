mod Protocol {
    const HEADER_1: u8 = 0x3A;
    const HEADER_2: u8 = 0xB5;
    const COMMAND_DISPLAY: u8 = 0x01;
    const COMMAND_INIT: u8 = 0x20;
    const MODE_TEMPERATURE: u8 = 0x00;
    const MODE_FAN: u8 = 0x01;
    const UNIT_CELSIUS: u8 = 0x01;
    const UNIT_FAHRENHEIT: u8 = 0x00;
    const BLANK_DIGIT: u8 = 0x20;

    pub const PACKET_SIZE: usize = 64;

    pub fn build_temperature_packet(temperature: f64, celsius: bool, flashing: bool) -> [u8; PACKET_SIZE] {
        let value = ((temperature * 10.0) as i32).clamp(0, 9999);
        let (d1, d2, d3, d4) = extract_digits(value, 2);
        build_packet(d1, d2, d3, d4, true, if celsius { UNIT_CELSIUS } else { UNIT_FAHRENHEIT }, true, MODE_TEMPERATURE, flashing)
    }

    pub fn build_fan_packet(rpm: i32, flashing: bool) -> [u8; PACKET_SIZE] {
        let value = rpm.clamp(0, 9999);
        let (d1, d2, d3, d4) = extract_digits(value, 3);
        build_packet(d1, d2, d3, d4, false, 0x00, false, MODE_FAN, flashing)
    }

    pub fn build_init_packet() -> [u8; PACKET_SIZE] {
        let mut packet = [0u8; PACKET_SIZE];
        packet[0] = HEADER_1;
        packet[1] = HEADER_2;
        packet[2] = COMMAND_INIT;
        packet[12] = calculate_checksum(&packet);
        packet
    }

    pub fn celsius_to_fahrenheit(celsius: f64) -> f64 {
        celsius * 9.0 / 5.0 + 32.0
    }

    fn build_packet(
        d1: u8, d2: u8, d3: u8, d4: u8,
        has_decimal: bool, unit: u8, show_cpu_icon: bool, mode: u8, flashing: bool
    ) -> [u8; PACKET_SIZE] {
        let mut packet = [0u8; PACKET_SIZE];
        packet[0] = HEADER_1;
        packet[1] = HEADER_2;
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
        packet[12] = calculate_checksum(&packet);
        packet
    }

    fn extract_digits(value: i32, blank_leading_zeros: i32) -> (u8, u8, u8, u8) {
        let d1 = (value / 1000) % 10;
        let d2 = (value / 100) % 10;
        let d3 = (value / 10) % 10;
        let d4 = value % 10;

        let b1 = if blank_leading_zeros >= 4 && d1 == 0 { BLANK_DIGIT } else { d1 as u8 };
        let b2 = if blank_leading_zeros >= 3 && d1 == 0 && d2 == 0 { BLANK_DIGIT } else { d2 as u8 };
        let b3 = if blank_leading_zeros >= 2 && d1 == 0 && d2 == 0 && d3 == 0 { BLANK_DIGIT } else { d3 as u8 };
        let b4 = d4 as u8;

        (b1, b2, b3, b4)
    }

    fn calculate_checksum(packet: &[u8; PACKET_SIZE]) -> u8 {
        packet[0..12].iter().fold(0u8, |acc, &x| acc.wrapping_add(x))
    }
}