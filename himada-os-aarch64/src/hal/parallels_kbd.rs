use crate::mm::vmm::phys_to_virt;

const PRL_KBD_BUF_PADDR: usize = 0x33B98;
const PRL_KBD_HEAD_PADDR: usize = 0x33C98;
const PRL_KBD_TAIL_PADDR: usize = 0x33C9C;

static mut SHIFT_PRESSED: bool = false;
static mut CTRL_PRESSED: bool = false;

pub fn read_char() -> Option<u8> {
    unsafe {
        let head_ptr = phys_to_virt(PRL_KBD_HEAD_PADDR) as *mut u32;
        let tail_ptr = phys_to_virt(PRL_KBD_TAIL_PADDR) as *const u32;
        let buf_ptr = phys_to_virt(PRL_KBD_BUF_PADDR) as *const u8;

        let head = core::ptr::read_volatile(head_ptr) as usize & 0xFF;
        let tail = core::ptr::read_volatile(tail_ptr) as usize & 0xFF;

        if head == tail {
            return None;
        }

        let scancode = core::ptr::read_volatile(buf_ptr.add(head));
        core::ptr::write_volatile(head_ptr, ((head + 1) & 0xFF) as u32);

        scancode_to_ascii(scancode)
    }
}

fn scancode_to_ascii(scancode: u8) -> Option<u8> {
    unsafe {
        if scancode == 0x2A || scancode == 0x36 {
            SHIFT_PRESSED = true;
            return None;
        }
        if scancode == 0xAA || scancode == 0xB6 {
            SHIFT_PRESSED = false;
            return None;
        }
        if scancode == 0x1D {
            CTRL_PRESSED = true;
            return None;
        }
        if scancode == 0x9D {
            CTRL_PRESSED = false;
            return None;
        }
        if (scancode & 0x80) != 0 {
            return None; // Key release
        }

        let shift = SHIFT_PRESSED;
        let ctrl = CTRL_PRESSED;

        let c: u8 = match scancode {
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
            0x0E => 0x08, // Backspace
            0x0F => b'\t', // Tab
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
            0x26 => if ctrl { 0x0C } else if shift { b'L' } else { b'l' }, // Ctrl+L
            0x27 => if shift { b':' } else { b';' },
            0x28 => if shift { b'"' } else { b'\'' },
            0x29 => if shift { b'~' } else { b'`' },
            0x2B => if shift { b'|' } else { b'\\' },
            0x2C => if ctrl { 0x1A } else if shift { b'Z' } else { b'z' },
            0x2D => if ctrl { 0x18 } else if shift { b'X' } else { b'x' },
            0x2E => if ctrl { 0x03 } else if shift { b'C' } else { b'c' }, // Ctrl+C
            0x2F => if ctrl { 0x16 } else if shift { b'V' } else { b'v' },
            0x30 => if ctrl { 0x02 } else if shift { b'B' } else { b'b' },
            0x31 => if ctrl { 0x0E } else if shift { b'N' } else { b'n' },
            0x32 => if ctrl { 0x0D } else if shift { b'M' } else { b'm' },
            0x33 => if shift { b'<' } else { b',' },
            0x34 => if shift { b'>' } else { b'.' },
            0x35 => if shift { b'?' } else { b'/' },
            0x39 => b' ',
            _ => return None,
        };
        Some(c)
    }
}
