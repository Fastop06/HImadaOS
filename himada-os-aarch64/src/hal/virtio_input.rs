// ─────────────────────────────────────────────────────────────
// VirtIO Input Driver (Device ID 18) — Keyboard events
// Parallels Desktop for Apple Silicon uses virtio-input to
// deliver keyboard events to AArch64 guest VMs.
//
// Protocol: virtio-input presents Linux evdev events:
//   struct virtio_input_event { type: u16, code: u16, value: u32 }
// EV_KEY (type=1), value=1 (press), code = Linux keycode
// ─────────────────────────────────────────────────────────────

use core::ptr;
use crate::mm::vmm::{phys_to_virt, map_device_page};
use crate::mm::pmm::alloc_frames;

// VirtIO MMIO register offsets
const VIRTIO_MMIO_MAGIC:          usize = 0x000;
const VIRTIO_MMIO_VERSION:        usize = 0x004;
const VIRTIO_MMIO_DEVICE_ID:      usize = 0x008;
const VIRTIO_MMIO_VENDOR_ID:      usize = 0x00C;
const VIRTIO_MMIO_DEVICE_FEATURES:usize = 0x010;
const VIRTIO_MMIO_DRIVER_FEATURES:usize = 0x020;
const VIRTIO_MMIO_QUEUE_SEL:      usize = 0x030;
const VIRTIO_MMIO_QUEUE_NUM_MAX:  usize = 0x034;
const VIRTIO_MMIO_QUEUE_NUM:      usize = 0x038;
const VIRTIO_MMIO_QUEUE_READY:    usize = 0x044;
const VIRTIO_MMIO_QUEUE_NOTIFY:   usize = 0x050;
const VIRTIO_MMIO_INTERRUPT_ACK:  usize = 0x064;
const VIRTIO_MMIO_STATUS:         usize = 0x070;
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080;
const VIRTIO_MMIO_QUEUE_DESC_HIGH:usize = 0x084;
const VIRTIO_MMIO_QUEUE_AVAIL_LOW:usize = 0x090;
const VIRTIO_MMIO_QUEUE_AVAIL_HI: usize = 0x094;
const VIRTIO_MMIO_QUEUE_USED_LOW: usize = 0x0A0;
const VIRTIO_MMIO_QUEUE_USED_HIGH:usize = 0x0A4;

const VIRTIO_STATUS_ACKNOWLEDGE:  u32 = 1;
const VIRTIO_STATUS_DRIVER:       u32 = 2;
const VIRTIO_STATUS_DRIVER_OK:    u32 = 4;
const VIRTIO_STATUS_FEATURES_OK:  u32 = 8;

const VIRTIO_MAGIC: u32 = 0x74726976; // "virt" LE

const QUEUE_SIZE: usize = 16;

// virtio descriptor flags
const VIRTQ_DESC_F_WRITE: u16 = 2;

#[repr(C, align(16))]
struct VirtqDesc {
    addr:  u64,
    len:   u32,
    flags: u16,
    next:  u16,
}

#[repr(C, align(2))]
struct VirtqAvail {
    flags: u16,
    idx:   u16,
    ring:  [u16; QUEUE_SIZE],
}

#[repr(C, align(4))]
struct VirtqUsedElem {
    id:  u32,
    len: u32,
}

#[repr(C, align(4))]
struct VirtqUsed {
    flags: u16,
    idx:   u16,
    ring:  [VirtqUsedElem; QUEUE_SIZE],
}

// virtio_input_event (Linux evdev format)
#[repr(C)]
struct InputEvent {
    ev_type: u16,
    code:    u16,
    value:   u32,
}

const EV_KEY: u16 = 1;
const KEY_PRESS: u32 = 1;

// State for one virtio-input device (eventq = queue 0)
struct VirtioInputState {
    base:        usize,           // MMIO virt base
    desc:        *mut VirtqDesc,  // descriptor table (QUEUE_SIZE entries)
    avail:       *mut VirtqAvail,
    used:        *const VirtqUsed,
    bufs:        *mut InputEvent, // QUEUE_SIZE event buffers
    last_used:   u16,
    shift:       bool,
    ctrl:        bool,
}

static mut INPUT: VirtioInputState = VirtioInputState {
    base:      0,
    desc:      core::ptr::null_mut(),
    avail:     core::ptr::null_mut(),
    used:      core::ptr::null(),
    bufs:      core::ptr::null_mut(),
    last_used: 0,
    shift:     false,
    ctrl:      false,
};

static mut INPUT_READY: bool = false;

/// Try to initialize a virtio-input device at the given MMIO physical address.
/// Returns true if successful.
pub fn try_init(base_phys: usize) -> bool {
    unsafe {
        let vbase = phys_to_virt(base_phys);

        // Validate magic
        let magic = ptr::read_volatile(vbase as *const u32);
        if magic != VIRTIO_MAGIC { return false; }

        let version   = ptr::read_volatile((vbase + VIRTIO_MMIO_VERSION) as *const u32);
        let device_id = ptr::read_volatile((vbase + VIRTIO_MMIO_DEVICE_ID) as *const u32);

        // Device ID 18 = virtio-input
        if device_id != 18 {
            return false;
        }

        crate::serial_println!("[virtio-input] Found at {:#X} (version {})", base_phys, version);

        // ── Device initialization sequence ──────────────────────
        // 1. Reset
        ptr::write_volatile((vbase + VIRTIO_MMIO_STATUS) as *mut u32, 0);

        // 2. Acknowledge
        ptr::write_volatile((vbase + VIRTIO_MMIO_STATUS) as *mut u32, VIRTIO_STATUS_ACKNOWLEDGE);

        // 3. Driver
        ptr::write_volatile((vbase + VIRTIO_MMIO_STATUS) as *mut u32,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

        // 4. Negotiate features (none required for input)
        ptr::write_volatile((vbase + VIRTIO_MMIO_DRIVER_FEATURES) as *mut u32, 0);
        ptr::write_volatile((vbase + VIRTIO_MMIO_STATUS) as *mut u32,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

        // 5. Set up eventq (queue 0)
        ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_SEL) as *mut u32, 0);
        let qmax = ptr::read_volatile((vbase + VIRTIO_MMIO_QUEUE_NUM_MAX) as *const u32) as usize;
        let qsize = if qmax >= QUEUE_SIZE { QUEUE_SIZE } else { qmax };
        if qsize == 0 {
            crate::serial_println!("[virtio-input] Queue max = 0, aborting");
            return false;
        }
        ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_NUM) as *mut u32, qsize as u32);

        // Allocate 3 pages: descriptors + avail + used
        let pages_needed = 3;
        let frame_phys = match alloc_frames(pages_needed) {
            Some(p) => p,
            None => { crate::serial_println!("[virtio-input] OOM"); return false; }
        };
        let frame_virt = phys_to_virt(frame_phys);
        // Zero out
        ptr::write_bytes(frame_virt as *mut u8, 0, pages_needed * 4096);

        let desc_phys  = frame_phys;
        let avail_phys = frame_phys + 0x1000;
        let used_phys  = frame_phys + 0x2000;

        let desc_virt  = frame_virt as *mut VirtqDesc;
        let avail_virt = (frame_virt + 0x1000) as *mut VirtqAvail;
        let used_virt  = (frame_virt + 0x2000) as *const VirtqUsed;

        // Allocate event buffers (one InputEvent per descriptor)
        let buf_frame = match alloc_frames(1) {
            Some(p) => p,
            None => { crate::serial_println!("[virtio-input] OOM buf"); return false; }
        };
        let buf_virt = phys_to_virt(buf_frame) as *mut InputEvent;
        ptr::write_bytes(buf_virt as *mut u8, 0, 4096);

        // Set descriptor table: each entry points to one InputEvent buffer (device-writable)
        for i in 0..qsize {
            let buf_phys = buf_frame + i * core::mem::size_of::<InputEvent>();
            let desc = &mut *desc_virt.add(i);
            desc.addr  = buf_phys as u64;
            desc.len   = core::mem::size_of::<InputEvent>() as u32;
            desc.flags = VIRTQ_DESC_F_WRITE;
            desc.next  = 0;
        }

        // Make all descriptors available to device
        let avail = &mut *avail_virt;
        avail.flags = 0;
        avail.idx = qsize as u16;
        for i in 0..qsize {
            avail.ring[i] = i as u16;
        }

        // Program queue registers
        if version == 2 {
            // Modern MMIO (version 2): separate desc/avail/used addresses
            ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_DESC_LOW) as *mut u32,  (desc_phys & 0xFFFF_FFFF) as u32);
            ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_DESC_HIGH) as *mut u32, (desc_phys >> 32) as u32);
            ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_AVAIL_LOW) as *mut u32, (avail_phys & 0xFFFF_FFFF) as u32);
            ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_AVAIL_HI) as *mut u32,  (avail_phys >> 32) as u32);
            ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_USED_LOW) as *mut u32,  (used_phys & 0xFFFF_FFFF) as u32);
            ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_USED_HIGH) as *mut u32, (used_phys >> 32) as u32);
            ptr::write_volatile((vbase + VIRTIO_MMIO_QUEUE_READY) as *mut u32, 1);
        }
        // (version 1 legacy uses different layout — skip for now)

        // 6. Driver OK
        ptr::write_volatile((vbase + VIRTIO_MMIO_STATUS) as *mut u32,
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK | VIRTIO_STATUS_DRIVER_OK);

        INPUT = VirtioInputState {
            base:      vbase,
            desc:      desc_virt,
            avail:     avail_virt,
            used:      used_virt,
            bufs:      buf_virt,
            last_used: 0,
            shift:     false,
            ctrl:      false,
        };
        INPUT_READY = true;
        crate::serial_println!("[virtio-input] Keyboard ready (qsize={})", qsize);
        true
    }
}

/// Poll for keyboard input. Returns ASCII byte or None.
pub fn poll_keyboard() -> Option<u8> {
    unsafe {
        if !INPUT_READY { return None; }

        let used = &*INPUT.used;
        let used_idx = ptr::read_volatile(&used.idx);

        if used_idx == INPUT.last_used {
            return None; // Nothing new
        }

        let elem_idx = (INPUT.last_used as usize) % QUEUE_SIZE;
        let elem = &used.ring[elem_idx];
        let desc_id = ptr::read_volatile(&elem.id) as usize;

        INPUT.last_used = INPUT.last_used.wrapping_add(1);

        // Read the event
        let ev = &*INPUT.bufs.add(desc_id % QUEUE_SIZE);
        let ev_type = ptr::read_volatile(&ev.ev_type);
        let code    = ptr::read_volatile(&ev.code);
        let value   = ptr::read_volatile(&ev.value);

        // Re-arm: put descriptor back in available ring
        let avail = &mut *INPUT.avail;
        let avail_idx = ptr::read_volatile(&avail.idx) as usize;
        ptr::write_volatile(&mut avail.ring[avail_idx % QUEUE_SIZE], desc_id as u16);
        ptr::write_volatile(&mut avail.idx, avail.idx.wrapping_add(1));
        // Notify device (queue 0)
        ptr::write_volatile((INPUT.base + VIRTIO_MMIO_QUEUE_NOTIFY) as *mut u32, 0);

        // Only process EV_KEY events
        if ev_type != EV_KEY { return None; }

        // Handle key release — update modifier state
        if value == 0 {
            handle_key_release(code);
            return None;
        }

        // Only convert key press (value=1) and repeat (value=2) to ASCII
        if value > 2 { return None; }

        linux_keycode_to_ascii(code)
    }
}

/// Convert Linux evdev keycode to ASCII
fn linux_keycode_to_ascii(code: u16) -> Option<u8> {
    unsafe {
        // Track modifiers
        match code {
            42 | 54 => { INPUT.shift = true;  return None; } // L/R Shift press
            29 | 97 => { INPUT.ctrl  = true;  return None; } // L/R Ctrl press
            56 | 100=> { return None; }                       // Alt — ignore
            _       => {}
        }

        let shift = INPUT.shift;
        let ctrl  = INPUT.ctrl;

        // Linux evdev keycodes (https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)
        let ch: u8 = match code {
            1  => 0x1B, // ESC
            2  => if shift { b'!' } else { b'1' },
            3  => if shift { b'@' } else { b'2' },
            4  => if shift { b'#' } else { b'3' },
            5  => if shift { b'$' } else { b'4' },
            6  => if shift { b'%' } else { b'5' },
            7  => if shift { b'^' } else { b'6' },
            8  => if shift { b'&' } else { b'7' },
            9  => if shift { b'*' } else { b'8' },
            10 => if shift { b'(' } else { b'9' },
            11 => if shift { b')' } else { b'0' },
            12 => if shift { b'_' } else { b'-' },
            13 => if shift { b'+' } else { b'=' },
            14 => 0x7F, // Backspace
            15 => b'\t',
            16 => if ctrl { 0x11 } else if shift { b'Q' } else { b'q' },
            17 => if ctrl { 0x17 } else if shift { b'W' } else { b'w' },
            18 => if ctrl { 0x05 } else if shift { b'E' } else { b'e' },
            19 => if ctrl { 0x12 } else if shift { b'R' } else { b'r' },
            20 => if ctrl { 0x14 } else if shift { b'T' } else { b't' },
            21 => if ctrl { 0x19 } else if shift { b'Y' } else { b'y' },
            22 => if ctrl { 0x15 } else if shift { b'U' } else { b'u' },
            23 => if ctrl { 0x09 } else if shift { b'I' } else { b'i' },
            24 => if ctrl { 0x0F } else if shift { b'O' } else { b'o' },
            25 => if ctrl { 0x10 } else if shift { b'P' } else { b'p' },
            26 => if shift { b'{' } else { b'[' },
            27 => if shift { b'}' } else { b']' },
            28 => b'\n', // Enter
            30 => if ctrl { 0x01 } else if shift { b'A' } else { b'a' },
            31 => if ctrl { 0x13 } else if shift { b'S' } else { b's' },
            32 => if ctrl { 0x04 } else if shift { b'D' } else { b'd' },
            33 => if ctrl { 0x06 } else if shift { b'F' } else { b'f' },
            34 => if ctrl { 0x07 } else if shift { b'G' } else { b'g' },
            35 => if ctrl { 0x08 } else if shift { b'H' } else { b'h' },
            36 => if ctrl { 0x0A } else if shift { b'J' } else { b'j' },
            37 => if ctrl { 0x0B } else if shift { b'K' } else { b'k' },
            38 => if ctrl { 0x0C } else if shift { b'L' } else { b'l' },
            39 => if shift { b':' } else { b';' },
            40 => if shift { b'"' } else { b'\'' },
            41 => if shift { b'~' } else { b'`' },
            43 => if shift { b'|' } else { b'\\' },
            44 => if ctrl { 0x1A } else if shift { b'Z' } else { b'z' },
            45 => if ctrl { 0x18 } else if shift { b'X' } else { b'x' },
            46 => if ctrl { 0x03 } else if shift { b'C' } else { b'c' },
            47 => if ctrl { 0x16 } else if shift { b'V' } else { b'v' },
            48 => if ctrl { 0x02 } else if shift { b'B' } else { b'b' },
            49 => if ctrl { 0x0E } else if shift { b'N' } else { b'n' },
            50 => if ctrl { 0x0D } else if shift { b'M' } else { b'm' },
            51 => if shift { b'<' } else { b',' },
            52 => if shift { b'>' } else { b'.' },
            53 => if shift { b'?' } else { b'/' },
            57 => b' ', // Space
            // Numpad
            71 => b'7', 72 => b'8', 73 => b'9',
            75 => b'4', 76 => b'5', 77 => b'6',
            79 => b'1', 80 => b'2', 81 => b'3',
            82 => b'0', 83 => b'.', 96 => b'\n',
            // Modifier releases (key up events won't reach here, but just in case)
            _ => return None,
        };
        Some(ch)
    }
}

/// Handle modifier key releases (must be called when EV_KEY value=0)
pub fn handle_key_release(code: u16) {
    unsafe {
        match code {
            42 | 54 => INPUT.shift = false,
            29 | 97 => INPUT.ctrl  = false,
            _       => {}
        }
    }
}
