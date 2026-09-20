use fdt::Fdt;
use crate::serial_println;

pub fn init(dtb_ptr: *const u8) {
    if dtb_ptr.is_null() {
        serial_println!("DTB pointer is null. Skipping FDT parsing.");
        return;
    }

    let fdt = match unsafe { Fdt::from_ptr(dtb_ptr) } {
        Ok(fdt) => fdt,
        Err(e) => {
            serial_println!("Failed to parse FDT: {:?}", e);
            return;
        }
    };

    serial_println!("FDT loaded successfully. Total size: {} bytes", fdt.total_size());

    // Search for VirtIO MMIO devices
    let mut count = 0;
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            if compatible.all().any(|s| s == "virtio,mmio") {
                if let Some(reg) = node.reg().and_then(|mut r| r.next()) {
                    serial_println!("Found VirtIO MMIO device at 0x{:X}, size 0x{:X}", 
                        reg.starting_address as usize, reg.size.unwrap_or(0));
                    
                    crate::hal::dev::virtio::init_mmio(reg.starting_address as usize);
                    count += 1;
                }
            }
        }
    }
    serial_println!("Total VirtIO devices found: {}", count);
}
