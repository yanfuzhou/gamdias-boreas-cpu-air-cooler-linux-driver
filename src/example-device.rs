use rusb::{Context, UsbContext};
use std::time::Duration;

fn main() -> rusb::Result<()> {
    // Replace with your device's actual Vendor ID and Product ID
    let vid: u16 = 0x1b80; 
    let pid: u16 = 0xb554;

    // 1. Initialize the USB context
    let context = Context::new()?;

    // 2. Open device by VID and PID
    let mut handle = context.open_device_with_vid_pid(vid, pid)
        .ok_or_else(|| rusb::Error::NoDevice)?;

    // 3. Detach kernel driver (required on Linux if a driver is active)
    let interface_num = 0;
    if handle.kernel_active_in_interface(interface_num).unwrap_or(false) {
        handle.detach_kernel_driver(interface_num)?;
    }

    // 4. Claim the interface
    handle.claim_interface(interface_num)?;

    // 5. Send data via bulk transfer (replace endpoint address with your device's OUT endpoint)
    let data_to_send = b"Hello USB Device";
    let endpoint_out = 0x01; 
    let timeout = Duration::from_secs(1);

    let bytes_written = handle.write_bulk(endpoint_out, data_to_send, timeout)?;
    println!("Successfully sent {} bytes.", bytes_written);

    // 6. Clean up by releasing the interface
    handle.release_interface(interface_num)?;

    Ok(())
}
