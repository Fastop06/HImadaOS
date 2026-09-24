use core::fmt;

pub static mut UART_BASE: usize = 0;
pub static mut UART_MAPPED: bool = false;

pub fn init(fdt_vaddr: usize) {
    if fdt_vaddr != 0 {
        let vaddr = fdt_vaddr as *const u8;
        if let Ok(fdt) = unsafe { fdt::Fdt::from_ptr(vaddr) } {
            for node in fdt.all_nodes() {
                if let Some(compat) = node.property("compatible") {
                    if let Ok(compat_str) = core::str::from_utf8(compat.value) {
                        if compat_str.contains("arm,pl011") {
                            if let Some(reg) = node.property("reg") {
                                if reg.value.len() >= 8 {
                                    let base = u64::from_be_bytes(reg.value[0..8].try_into().unwrap()) as usize;
                                    unsafe {
                                        crate::mm::vmm::map_device_page(base, base);
                                        UART_BASE = base;
                                        UART_MAPPED = true;
                                    }
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check if ACPI SPCR table found a serial console
    unsafe {
        if let Some(base) = crate::hal::acpi::ACPI_INFO.spcr_base {
            crate::mm::vmm::map_device_page(base, base);
            UART_BASE = base;
            UART_MAPPED = true;
            return;
        }
    }
}

pub fn has_byte() -> bool {
    unsafe {
        if !UART_MAPPED || UART_BASE == 0 { return false; }
        let uart_fr = (UART_BASE + 0x18) as *const u32;
        (core::ptr::read_volatile(uart_fr) & 0x10) == 0
    }
}

pub fn read_byte() -> Option<u8> {
    unsafe {
        if !UART_MAPPED || UART_BASE == 0 { return None; }
        let uart_fr = (UART_BASE + 0x18) as *const u32;
        let uart_dr = UART_BASE as *const u32;
        // FR bit 4 is RXFE (Receive FIFO Empty)
        if (core::ptr::read_volatile(uart_fr) & 0x10) == 0 {
            let ch = (core::ptr::read_volatile(uart_dr) & 0xFF) as u8;
            Some(ch)
        } else {
            None
        }
    }
}

pub fn write_byte(ch: u8) {
    unsafe {
        if !UART_MAPPED || UART_BASE == 0 { return; }
        let uart_fr = (UART_BASE + 0x18) as *const u32;
        let uart_dr = UART_BASE as *mut u32;
        // FR bit 5 is TXFF (Transmit FIFO Full)
        let mut timeout = 200;
        while (core::ptr::read_volatile(uart_fr) & 0x20) != 0 && timeout > 0 {
            timeout -= 1;
        }
        if timeout == 0 {
            // UART FIFO full and unserviced (e.g. Parallels without serial port attached).
            // Disable mapping so subsequent writes do not spin.
            UART_MAPPED = false;
            return;
        }
        core::ptr::write_volatile(uart_dr, ch as u32);
    }
}

pub struct SerialPort;

pub static SERIAL_LOCK: spin::Mutex<SerialPort> = spin::Mutex::new(SerialPort);

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            write_byte(b);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            if let Some(mut port) = $crate::hal::serial::SERIAL_LOCK.try_lock() {
                let _ = write!(port, $($arg)*);
            } else {
                let mut direct = $crate::hal::serial::SerialPort;
                let _ = write!(direct, $($arg)*);
            }
        }
    };
}

#[macro_export]
macro_rules! serial_println {
    () => { $crate::serial_print!("\n") };
    ($($arg:tt)*) => {
        {
            $crate::serial_print!($($arg)*);
            $crate::serial_print!("\n")
        }
    };
}

