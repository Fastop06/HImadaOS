use core::ptr;
use crate::mm::vmm::{map_device_page, phys_to_virt};
use crate::mm::pmm::alloc_frame;
use crate::serial_println;

// ─────────────────────────────────────────────────────────────
// xHCI Controller & USB HID Keyboard Driver for HimadaOS ARM64
// Supports Parallels Desktop on Apple Silicon & QEMU
// ─────────────────────────────────────────────────────────────

#[repr(C, align(16))]
pub struct Trb {
    pub parameter: u64,
    pub status: u32,
    pub control: u32,
}

#[derive(Copy, Clone)]
pub struct XhciDevice {
    pub slot_id: u8,
    pub port_id: usize,
    pub speed: usize,
    pub active: bool,
    pub is_keyboard: bool,

    pub ep0_ring_phys: usize,
    pub ep0_ring_virt: *mut Trb,
    pub ep0_enqueue_idx: usize,
    pub ep0_cycle: u32,

    pub ep1_ring_phys: usize,
    pub ep1_ring_virt: *mut Trb,
    pub ep1_enqueue_idx: usize,
    pub ep1_cycle: u32,

    pub report_buf_phys: usize,
    pub report_buf_virt: *mut u8,
}

impl Default for XhciDevice {
    fn default() -> Self {
        Self {
            slot_id: 0,
            port_id: 0,
            speed: 0,
            active: false,
            is_keyboard: true,
            ep0_ring_phys: 0,
            ep0_ring_virt: ptr::null_mut(),
            ep0_enqueue_idx: 0,
            ep0_cycle: 1,
            ep1_ring_phys: 0,
            ep1_ring_virt: ptr::null_mut(),
            ep1_enqueue_idx: 0,
            ep1_cycle: 1,
            report_buf_phys: 0,
            report_buf_virt: ptr::null_mut(),
        }
    }
}

// Global xHCI State
pub struct XhciState {
    pub initialized: bool,
    pub mmio_base: usize,
    pub oper_base: usize,
    pub rts_base: usize,
    pub db_base: usize,
    pub cap_length: usize,
    pub max_slots: usize,
    pub max_ports: usize,
    pub context_size: usize,

    // Command Ring
    pub cmd_ring_phys: usize,
    pub cmd_ring_virt: *mut Trb,
    pub cmd_enqueue_idx: usize,
    pub cmd_cycle: u32,

    // Event Ring
    pub event_ring_phys: usize,
    pub event_ring_virt: *mut Trb,
    pub event_dequeue_idx: usize,
    pub event_cycle: u32,

    // Multi-device slots (up to 4 active USB input devices)
    pub devices: [XhciDevice; 4],
    pub num_devices: usize,

    pub prev_keycode: u8,
}

pub static mut XHCI: XhciState = XhciState {
    initialized: false,
    mmio_base: 0,
    oper_base: 0,
    rts_base: 0,
    db_base: 0,
    cap_length: 0,
    max_slots: 0,
    max_ports: 0,
    context_size: 32,

    cmd_ring_phys: 0,
    cmd_ring_virt: ptr::null_mut(),
    cmd_enqueue_idx: 0,
    cmd_cycle: 1,

    event_ring_phys: 0,
    event_ring_virt: ptr::null_mut(),
    event_dequeue_idx: 0,
    event_cycle: 1,

    devices: [
        XhciDevice {
            slot_id: 0, port_id: 0, speed: 0, active: false, is_keyboard: true,
            ep0_ring_phys: 0, ep0_ring_virt: ptr::null_mut(), ep0_enqueue_idx: 0, ep0_cycle: 1,
            ep1_ring_phys: 0, ep1_ring_virt: ptr::null_mut(), ep1_enqueue_idx: 0, ep1_cycle: 1,
            report_buf_phys: 0, report_buf_virt: ptr::null_mut(),
        },
        XhciDevice {
            slot_id: 0, port_id: 0, speed: 0, active: false, is_keyboard: true,
            ep0_ring_phys: 0, ep0_ring_virt: ptr::null_mut(), ep0_enqueue_idx: 0, ep0_cycle: 1,
            ep1_ring_phys: 0, ep1_ring_virt: ptr::null_mut(), ep1_enqueue_idx: 0, ep1_cycle: 1,
            report_buf_phys: 0, report_buf_virt: ptr::null_mut(),
        },
        XhciDevice {
            slot_id: 0, port_id: 0, speed: 0, active: false, is_keyboard: true,
            ep0_ring_phys: 0, ep0_ring_virt: ptr::null_mut(), ep0_enqueue_idx: 0, ep0_cycle: 1,
            ep1_ring_phys: 0, ep1_ring_virt: ptr::null_mut(), ep1_enqueue_idx: 0, ep1_cycle: 1,
            report_buf_phys: 0, report_buf_virt: ptr::null_mut(),
        },
        XhciDevice {
            slot_id: 0, port_id: 0, speed: 0, active: false, is_keyboard: true,
            ep0_ring_phys: 0, ep0_ring_virt: ptr::null_mut(), ep0_enqueue_idx: 0, ep0_cycle: 1,
            ep1_ring_phys: 0, ep1_ring_virt: ptr::null_mut(), ep1_enqueue_idx: 0, ep1_cycle: 1,
            report_buf_phys: 0, report_buf_virt: ptr::null_mut(),
        },
    ],
    num_devices: 0,

    prev_keycode: 0,
};

#[inline(always)]
unsafe fn clean_cache(vaddr: usize, len: usize) {
    crate::graphics::clean_dcache_range(vaddr, len);
}

pub fn init() {
    unsafe {
        // 1. Locate xHCI Controller
        let mmio_paddr = find_xhci_base();
        if mmio_paddr == 0 {
            serial_println!("[xHCI] No xHCI controller found.");
            return;
        }
        serial_println!("[xHCI] Probing xHCI Controller at {:#X}...", mmio_paddr);

        // Map MMIO pages (at least 256 pages = 1 MB to cover cap, oper, rts, and db registers)
        for i in 0..256 {
            map_device_page(mmio_paddr + i * 0x1000, mmio_paddr + i * 0x1000);
        }

        let base = mmio_paddr as *mut u8;
        let dword0 = match crate::hal::exceptions::safe_read_u32(mmio_paddr) {
            Some(d) => d,
            None => {
                serial_println!("[xHCI] Fault reading xHCI base {:#X}", mmio_paddr);
                return;
            }
        };
        let caplength = (dword0 & 0xFF) as usize;
        let hciversion = ((dword0 >> 16) & 0xFFFF) as u16;
        let hcsparams1 = ptr::read_volatile(base.add(4) as *const u32);
        let hccparams1 = ptr::read_volatile(base.add(0x10) as *const u32);
        let dboff = (ptr::read_volatile(base.add(0x14) as *const u32) & !3) as usize;
        let rtsoff = (ptr::read_volatile(base.add(0x18) as *const u32) & !0x1F) as usize;

        if caplength == 0 || caplength > 0x100 || (hciversion == 0 && caplength != 0x40 && caplength != 0x20) {
            serial_println!("[xHCI] Invalid capability registers (CapLen: {}, Version: {:#X})", caplength, hciversion);
            return;
        }

        let max_slots = (hcsparams1 & 0xFF) as usize;
        let max_ports = ((hcsparams1 >> 24) & 0xFF) as usize;
        let context_size = if (hccparams1 & (1 << 2)) != 0 { 64 } else { 32 };

        let oper_base = mmio_paddr + caplength;
        let rts_base = mmio_paddr + rtsoff;
        let db_base = mmio_paddr + dboff;

        // Ensure higher regions are mapped if dboff or rtsoff are large
        let max_offset = rtsoff.max(dboff) + 0x10000;
        let pages_needed = (max_offset + 0xFFF) / 0x1000;
        if pages_needed > 256 {
            for i in 256..pages_needed {
                map_device_page(mmio_paddr + i * 0x1000, mmio_paddr + i * 0x1000);
            }
        }

        serial_println!("[xHCI] CapLen: {}, Ver: {:#X}, Slots: {}, Ports: {}, CSZ: {}", 
            caplength, hciversion, max_slots, max_ports, context_size);

        XHCI.mmio_base = mmio_paddr;
        XHCI.oper_base = oper_base;
        XHCI.rts_base = rts_base;
        XHCI.db_base = db_base;
        XHCI.cap_length = caplength;
        XHCI.max_slots = max_slots;
        XHCI.max_ports = max_ports;
        XHCI.context_size = context_size;

        // 2. OS/BIOS Handoff (Claim OS Ownership)
        let ext_cap_ptr = ((hccparams1 >> 16) & 0xFFFF) as usize * 4;
        if ext_cap_ptr != 0 {
            let mut offset = ext_cap_ptr;
            while offset != 0 && offset < 0x2000 {
                let cap = ptr::read_volatile((mmio_paddr + offset) as *const u32);
                let cap_id = cap & 0xFF;
                if cap_id == 1 { // USB Legacy Support
                    ptr::write_volatile((mmio_paddr + offset) as *mut u32, cap | (1 << 24)); // Set OS Owned
                    for _ in 0..10_000 {
                        let c = ptr::read_volatile((mmio_paddr + offset) as *const u32);
                        if (c & (1 << 16)) == 0 { break; } // BIOS released
                        core::hint::spin_loop();
                    }
                    break;
                }
                let next = ((cap >> 8) & 0xFF) as usize * 4;
                if next == 0 { break; }
                offset += next;
            }
        }

        // 3. Stop Host Controller
        let mut usbcmd = ptr::read_volatile(oper_base as *const u32);
        if (usbcmd & 1) != 0 {
            ptr::write_volatile(oper_base as *mut u32, usbcmd & !1);
            for _ in 0..100_000 {
                let sts = ptr::read_volatile((oper_base + 4) as *const u32);
                if (sts & 1) != 0 { break; } // Halted
                core::hint::spin_loop();
            }
        }

        // 4. Reset Host Controller (HCRST)
        ptr::write_volatile(oper_base as *mut u32, 2);
        for _ in 0..100_000 {
            let cmd = ptr::read_volatile(oper_base as *const u32);
            let sts = ptr::read_volatile((oper_base + 4) as *const u32);
            if (cmd & 2) == 0 && (sts & (1 << 11)) == 0 { break; } // HCRST=0 and CNR=0
            core::hint::spin_loop();
        }

        // 5. Allocate Global DMA Frames
        let frame1_phys = match alloc_frame() {
            Some(f) => f,
            None => { serial_println!("[xHCI] OOM Frame 1"); return; }
        };
        let frame1_virt = phys_to_virt(frame1_phys) as *mut u8;
        ptr::write_bytes(frame1_virt, 0, 4096);
        clean_cache(frame1_virt as usize, 4096);

        // Memory Layout within Frame 1:
        // 0x000..0x200: DCBAA (64 pointers = 512 bytes)
        let dcbaa_phys = frame1_phys;
        let dcbaa_virt = frame1_virt as *mut u64;

        // 0x200..0x600: Command Ring (64 TRBs = 1024 bytes)
        let cmd_ring_phys = frame1_phys + 0x200;
        let cmd_ring_virt = frame1_virt.add(0x200) as *mut Trb;
        let cmd_link = &mut *cmd_ring_virt.add(63);
        cmd_link.parameter = cmd_ring_phys as u64;
        cmd_link.control = (6 << 10) | (1 << 1) | 1; // Type 6 (Link), TC=1, Cycle=1
        clean_cache(cmd_ring_virt as usize, 1024);

        // 0x600..0xA00: Event Ring (64 TRBs = 1024 bytes)
        let event_ring_phys = frame1_phys + 0x600;
        let event_ring_virt = frame1_virt.add(0x600) as *mut Trb;

        // 0xA00..0xA40: ERST (1 segment = 16 bytes)
        let erst_phys = frame1_phys + 0xA00;
        let erst_virt = frame1_virt.add(0xA00) as *mut u32;
        ptr::write(erst_virt, event_ring_phys as u32);
        ptr::write(erst_virt.add(1), (event_ring_phys >> 32) as u32);
        ptr::write(erst_virt.add(2), 64); // Size = 64
        ptr::write(erst_virt.add(3), 0);
        clean_cache(erst_virt as usize, 64);

        XHCI.cmd_ring_phys = cmd_ring_phys;
        XHCI.cmd_ring_virt = cmd_ring_virt;
        XHCI.cmd_enqueue_idx = 0;
        XHCI.cmd_cycle = 1;

        XHCI.event_ring_phys = event_ring_phys;
        XHCI.event_ring_virt = event_ring_virt;
        XHCI.event_dequeue_idx = 0;
        XHCI.event_cycle = 1;

        // 6. Program Controller Operational & Runtime Registers
        let max_slots_en = (max_slots.min(16)) as u32;
        ptr::write_volatile((oper_base + 0x38) as *mut u32, max_slots_en);

        // Program DCBAAP
        ptr::write_volatile((oper_base + 0x30) as *mut u64, dcbaa_phys as u64);

        // Program CRCR
        ptr::write_volatile((oper_base + 0x18) as *mut u64, (cmd_ring_phys as u64) | 1);

        // Program Event Ring in Interrupter 0
        ptr::write_volatile((rts_base + 0x28) as *mut u32, 1); // ERSTSZ = 1
        ptr::write_volatile((rts_base + 0x30) as *mut u32, erst_phys as u32); // ERSTBA
        ptr::write_volatile((rts_base + 0x34) as *mut u32, (erst_phys >> 32) as u32);
        ptr::write_volatile((rts_base + 0x38) as *mut u32, (event_ring_phys as u32) | (1 << 3)); // ERDP with EHB=1
        ptr::write_volatile((rts_base + 0x3C) as *mut u32, (event_ring_phys >> 32) as u32);

        // Enable Interrupter (IE = 1)
        ptr::write_volatile((rts_base + 0x20) as *mut u32, 2);

        // 7. Start Controller: USBCMD.RS = 1
        ptr::write_volatile(oper_base as *mut u32, 1);
        for _ in 0..100_000 {
            let sts = ptr::read_volatile((oper_base + 4) as *const u32);
            if (sts & 1) == 0 { break; } // Running!
            core::hint::spin_loop();
        }
        serial_println!("[xHCI] Host Controller started successfully.");

        // 8. Discover and Reset Connected Ports
        let mut connected_ports = [(0usize, 0usize); 4]; // (port, speed)
        let mut num_connected = 0;

        for port in 1..=max_ports {
            if num_connected >= 4 { break; }
            let portsc_ptr = (oper_base + 0x400 + (port - 1) * 0x10) as *mut u32;
            let mut sc = ptr::read_volatile(portsc_ptr);
            if (sc & 1) != 0 { // Current Connect Status (CCS = 1)
                serial_println!("[xHCI] Port {} device connected. Resetting...", port);
                // In xHCI PORTSC: R/W1CS bits (especially PED bit 1 and change bits 17..23)
                // MUST BE PRESERVED AS 0 to avoid disabling port or clearing status unexpectedly!
                const PORT_RWC_MASK: u32 = (1 << 1) | (1 << 17) | (1 << 18) | (1 << 19) | (1 << 20) | (1 << 21) | (1 << 22) | (1 << 23);
                ptr::write_volatile(portsc_ptr, (sc & !PORT_RWC_MASK) | (1 << 4)); // Assert PR=1

                for _ in 0..1_000_000 {
                    sc = ptr::read_volatile(portsc_ptr);
                    if (sc & (1 << 4)) == 0 && (sc & 2) != 0 { break; } // PR=0, PED=1 (Port Enabled)
                    core::hint::spin_loop();
                }
                // Clear Port Reset Change (PRC = bit 21)
                ptr::write_volatile(portsc_ptr, (sc & !PORT_RWC_MASK) | (1 << 21));

                let speed = (sc >> 10) & 0xF;
                let ped = (sc & 2) != 0;
                serial_println!("[xHCI] Port {} reset complete. PED={}, Speed: {}", port, ped, speed);

                if ped && speed != 0 {
                    connected_ports[num_connected] = (port, speed as usize);
                    num_connected += 1;
                }
            }
        }

        if num_connected == 0 {
            serial_println!("[xHCI] No active USB devices found on ports.");
            XHCI.initialized = true;
            return;
        }

        // 9. Configure Each Connected Port
        for i in 0..num_connected {
            let (port, speed) = connected_ports[i];
            let slot_id = send_enable_slot_cmd();
            if slot_id == 0 {
                serial_println!("[xHCI] Failed to enable device slot for Port {}.", port);
                continue;
            }
            serial_println!("[xHCI] Port {} assigned to Device Slot {}", port, slot_id);

            // Allocate 1 frame for this device's rings, buffers, and contexts
            let dev_frame_phys = match alloc_frame() {
                Some(f) => f,
                None => { serial_println!("[xHCI] OOM for device frame"); continue; }
            };
            let dev_frame_virt = phys_to_virt(dev_frame_phys) as *mut u8;
            ptr::write_bytes(dev_frame_virt, 0, 4096);
            clean_cache(dev_frame_virt as usize, 4096);

            // 0x000..0x100: EP0 Ring (16 TRBs = 256 bytes)
            let ep0_ring_phys = dev_frame_phys;
            let ep0_ring_virt = dev_frame_virt as *mut Trb;
            let ep0_link = &mut *ep0_ring_virt.add(15);
            ep0_link.parameter = ep0_ring_phys as u64;
            ep0_link.control = (6 << 10) | (1 << 1) | 1;

            // 0x100..0x200: EP1 Ring (16 TRBs = 256 bytes)
            let ep1_ring_phys = dev_frame_phys + 0x100;
            let ep1_ring_virt = dev_frame_virt.add(0x100) as *mut Trb;
            let ep1_link = &mut *ep1_ring_virt.add(15);
            ep1_link.parameter = ep1_ring_phys as u64;
            ep1_link.control = (6 << 10) | (1 << 1) | 1;

            clean_cache(ep0_ring_virt as usize, 256);
            clean_cache(ep1_ring_virt as usize, 256);

            // 0x200..0x280: HID Report Buffer (128 bytes)
            let report_buf_phys = dev_frame_phys + 0x200;
            let report_buf_virt = dev_frame_virt.add(0x200);

            // 0x800..0xC00: Input Context (1024 bytes)
            let input_ctx_phys = dev_frame_phys + 0x800;
            let input_ctx_virt = dev_frame_virt.add(0x800);

            // 0xC00..0x1000: Output Device Context (1024 bytes)
            let output_ctx_phys = dev_frame_phys + 0xC00;

            // Set DCBAA[slot_id] = output_ctx_phys
            *dcbaa_virt.add(slot_id as usize) = output_ctx_phys as u64;
            clean_cache(dcbaa_virt.add(slot_id as usize) as usize, 8);

            // 10. Address Device Command
            let ctrl_ctx = input_ctx_virt as *mut u32;
            ptr::write(ctrl_ctx.add(1), 0b11); // Add Slot (bit 0) + EP0 (bit 1)

            let slot_ctx = input_ctx_virt.add(context_size) as *mut u32;
            ptr::write(slot_ctx, ((speed as u32) << 20) | (4 << 27)); // Speed, Context Entries = 4
            ptr::write(slot_ctx.add(1), (port as u32) << 16); // Root Hub Port

            let ep0_ctx = input_ctx_virt.add(context_size * 2) as *mut u32;
            let mps = if speed == 3 { 64u32 } else { 8u32 };
            ptr::write(ep0_ctx.add(1), (4 << 3) | (mps << 16) | (3 << 1)); // EP Type 4 (Control), CErr=3
            ptr::write(ep0_ctx.add(2), (ep0_ring_phys as u32) | 1);
            ptr::write(ep0_ctx.add(3), (ep0_ring_phys >> 32) as u32);
            ptr::write(ep0_ctx.add(4), 8);
            clean_cache(input_ctx_virt as usize, 1024);

            if !send_address_device_cmd(slot_id, input_ctx_phys) {
                serial_println!("[xHCI] Address Device failed for Slot {}.", slot_id);
                continue;
            }
            serial_println!("[xHCI] Addressed Device on Slot {}", slot_id);

            // 11. Configure EP1 IN (Interrupt IN for Keyboard / HID)
            ptr::write_bytes(input_ctx_virt, 0, 1024);
            ptr::write(ctrl_ctx.add(1), (1 << 0) | (1 << 3)); // Add Slot + EP1 IN (DCI 3)
            ptr::write(slot_ctx, ((speed as u32) << 20) | (4 << 27));
            ptr::write(slot_ctx.add(1), (port as u32) << 16);

            let ep1_ctx = input_ctx_virt.add(context_size * 4) as *mut u32;
            ptr::write(ep1_ctx, 3 << 16); // Interval = 3 (8ms)
            ptr::write(ep1_ctx.add(1), (7 << 3) | (8 << 16) | (3 << 1)); // EP Type 7 (Interrupt IN), MPS=8, CErr=3
            ptr::write(ep1_ctx.add(2), (ep1_ring_phys as u32) | 1);
            ptr::write(ep1_ctx.add(3), (ep1_ring_phys >> 32) as u32);
            ptr::write(ep1_ctx.add(4), 8);
            clean_cache(input_ctx_virt as usize, 1024);

            if !send_configure_endpoint_cmd(slot_id, input_ctx_phys) {
                serial_println!("[xHCI] Configure Endpoint failed for Slot {}.", slot_id);
                continue;
            }
            serial_println!("[xHCI] Configured Endpoint 1 IN for Slot {}", slot_id);

            let desc_buf_phys = dev_frame_phys + 0x300;
            let desc_buf_virt = dev_frame_virt.add(0x300);

            let mut dev = XhciDevice {
                slot_id,
                port_id: port,
                speed,
                active: true,
                is_keyboard: true,
                ep0_ring_phys,
                ep0_ring_virt,
                ep0_enqueue_idx: 0,
                ep0_cycle: 1,
                ep1_ring_phys,
                ep1_ring_virt,
                ep1_enqueue_idx: 0,
                ep1_cycle: 1,
                report_buf_phys,
                report_buf_virt,
            };

            // 12. USB Standard & Class Requests
            // A. SET_CONFIGURATION(1) — Mandatory to transition device into Configured State!
            send_ep0_control_transfer(&mut dev, 0x00, 0x09, 1, 0, 0);
            for _ in 0..10_000 { core::hint::spin_loop(); }

            // Probe Configuration Descriptor to detect Interface Class and Protocol
            let mut is_keyboard = false;
            if send_ep0_get_descriptor(&mut dev, desc_buf_phys, desc_buf_virt, 2, 0, 64) {
                let w_total = (*desc_buf_virt.add(2) as usize) | ((*desc_buf_virt.add(3) as usize) << 8);
                let total_len = w_total.min(64);
                let mut offset = 0;
                while offset + 2 <= total_len {
                    let b_len = *desc_buf_virt.add(offset) as usize;
                    let b_type = *desc_buf_virt.add(offset + 1);
                    if b_len == 0 { break; }
                    if b_type == 4 && offset + 8 <= total_len {
                        let iface_class = *desc_buf_virt.add(offset + 5);
                        let iface_proto = *desc_buf_virt.add(offset + 7);
                        if iface_class == 3 {
                            if iface_proto == 1 {
                                is_keyboard = true;
                                serial_println!("[xHCI] Slot {} confirmed as USB Boot Keyboard (proto=1).", slot_id);
                            } else {
                                is_keyboard = false;
                                serial_println!("[xHCI] Slot {} identified as non-keyboard HID (proto={}).", slot_id, iface_proto);
                            }
                        }
                        break;
                    }
                    offset += b_len;
                }
            } else {
                // Fallback: Port 1 is Keyboard, others are not
                if port == 1 {
                    is_keyboard = true;
                    serial_println!("[xHCI] Port 1 assumed USB Keyboard by fallback.");
                } else {
                    is_keyboard = false;
                    serial_println!("[xHCI] Port {} assumed non-keyboard by fallback.", port);
                }
            }
            dev.is_keyboard = is_keyboard;

            if is_keyboard {
                // B. SET_IDLE(0, 0) — Only report when key state changes
                send_ep0_control_transfer(&mut dev, 0x21, 0x0A, 0, 0, 0);
                for _ in 0..10_000 { core::hint::spin_loop(); }

                // C. SET_PROTOCOL(0) — Switch HID device to Boot Protocol
                send_ep0_control_transfer(&mut dev, 0x21, 0x0B, 0, 0, 0);
                for _ in 0..10_000 { core::hint::spin_loop(); }

                // 13. Queue Initial Normal TRB for Keyboard Input
                queue_ep1_trb(&mut dev);
                serial_println!("[xHCI] Keyboard on Port {} (Slot {}) active and polling!", port, slot_id);
            } else {
                serial_println!("[xHCI] Device on Port {} (Slot {}) configured dormant (mouse ignore).", port, slot_id);
            }

            let dev_idx = XHCI.num_devices;
            XHCI.devices[dev_idx] = dev;
            XHCI.num_devices += 1;
        }

        drain_event_ring();
        XHCI.initialized = true;
        serial_println!("[xHCI] Driver ready. Total active devices: {}", XHCI.num_devices);
    }
}

// ─────────────────────────────────────────────────────────────
// Command Helpers
// ─────────────────────────────────────────────────────────────

unsafe fn send_enable_slot_cmd() -> u8 {
    let trb = &mut *XHCI.cmd_ring_virt.add(XHCI.cmd_enqueue_idx);
    trb.parameter = 0;
    trb.status = 0;
    trb.control = (9 << 10) | XHCI.cmd_cycle; // Type 9 = Enable Slot Command
    clean_cache(trb as *const _ as usize, 16);

    advance_cmd_enqueue();
    ring_doorbell(0, 0);

    // Wait for Command Completion Event
    if let Some((code, slot)) = wait_command_completion() {
        if code == 1 { return slot; } // Success
    }
    0
}

unsafe fn send_address_device_cmd(slot_id: u8, input_ctx_phys: usize) -> bool {
    let trb = &mut *XHCI.cmd_ring_virt.add(XHCI.cmd_enqueue_idx);
    trb.parameter = input_ctx_phys as u64;
    trb.status = 0;
    trb.control = (11 << 10) | ((slot_id as u32) << 24) | XHCI.cmd_cycle; // Type 11 = Address Device
    clean_cache(trb as *const _ as usize, 16);

    advance_cmd_enqueue();
    ring_doorbell(0, 0);

    if let Some((code, _)) = wait_command_completion() {
        return code == 1;
    }
    false
}

unsafe fn send_configure_endpoint_cmd(slot_id: u8, input_ctx_phys: usize) -> bool {
    let trb = &mut *XHCI.cmd_ring_virt.add(XHCI.cmd_enqueue_idx);
    trb.parameter = input_ctx_phys as u64;
    trb.status = 0;
    trb.control = (12 << 10) | ((slot_id as u32) << 24) | XHCI.cmd_cycle; // Type 12 = Configure Endpoint
    clean_cache(trb as *const _ as usize, 16);

    advance_cmd_enqueue();
    ring_doorbell(0, 0);

    if let Some((code, _)) = wait_command_completion() {
        return code == 1;
    }
    false
}

unsafe fn send_ep0_control_transfer(dev: &mut XhciDevice, req_type: u8, request: u8, val: u16, idx: u16, len: u16) {
    let setup_param = (req_type as u64) | ((request as u64) << 8) | ((val as u64) << 16) | ((idx as u64) << 32) | ((len as u64) << 48);

    // Setup Stage TRB
    let trb0 = &mut *dev.ep0_ring_virt.add(dev.ep0_enqueue_idx);
    trb0.parameter = setup_param;
    trb0.status = 8;
    trb0.control = (2 << 10) | (1 << 6) | dev.ep0_cycle; // Type 2 (Setup), IDT=1, TRT=0 (No Data Stage)
    clean_cache(trb0 as *const _ as usize, 16);
    advance_dev_ep0_enqueue(dev);

    // Status Stage TRB
    let trb1 = &mut *dev.ep0_ring_virt.add(dev.ep0_enqueue_idx);
    trb1.parameter = 0;
    trb1.status = 0;
    trb1.control = (4 << 10) | (1 << 16) | (1 << 5) | dev.ep0_cycle; // Type 4 (Status), IN, IOC=1
    clean_cache(trb1 as *const _ as usize, 16);
    advance_dev_ep0_enqueue(dev);

    // Ring Doorbell: slot_id with Target = 1 (EP0)
    ring_doorbell(dev.slot_id, 1);
}

unsafe fn send_ep0_get_descriptor(
    dev: &mut XhciDevice,
    buf_phys: usize,
    buf_virt: *mut u8,
    desc_type: u8,
    desc_idx: u8,
    length: u16,
) -> bool {
    ptr::write_bytes(buf_virt, 0, length as usize);
    clean_cache(buf_virt as usize, length as usize);

    let w_val = ((desc_type as u16) << 8) | (desc_idx as u16);
    let setup_param = 0x80u64 | (0x06u64 << 8) | ((w_val as u64) << 16) | ((length as u64) << 48);

    // 1. Setup Stage TRB
    let trb0 = &mut *dev.ep0_ring_virt.add(dev.ep0_enqueue_idx);
    trb0.parameter = setup_param;
    trb0.status = 8;
    trb0.control = (2 << 10) | (1 << 6) | (3 << 16) | dev.ep0_cycle; // Type 2 (Setup), IDT=1, TRT=3 (IN Data Stage)
    clean_cache(trb0 as *const _ as usize, 16);
    advance_dev_ep0_enqueue(dev);

    // 2. Data Stage TRB
    let trb1 = &mut *dev.ep0_ring_virt.add(dev.ep0_enqueue_idx);
    trb1.parameter = buf_phys as u64;
    trb1.status = length as u32;
    trb1.control = (3 << 10) | (1 << 16) | dev.ep0_cycle; // Type 3 (Data), DIR=1 (IN)
    clean_cache(trb1 as *const _ as usize, 16);
    advance_dev_ep0_enqueue(dev);

    // 3. Status Stage TRB
    let trb2 = &mut *dev.ep0_ring_virt.add(dev.ep0_enqueue_idx);
    trb2.parameter = 0;
    trb2.status = 0;
    trb2.control = (4 << 10) | (1 << 5) | dev.ep0_cycle; // Type 4 (Status), DIR=0 (OUT for IN data transfer), IOC=1
    clean_cache(trb2 as *const _ as usize, 16);
    advance_dev_ep0_enqueue(dev);

    // Ring Doorbell: slot_id with Target = 1 (EP0)
    ring_doorbell(dev.slot_id, 1);

    for _ in 0..50_000 {
        core::hint::spin_loop();
    }
    core::arch::asm!("dc ivac, {}", in(reg) buf_virt as usize);
    core::arch::asm!("dsb ish");

    *buf_virt > 0
}

unsafe fn queue_ep1_trb(dev: &mut XhciDevice) {
    let trb = &mut *dev.ep1_ring_virt.add(dev.ep1_enqueue_idx);
    trb.parameter = dev.report_buf_phys as u64;
    trb.status = 8;
    trb.control = (1 << 10) | (1 << 5) | (1 << 2) | dev.ep1_cycle; // Type 1 (Normal), IOC=1, ISP=1
    clean_cache(trb as *const _ as usize, 16);

    advance_dev_ep1_enqueue(dev);
    ring_doorbell(dev.slot_id, 3); // Target 3 = EP1 IN (DCI 3)
}

// ─────────────────────────────────────────────────────────────
// Ring & Event Processing
// ─────────────────────────────────────────────────────────────

unsafe fn advance_cmd_enqueue() {
    XHCI.cmd_enqueue_idx += 1;
    if XHCI.cmd_enqueue_idx == 63 {
        let link = &mut *XHCI.cmd_ring_virt.add(63);
        link.control = (6 << 10) | (1 << 1) | XHCI.cmd_cycle;
        clean_cache(link as *const _ as usize, 16);
        XHCI.cmd_cycle ^= 1;
        XHCI.cmd_enqueue_idx = 0;
    }
}

unsafe fn advance_dev_ep0_enqueue(dev: &mut XhciDevice) {
    dev.ep0_enqueue_idx += 1;
    if dev.ep0_enqueue_idx == 15 {
        let link = &mut *dev.ep0_ring_virt.add(15);
        link.control = (6 << 10) | (1 << 1) | dev.ep0_cycle;
        clean_cache(link as *const _ as usize, 16);
        dev.ep0_cycle ^= 1;
        dev.ep0_enqueue_idx = 0;
    }
}

unsafe fn advance_dev_ep1_enqueue(dev: &mut XhciDevice) {
    dev.ep1_enqueue_idx += 1;
    if dev.ep1_enqueue_idx == 15 {
        let link = &mut *dev.ep1_ring_virt.add(15);
        link.control = (6 << 10) | (1 << 1) | dev.ep1_cycle;
        clean_cache(link as *const _ as usize, 16);
        dev.ep1_cycle ^= 1;
        dev.ep1_enqueue_idx = 0;
    }
}

unsafe fn ring_doorbell(slot: u8, target: u32) {
    let db_ptr = (XHCI.db_base + (slot as usize) * 4) as *mut u32;
    ptr::write_volatile(db_ptr, target);
}

unsafe fn wait_command_completion() -> Option<(u8, u8)> {
    for _ in 0..100_000 {
        let ev_addr = XHCI.event_ring_virt.add(XHCI.event_dequeue_idx) as usize;
        core::arch::asm!("dc ivac, {}", in(reg) ev_addr);
        core::arch::asm!("dsb ish");

        let ev = &*XHCI.event_ring_virt.add(XHCI.event_dequeue_idx);
        let ctrl = ptr::read_volatile(&ev.control);
        if (ctrl & 1) == XHCI.event_cycle {
            let trb_type = (ctrl >> 10) & 0x3F;
            let status = ptr::read_volatile(&ev.status);
            let completion_code = ((status >> 24) & 0xFF) as u8;
            let slot_id = ((ctrl >> 24) & 0xFF) as u8;

            advance_event_dequeue();

            if trb_type == 33 { // Command Completion Event
                return Some((completion_code, slot_id));
            }
        }
        core::hint::spin_loop();
    }
    None
}

unsafe fn drain_event_ring() {
    for _ in 0..10_000 {
        let ev_addr = XHCI.event_ring_virt.add(XHCI.event_dequeue_idx) as usize;
        core::arch::asm!("dc ivac, {}", in(reg) ev_addr);
        core::arch::asm!("dsb ish");

        let ev = &*XHCI.event_ring_virt.add(XHCI.event_dequeue_idx);
        let ctrl = ptr::read_volatile(&ev.control);
        if (ctrl & 1) == XHCI.event_cycle {
            advance_event_dequeue();
        } else {
            break;
        }
    }
}

unsafe fn advance_event_dequeue() {
    XHCI.event_dequeue_idx += 1;
    if XHCI.event_dequeue_idx == 64 {
        XHCI.event_dequeue_idx = 0;
        XHCI.event_cycle ^= 1;
    }
    let erdp = XHCI.event_ring_phys + XHCI.event_dequeue_idx * 16;
    ptr::write_volatile((XHCI.rts_base + 0x38) as *mut u32, (erdp as u32) | (1 << 3));
    ptr::write_volatile((XHCI.rts_base + 0x3C) as *mut u32, (erdp >> 32) as u32);
}

// ─────────────────────────────────────────────────────────────
// Keyboard Polling & Keycode Translation
// ─────────────────────────────────────────────────────────────

static mut KEY_QUEUE: [u8; 32] = [0; 32];
static mut KEY_HEAD: usize = 0;
static mut KEY_TAIL: usize = 0;

fn push_key(b: u8) {
    unsafe {
        let next = (KEY_TAIL + 1) % KEY_QUEUE.len();
        if next != KEY_HEAD {
            KEY_QUEUE[KEY_TAIL] = b;
            KEY_TAIL = next;
        }
    }
}

fn pop_key() -> Option<u8> {
    unsafe {
        if KEY_HEAD == KEY_TAIL {
            None
        } else {
            let b = KEY_QUEUE[KEY_HEAD];
            KEY_HEAD = (KEY_HEAD + 1) % KEY_QUEUE.len();
            Some(b)
        }
    }
}

pub fn poll_keyboard() -> Option<u8> {
    // 1. Drain any pending bytes from multi-byte escape sequences (arrows, home, end, etc.)
    if let Some(b) = pop_key() {
        return Some(b);
    }

    unsafe {
        if !XHCI.initialized || XHCI.num_devices == 0 { return None; }

        // Check up to 16 events in ring to not stall behind skipped non-keyboard events
        for _ in 0..16 {
            let ev_addr = XHCI.event_ring_virt.add(XHCI.event_dequeue_idx) as usize;
            core::arch::asm!("dc ivac, {}", in(reg) ev_addr);
            core::arch::asm!("dsb ish");

            let ev = &*XHCI.event_ring_virt.add(XHCI.event_dequeue_idx);
            let ctrl = ptr::read_volatile(&ev.control);
            if (ctrl & 1) != XHCI.event_cycle {
                return None;
            }

            let trb_type = (ctrl >> 10) & 0x3F;
            let slot_id = ((ctrl >> 24) & 0xFF) as u8;
            let ep_id = ((ctrl >> 16) & 0x1F) as u8;

            advance_event_dequeue();

            if trb_type == 32 && ep_id == 3 {
                for dev in XHCI.devices.iter_mut() {
                    if dev.active && dev.slot_id == slot_id {
                        if !dev.is_keyboard {
                            // Non-keyboard device (e.g. mouse) - ignore completely
                            break;
                        }

                        // Invalidate report buffer cache
                        core::arch::asm!("dc ivac, {}", in(reg) dev.report_buf_virt);
                        core::arch::asm!("dsb ish");

                        let modifier = *dev.report_buf_virt;
                        let reserved = *dev.report_buf_virt.add(1);
                        let keycode = *dev.report_buf_virt.add(2);

                        // Re-arm EP1 transfer immediately
                        queue_ep1_trb(dev);

                        // Standard USB HID Boot Protocol Keyboard Report:
                        // byte 0: modifier keys
                        // byte 1: reserved (ALWAYS 0 in boot keyboard report!)
                        // byte 2: keycode 1
                        if reserved == 0 {
                            if keycode != 0 && keycode != XHCI.prev_keycode {
                                XHCI.prev_keycode = keycode;
                                if let Some(ch) = hid_to_ascii(modifier, keycode) {
                                    return Some(ch);
                                }
                            } else if keycode == 0 {
                                XHCI.prev_keycode = 0;
                            }
                        }
                        break;
                    }
                }
            }
        }
        None
    }
}

pub fn has_input() -> bool {
    unsafe {
        if KEY_HEAD != KEY_TAIL { return true; }
        if !XHCI.initialized || XHCI.num_devices == 0 { return false; }
        let ev_addr = XHCI.event_ring_virt.add(XHCI.event_dequeue_idx) as usize;
        core::arch::asm!("dc ivac, {}", in(reg) ev_addr);
        core::arch::asm!("dsb ish");
        let ev = &*XHCI.event_ring_virt.add(XHCI.event_dequeue_idx);
        let ctrl = ptr::read_volatile(&ev.control);
        (ctrl & 1) == XHCI.event_cycle
    }
}

fn hid_to_ascii(modifier: u8, keycode: u8) -> Option<u8> {
    let shift = (modifier & 0x22) != 0; // Left Shift (0x02) or Right Shift (0x20)
    let ctrl = (modifier & 0x11) != 0;  // Left Ctrl (0x01) or Right Ctrl (0x10)

    match keycode {
        0x04..=0x1D => { // 'a' - 'z' (0x04 = 'a')
            let base = b'a' + (keycode - 0x04);
            if ctrl {
                Some(keycode - 0x04 + 1) // Ctrl+A = 1, Ctrl+C = 3, Ctrl+L = 12, etc.
            } else if shift {
                Some(base - 32) // Upper case 'A' - 'Z'
            } else {
                Some(base)
            }
        }
        0x1E..=0x27 => { // '1' - '0' (0x1E = '1', 0x27 = '0')
            let normal = [b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0'];
            let shifted = [b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')'];
            let idx = (keycode - 0x1E) as usize;
            Some(if shift { shifted[idx] } else { normal[idx] })
        }
        0x28 => Some(b'\n'), // Enter
        0x29 => Some(0x1B),  // Escape
        0x2A => Some(0x7F),  // Backspace (ASCII DEL 0x7F)
        0x2B => Some(b'\t'), // Tab
        0x2C => Some(b' '),  // Space
        0x2D => Some(if shift { b'_' } else { b'-' }), // - and _
        0x2E => Some(if shift { b'+' } else { b'=' }), // = and +
        0x2F => Some(if shift { b'{' } else { b'[' }), // [ and {
        0x30 => Some(if shift { b'}' } else { b']' }), // ] and }
        0x31 => Some(if shift { b'|' } else { b'\\' }), // \ and |
        0x33 => Some(if shift { b':' } else { b';' }), // ; and :
        0x34 => Some(if shift { b'"' } else { b'\'' }), // ' and "
        0x35 => Some(if shift { b'~' } else { b'`' }), // ` and ~
        0x36 => Some(if shift { b'<' } else { b',' }), // , and <
        0x37 => Some(if shift { b'>' } else { b'.' }), // . and >
        0x38 => Some(if shift { b'?' } else { b'/' }), // / and ?

        // Special Navigation & Editing keys (VT100 ANSI sequences)
        0x52 => { // Up Arrow -> \x1b[A
            push_key(b'[');
            push_key(b'A');
            Some(0x1B)
        }
        0x51 => { // Down Arrow -> \x1b[B
            push_key(b'[');
            push_key(b'B');
            Some(0x1B)
        }
        0x50 => { // Left Arrow -> \x1b[D
            push_key(b'[');
            push_key(b'D');
            Some(0x1B)
        }
        0x4F => { // Right Arrow -> \x1b[C
            push_key(b'[');
            push_key(b'C');
            Some(0x1B)
        }
        0x4A => { // Home -> \x1b[H
            push_key(b'[');
            push_key(b'H');
            Some(0x1B)
        }
        0x4D => { // End -> \x1b[F
            push_key(b'[');
            push_key(b'F');
            Some(0x1B)
        }
        0x4C => { // Delete -> \x1b[3~
            push_key(b'[');
            push_key(b'3');
            push_key(b'~');
            Some(0x1B)
        }
        0x4B => { // PageUp -> \x1b[5~
            push_key(b'[');
            push_key(b'5');
            push_key(b'~');
            Some(0x1B)
        }
        0x4E => { // PageDown -> \x1b[6~
            push_key(b'[');
            push_key(b'6');
            push_key(b'~');
            Some(0x1B)
        }

        // Numpad
        0x59..=0x61 => Some(b'1' + (keycode - 0x59)),
        0x62 => Some(b'0'),
        0x63 => Some(b'.'),
        0x58 => Some(b'\n'),
        0x54 => Some(b'/'),
        0x55 => Some(b'*'),
        0x56 => Some(b'-'),
        0x57 => Some(b'+'),

        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────
// Hardware Scanning
// ─────────────────────────────────────────────────────────────

fn find_xhci_base() -> usize {
    // 1. Scan PCIe ECAM if available from ACPI MCFG
    let mut ecam_candidates = [0usize; 4];
    let mut num_candidates = 0;

    if let Some(ecam) = unsafe { crate::hal::acpi::ACPI_INFO.ecam_base } {
        ecam_candidates[num_candidates] = ecam;
        num_candidates += 1;
    }
    for &fb in &[0x40_1000_0000usize, 0x3F00_0000usize, 0x1000_0000usize] {
        if num_candidates < 4 && !ecam_candidates[..num_candidates].contains(&fb) {
            ecam_candidates[num_candidates] = fb;
            num_candidates += 1;
        }
    }

    for &ecam in &ecam_candidates[..num_candidates] {
        for bus in 0..4 {
            for dev in 0..32 {
                for func in 0..8 {
                    let dev_addr = ecam + ((bus << 20) | (dev << 15) | (func << 12));
                    unsafe {
                        map_device_page(dev_addr, dev_addr);
                        let id_reg = match crate::hal::exceptions::safe_read_u32(dev_addr) {
                            Some(v) => v,
                            None => 0xFFFF_FFFF,
                        };
                        if id_reg == 0xFFFF_FFFF || id_reg == 0 {
                            if func == 0 { break; }
                            continue;
                        }
                        let class_reg = ptr::read_volatile((dev_addr + 8) as *const u32);
                        let class = (class_reg >> 24) & 0xFF;
                        let subclass = (class_reg >> 16) & 0xFF;
                        let prog_if = (class_reg >> 8) & 0xFF;
                        if class == 0x0C && subclass == 0x03 && prog_if == 0x30 {
                            // Enable Memory Space (bit 1) and Bus Master (bit 2)
                            let cmd = ptr::read_volatile((dev_addr + 4) as *const u16);
                            ptr::write_volatile((dev_addr + 4) as *mut u16, cmd | 0b111);

                            let bar0 = ptr::read_volatile((dev_addr + 0x10) as *const u32);
                            let bar1 = ptr::read_volatile((dev_addr + 0x14) as *const u32);
                            let is_64 = (bar0 & 0b110) == 0b100;
                            let bar_addr = if is_64 {
                                (((bar1 as u64) << 32) | ((bar0 & !0xF) as u64)) as usize
                            } else {
                                (bar0 & !0xF) as usize
                            };
                            serial_println!("[PCI] Found xHCI: dev_addr={:#X}, bar0={:#X}, bar1={:#X} -> bar_addr={:#X}", dev_addr, bar0, bar1, bar_addr);
                            if bar_addr != 0 {
                                return bar_addr;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Parallels Desktop Apple Silicon Known Physical MMIO Base fallback
    0x0217_0000
}
