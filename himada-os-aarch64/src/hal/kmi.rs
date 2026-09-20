// ─────────────────────────────────────────────────────────────
// ARM PL050 KMI — PS/2 Keyboard Mouse Interface Driver
// Used by: Parallels Desktop (Apple Silicon), QEMU virt with -device pl050
//
// Typical MMIO addresses on AArch64 virt boards:
//   Parallels:  0x1C06_0000  (keyboard)  0x1C07_0000  (mouse)
//   QEMU virt:  0x0903_0000  (keyboard)  0x0904_0000  (mouse)
// ─────────────────────────────────────────────────────────────

use core::ptr;
use crate::mm::vmm::map_device_page;

// PL050 register offsets
const KMI_CR:     usize = 0x00; // Control
const KMI_STAT:   usize = 0x04; // Status
const KMI_DATA:   usize = 0x08; // Data (TX/RX)
const KMI_CLKDIV: usize = 0x0C; // Clock divisor
const KMI_IR:     usize = 0x10; // Interrupt status

const STAT_RXB: u32 = 1 << 4; // RX buffer full

/// Known PL050 KMI keyboard base addresses to probe
static KMI_BASES: &[usize] = &[
    0x1C06_0000, // Parallels Desktop AArch64 virt
    0x0903_0000, // QEMU virt with pl050
    0x1000_6000, // Some Cortex virt boards
];

static mut KMI_BASE: usize = 0;
static mut KMI_READY: bool = false;
static mut SHIFT: bool = false;
static mut CTRL: bool = false;
static mut EXTENDED: bool = false; // E0 prefix

/// Attempt to detect and initialize the PL050 KMI keyboard controller.
/// Called early in kernel init (after ACPI/MMU setup).
pub fn init() {
    unsafe {
        for &base in KMI_BASES {
            // Map 1 page for this MMIO region
            map_device_page(base, base);
            // Validate: PL050 PrimeCell ID is at +0xFE0..+0xFF0
            let pid0 = ptr::read_volatile((base + 0xFE0) as *const u8);
            let pid1 = ptr::read_volatile((base + 0xFE4) as *const u8);
            let ccid = ptr::read_volatile((base + 0xFF0) as *const u8);
            // PL050: PID0=0x50, PID1=0x10; PrimeCellID: 0x0D
            if (pid0 == 0x50 || pid0 == 0x51) && ccid == 0x0D {
                // Enable KMI: bit 2 = KMIEN, bit 4 = RXINTREN (RX interrupt enable)
                ptr::write_volatile((base + KMI_CR) as *mut u32, (1 << 2) | (1 << 4));
                // Set clock divisor for PS/2 (approx 8MHz / 2 = 4MHz)
                ptr::write_volatile((base + KMI_CLKDIV) as *mut u32, 0);
                KMI_BASE = base;
                KMI_READY = true;
                crate::serial_println!("[KMI] PL050 keyboard at {:#X} (PID0={:#X})", base, pid0);
                return;
            }
        }
        // Fallback: try probing by checking STAT register sanity
        for &base in KMI_BASES {
            map_device_page(base, base);
            let stat = ptr::read_volatile((base + KMI_STAT) as *const u32);
            // Sane status: not 0x00 and not 0xFFFF_FFFF
            if stat != 0 && stat != 0xFFFF_FFFF && stat < 0x100 {
                ptr::write_volatile((base + KMI_CR) as *mut u32, (1 << 2) | (1 << 4));
                ptr::write_volatile((base + KMI_CLKDIV) as *mut u32, 0);
                KMI_BASE = base;
                KMI_READY = true;
                crate::serial_println!("[KMI] PL050 fallback at {:#X} (STAT={:#X})", base, stat);
                return;
            }
        }
        crate::serial_println!("[KMI] No PL050 keyboard found");
    }
}

/// Poll for a ready keypress. Returns ASCII char or None.
pub fn poll_keyboard() -> Option<u8> {
    unsafe {
        if !KMI_READY || KMI_BASE == 0 { return None; }
        let stat = ptr::read_volatile((KMI_BASE + KMI_STAT) as *const u32);
        if (stat & STAT_RXB) == 0 { return None; }
        let scancode = ptr::read_volatile((KMI_BASE + KMI_DATA) as *const u32) as u8;
        ps2_scancode_to_ascii(scancode)
    }
}

/// PS/2 Set 1 scancode decoder (make codes only; break codes = make | 0x80)
fn ps2_scancode_to_ascii(sc: u8) -> Option<u8> {
    unsafe {
        // Extended key prefix — consume it
        if sc == 0xE0 { EXTENDED = true; return None; }
        let extended = EXTENDED;
        EXTENDED = false;

        // Modifier keys
        match sc {
            0x2A | 0x36          => { SHIFT = true;  return None; } // L/R Shift make
            0xAA | 0xB6          => { SHIFT = false; return None; } // L/R Shift break
            0x1D                 => { CTRL = true;   return None; } // Ctrl make
            0x9D                 => { CTRL = false;  return None; } // Ctrl break
            0x38                 => { return None; }                  // Alt (ignore)
            _ if sc & 0x80 != 0  => { return None; }                 // Any break code
            _ => {}
        }

        // Extended keys (after E0)
        if extended {
            return match sc {
                0x1C => Some(b'\n'), // Numpad Enter
                0x35 => Some(b'/'),  // Numpad /
                _    => None,
            };
        }

        let shift = SHIFT;
        let ctrl  = CTRL;

        let ch: u8 = match sc {
            0x01 => 0x1B, // ESC
            0x02 => if shift { b'!' } else { b'1' },
            0x03 => if shift { b'@' } else { b'2' },
            0x04 => if shift { b'#' } else { b'3' },
            0x05 => if shift { b'$' } else { b'4' },
            0x06 => if shift { b'%' } else { b'5' },
            0x07 => if shift { b'^' } else { b'6' },
            0x08 => if shift { b'&' } else { b'7' },
            0x09 => if shift { b'*' } else { b'8' },
            0x0A => if shift { b'(' } else { b'9' },
            0x0B => if shift { b')' } else { b'0' },
            0x0C => if shift { b'_' } else { b'-' },
            0x0D => if shift { b'+' } else { b'=' },
            0x0E => 0x7F, // Backspace → DEL
            0x0F => b'\t',
            0x10 => if ctrl { 0x11 } else if shift { b'Q' } else { b'q' },
            0x11 => if ctrl { 0x17 } else if shift { b'W' } else { b'w' },
            0x12 => if ctrl { 0x05 } else if shift { b'E' } else { b'e' },
            0x13 => if ctrl { 0x12 } else if shift { b'R' } else { b'r' },
            0x14 => if ctrl { 0x14 } else if shift { b'T' } else { b't' },
            0x15 => if ctrl { 0x19 } else if shift { b'Y' } else { b'y' },
            0x16 => if ctrl { 0x15 } else if shift { b'U' } else { b'u' },
            0x17 => if ctrl { 0x09 } else if shift { b'I' } else { b'i' },
            0x18 => if ctrl { 0x0F } else if shift { b'O' } else { b'o' },
            0x19 => if ctrl { 0x10 } else if shift { b'P' } else { b'p' },
            0x1A => if shift { b'{' } else { b'[' },
            0x1B => if shift { b'}' } else { b']' },
            0x1C => b'\n', // Enter
            0x1E => if ctrl { 0x01 } else if shift { b'A' } else { b'a' },
            0x1F => if ctrl { 0x13 } else if shift { b'S' } else { b's' },
            0x20 => if ctrl { 0x04 } else if shift { b'D' } else { b'd' },
            0x21 => if ctrl { 0x06 } else if shift { b'F' } else { b'f' },
            0x22 => if ctrl { 0x07 } else if shift { b'G' } else { b'g' },
            0x23 => if ctrl { 0x08 } else if shift { b'H' } else { b'h' },
            0x24 => if ctrl { 0x0A } else if shift { b'J' } else { b'j' },
            0x25 => if ctrl { 0x0B } else if shift { b'K' } else { b'k' },
            0x26 => if ctrl { 0x0C } else if shift { b'L' } else { b'l' },
            0x27 => if shift { b':' } else { b';' },
            0x28 => if shift { b'"' } else { b'\'' },
            0x29 => if shift { b'~' } else { b'`' },
            0x2B => if shift { b'|' } else { b'\\' },
            0x2C => if ctrl { 0x1A } else if shift { b'Z' } else { b'z' },
            0x2D => if ctrl { 0x18 } else if shift { b'X' } else { b'x' },
            0x2E => if ctrl { 0x03 } else if shift { b'C' } else { b'c' },
            0x2F => if ctrl { 0x16 } else if shift { b'V' } else { b'v' },
            0x30 => if ctrl { 0x02 } else if shift { b'B' } else { b'b' },
            0x31 => if ctrl { 0x0E } else if shift { b'N' } else { b'n' },
            0x32 => if ctrl { 0x0D } else if shift { b'M' } else { b'm' },
            0x33 => if shift { b'<' } else { b',' },
            0x34 => if shift { b'>' } else { b'.' },
            0x35 => if shift { b'?' } else { b'/' },
            0x39 => b' ',
            // Numpad digits
            0x47 => b'7', 0x48 => b'8', 0x49 => b'9',
            0x4B => b'4', 0x4C => b'5', 0x4D => b'6',
            0x4F => b'1', 0x50 => b'2', 0x51 => b'3',
            0x52 => b'0', 0x53 => b'.', // Numpad 0, .
            _ => return None,
        };
        Some(ch)
    }
}
