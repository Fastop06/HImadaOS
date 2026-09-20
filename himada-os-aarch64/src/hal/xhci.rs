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

    // Ring Pointers (Virtual & Physical)
    pub cmd_ring_phys: usize,
    pub cmd_ring_virt: *mut Trb,
    pub cmd_enqueue_idx: usize,
    pub cmd_cycle: u32,

    pub event_ring_phys: usize,
    pub event_ring_virt: *mut Trb,
    pub event_dequeue_idx: usize,
    pub event_cycle: u32,

    // Keyboard Slot & Endpoints
    pub kbd_slot: u8,
    pub kbd_port: usize,
    pub kbd_ready: bool,

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

    kbd_slot: 0,
    kbd_port: 0,
    kbd_ready: false,

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

        // Map MMIO pages (64 KB = 16 pages)
        for i in 0..16 {
            map_device_page(mmio_paddr + i * 0x1000, mmio_paddr + i * 0x1000);
        }

        let base = mmio_paddr as *mut u8;
        let dword0 = ptr::read_volatile(base as *const u32);
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
            for _ in 0..50_000 {
                let sts = ptr::read_volatile((oper_base + 4) as *const u32);
                if (sts & 1) != 0 { break; } // Halted
                core::hint::spin_loop();
            }
        }

        // 4. Reset Host Controller (HCRST)
        ptr::write_volatile(oper_base as *mut u32, 2);
        for _ in 0..50_000 {
            let cmd = ptr::read_volatile(oper_base as *const u32);
            let sts = ptr::read_volatile((oper_base + 4) as *const u32);
            if (cmd & 2) == 0 && (sts & (1 << 11)) == 0 { break; } // HCRST=0 and CNR=0
            core::hint::spin_loop();
        }

        // 5. Allocate DMA Frames
        let frame1_phys = match alloc_frame() {
            Some(f) => f,
            None => { serial_println!("[xHCI] OOM Frame 1"); return; }
        };
        let frame1_virt = phys_to_virt(frame1_phys) as *mut u8;
        ptr::write_bytes(frame1_virt, 0, 4096);
        clean_cache(frame1_virt as usize, 4096);

        let frame2_phys = match alloc_frame() {
            Some(f) => f,
            None => { serial_println!("[xHCI] OOM Frame 2"); return; }
        };
        let frame2_virt = phys_to_virt(frame2_phys) as *mut u8;
        ptr::write_bytes(frame2_virt, 0, 4096);
        clean_cache(frame2_virt as usize, 4096);

        // Memory Layout within Frame 1:
        // 0x000..0x200: DCBAA (64 pointers = 512 bytes)
        let dcbaa_phys = frame1_phys;
        let dcbaa_virt = frame1_virt as *mut u64;

        // 0x200..0x600: Command Ring (64 TRBs = 1024 bytes)
        let cmd_ring_phys = frame1_phys + 0x200;
        let cmd_ring_virt = frame1_virt.add(0x200) as *mut Trb;
        // Last TRB is Link TRB back to start
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

        // 0xA40..0xB40: EP0 Control Transfer Ring (16 TRBs = 256 bytes)
        let ep0_ring_phys = frame1_phys + 0xA40;
        let ep0_ring_virt = frame1_virt.add(0xA40) as *mut Trb;
        let ep0_link = &mut *ep0_ring_virt.add(15);
        ep0_link.parameter = ep0_ring_phys as u64;
        ep0_link.control = (6 << 10) | (1 << 1) | 1;
        clean_cache(ep0_ring_virt as usize, 256);

        // 0xB40..0xC40: EP1 Interrupt IN Transfer Ring (16 TRBs = 256 bytes)
        let ep1_ring_phys = frame1_phys + 0xB40;
        let ep1_ring_virt = frame1_virt.add(0xB40) as *mut Trb;
        let ep1_link = &mut *ep1_ring_virt.add(15);
        ep1_link.parameter = ep1_ring_phys as u64;
        ep1_link.control = (6 << 10) | (1 << 1) | 1;
        clean_cache(ep1_ring_virt as usize, 256);

        // 0xC40..0xC80: HID Report Buffer (64 bytes)
        let report_buf_phys = frame1_phys + 0xC40;
        let report_buf_virt = frame1_virt.add(0xC40);

        // Memory Layout within Frame 2:
        // 0x000..0x800: Input Context (2048 bytes)
        let input_ctx_phys = frame2_phys;
        let input_ctx_virt = frame2_virt;

        // 0x800..0x1000: Output Device Context (2048 bytes)
        let output_ctx_phys = frame2_phys + 0x800;
        let output_ctx_virt = frame2_virt.add(0x800);

        XHCI.cmd_ring_phys = cmd_ring_phys;
        XHCI.cmd_ring_virt = cmd_ring_virt;
        XHCI.cmd_enqueue_idx = 0;
        XHCI.cmd_cycle = 1;

        XHCI.event_ring_phys = event_ring_phys;
        XHCI.event_ring_virt = event_ring_virt;
        XHCI.event_dequeue_idx = 0;
        XHCI.event_cycle = 1;

        XHCI.ep0_ring_phys = ep0_ring_phys;
        XHCI.ep0_ring_virt = ep0_ring_virt;
        XHCI.ep0_enqueue_idx = 0;
        XHCI.ep0_cycle = 1;

        XHCI.ep1_ring_phys = ep1_ring_phys;
        XHCI.ep1_ring_virt = ep1_ring_virt;
        XHCI.ep1_enqueue_idx = 0;
        XHCI.ep1_cycle = 1;

        XHCI.report_buf_phys = report_buf_phys;
        XHCI.report_buf_virt = report_buf_virt;

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
        ptr::write_volatile((rts_base + 0x38) as *mut u32, (event_ring_phys as u32) | (1 << 3)); // ERDP
        ptr::write_volatile((rts_base + 0x3C) as *mut u32, (event_ring_phys >> 32) as u32);

        // Enable Interrupter (IE = 1)
        ptr::write_volatile((rts_base + 0x20) as *mut u32, 2);

        // 7. Start Controller: USBCMD.RS = 1
        ptr::write_volatile(oper_base as *mut u32, 1);
        for _ in 0..50_000 {
            let sts = ptr::read_volatile((oper_base + 4) as *const u32);
            if (sts & 1) == 0 { break; } // Running!
            core::hint::spin_loop();
        }
        serial_println!("[xHCI] Host Controller started successfully.");

        // 8. Discover and Reset Connected Ports
        let mut target_port = 0;
        let mut target_speed = 0;

        for port in 1..=max_ports {
            let portsc_ptr = (oper_base + 0x400 + (port - 1) * 0x10) as *mut u32;
            let mut sc = ptr::read_volatile(portsc_ptr);
            if (sc & 1) != 0 { // Current Connect Status (CCS = 1)
                serial_println!("[xHCI] Port {} device connected. Resetting...", port);
                // Reset port (PR = bit 4)
                ptr::write_volatile(portsc_ptr, sc | (1 << 4));
                for _ in 0..50_000 {
                    sc = ptr::read_volatile(portsc_ptr);
                    if (sc & (1 << 4)) == 0 && (sc & 2) != 0 { break; } // PR=0, PED=1 (Port Enabled)
                    core::hint::spin_loop();
                }
                let speed = (sc >> 10) & 0xF;
                serial_println!("[xHCI] Port {} reset complete. Speed: {}", port, speed);
                
                // In Parallels: Port 2 is the Keyboard (Port 1 is Mouse)
                // In QEMU: First connected port with speed is the Keyboard
                if port == 2 || target_port == 0 {
                    target_port = port;
                    target_speed = speed;
                }
            }
        }

        if target_port == 0 {
            serial_println!("[xHCI] No active USB devices on ports.");
            XHCI.initialized = true;
            return;
        }

        // 9. Enable Slot Command
        let slot_id = send_enable_slot_cmd();
        if slot_id == 0 {
            serial_println!("[xHCI] Failed to enable device slot.");
            return;
        }
        serial_println!("[xHCI] Enabled Device Slot {}", slot_id);
        XHCI.kbd_slot = slot_id;
        XHCI.kbd_port = target_port;

        // Set DCBAA[slot_id] = output_ctx_phys
        *dcbaa_virt.add(slot_id as usize) = output_ctx_phys as u64;
        clean_cache(dcbaa_virt.add(slot_id as usize) as usize, 8);

        // 10. Address Device Command
        // Setup Input Context:
        // Control Context: add Slot (bit 0) and EP0 (bit 1)
        let ctrl_ctx = input_ctx_virt as *mut u32;
        ptr::write(ctrl_ctx.add(1), 0b11); // Add flags: Slot (bit 0) + EP0 (bit 1)

        // Slot Context:
        let slot_ctx = input_ctx_virt.add(context_size) as *mut u32;
        // Dword 0: Route String (0), Speed (target_speed << 20), Context Entries (4 << 27)
        ptr::write(slot_ctx, ((target_speed as u32) << 20) | (4 << 27));
        // Dword 1: Root Hub Port Number (target_port << 16)
        ptr::write(slot_ctx.add(1), (target_port as u32) << 16);

        // EP0 Context:
        let ep0_ctx = input_ctx_virt.add(context_size * 2) as *mut u32;
        // Dword 1: EP Type = 4 (Control bidirectional), Max Packet Size = 8 (or 64 for HighSpeed)
        let mps = if target_speed == 3 { 64u32 } else { 8u32 };
        ptr::write(ep0_ctx.add(1), (4 << 3) | (mps << 16) | (3 << 1)); // CErr=3
        // Dwords 2 & 3: TR Dequeue Pointer | DCS (bit 0 = 1)
        ptr::write(ep0_ctx.add(2), (ep0_ring_phys as u32) | 1);
        ptr::write(ep0_ctx.add(3), (ep0_ring_phys >> 32) as u32);
        // Dword 4: Average TRB Length = 8
        ptr::write(ep0_ctx.add(4), 8);

        clean_cache(input_ctx_virt as usize, 1024);

        if !send_address_device_cmd(slot_id, input_ctx_phys) {
            serial_println!("[xHCI] Address Device failed.");
            return;
        }
        serial_println!("[xHCI] Addressed Device on Slot {}", slot_id);

        // 11. Configure EP1 IN (Interrupt IN for Keyboard)
        ptr::write_bytes(input_ctx_virt, 0, 1024);
        ptr::write(ctrl_ctx.add(1), (1 << 0) | (1 << 3)); // Add Slot + EP1 IN (DCI 3)

        // Slot Context: Context Entries = 4
        ptr::write(slot_ctx, ((target_speed as u32) << 20) | (4 << 27));
        ptr::write(slot_ctx.add(1), (target_port as u32) << 16);

        // EP1 IN Context (DCI 3 = index 3 in 1-based DCI, context offset = index 4):
        let ep1_ctx = input_ctx_virt.add(context_size * 4) as *mut u32;
        // Dword 0: Interval = 3 (8ms)
        ptr::write(ep1_ctx, 3 << 16);
        // Dword 1: EP Type = 7 (Interrupt IN), Max Packet Size = 8, CErr = 3
        ptr::write(ep1_ctx.add(1), (7 << 3) | (8 << 16) | (3 << 1));
        // Dwords 2 & 3: TR Dequeue Pointer | DCS (bit 0 = 1)
        ptr::write(ep1_ctx.add(2), (ep1_ring_phys as u32) | 1);
        ptr::write(ep1_ctx.add(3), (ep1_ring_phys >> 32) as u32);
        // Dword 4: Average TRB Length = 8
        ptr::write(ep1_ctx.add(4), 8);

        clean_cache(input_ctx_virt as usize, 1024);

        if !send_configure_endpoint_cmd(slot_id, input_ctx_phys) {
            serial_println!("[xHCI] Configure Endpoint failed.");
            return;
        }
        serial_println!("[xHCI] Configured Endpoint 1 IN for Keyboard!");

        // 12. Set Boot Protocol: USB HID Class Request SET_PROTOCOL(0) via EP0
        // Setup packet: bmRequestType=0x21, bRequest=0x0B, wValue=0, wIndex=0, wLength=0
        send_ep0_control_transfer(slot_id, 0x21, 0x0B, 0, 0, 0, 0);

        // 13. Queue Initial Normal TRB for Keyboard Input
        queue_keyboard_trb();

        XHCI.kbd_ready = true;
        XHCI.initialized = true;
        serial_println!("[xHCI] USB Keyboard driver active and polling!");
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

unsafe fn send_ep0_control_transfer(slot_id: u8, req_type: u8, request: u8, val: u16, idx: u16, len: u16, data_phys: usize) {
    let setup_param = (req_type as u64) | ((request as u64) << 8) | ((val as u64) << 16) | ((idx as u64) << 32) | ((len as u64) << 48);

    // Setup Stage TRB
    let trb0 = &mut *XHCI.ep0_ring_virt.add(XHCI.ep0_enqueue_idx);
    trb0.parameter = setup_param;
    trb0.status = 8;
    trb0.control = (2 << 10) | (1 << 6) | XHCI.ep0_cycle; // Type 2 (Setup), IDT=1
    clean_cache(trb0 as *const _ as usize, 16);
    advance_ep0_enqueue();

    // Status Stage TRB
    let trb1 = &mut *XHCI.ep0_ring_virt.add(XHCI.ep0_enqueue_idx);
    trb1.parameter = 0;
    trb1.status = 0;
    trb1.control = (4 << 10) | (1 << 16) | (1 << 5) | XHCI.ep0_cycle; // Type 4 (Status), IN, IOC=1
    clean_cache(trb1 as *const _ as usize, 16);
    advance_ep0_enqueue();

    // Ring Doorbell: slot_id with Target = 1 (EP0)
    ring_doorbell(slot_id, 1);
}

unsafe fn queue_keyboard_trb() {
    let trb = &mut *XHCI.ep1_ring_virt.add(XHCI.ep1_enqueue_idx);
    trb.parameter = XHCI.report_buf_phys as u64;
    trb.status = 8;
    trb.control = (1 << 10) | (1 << 5) | (1 << 2) | XHCI.ep1_cycle; // Type 1 (Normal), IOC=1, ISP=1
    clean_cache(trb as *const _ as usize, 16);

    advance_ep1_enqueue();
    ring_doorbell(XHCI.kbd_slot, 3); // Target 3 = EP1 IN (DCI 3)
}

// ─────────────────────────────────────────────────────────────
// Ring & Event Processing
// ─────────────────────────────────────────────────────────────

unsafe fn advance_cmd_enqueue() {
    XHCI.cmd_enqueue_idx += 1;
    if XHCI.cmd_enqueue_idx == 63 {
        // Link TRB
        let link = &mut *XHCI.cmd_ring_virt.add(63);
        link.control = (6 << 10) | (1 << 1) | XHCI.cmd_cycle;
        clean_cache(link as *const _ as usize, 16);
        XHCI.cmd_cycle ^= 1;
        XHCI.cmd_enqueue_idx = 0;
    }
}

unsafe fn advance_ep0_enqueue() {
    XHCI.ep0_enqueue_idx += 1;
    if XHCI.ep0_enqueue_idx == 15 {
        let link = &mut *XHCI.ep0_ring_virt.add(15);
        link.control = (6 << 10) | (1 << 1) | XHCI.ep0_cycle;
        clean_cache(link as *const _ as usize, 16);
        XHCI.ep0_cycle ^= 1;
        XHCI.ep0_enqueue_idx = 0;
    }
}

unsafe fn advance_ep1_enqueue() {
    XHCI.ep1_enqueue_idx += 1;
    if XHCI.ep1_enqueue_idx == 15 {
        let link = &mut *XHCI.ep1_ring_virt.add(15);
        link.control = (6 << 10) | (1 << 1) | XHCI.ep1_cycle;
        clean_cache(link as *const _ as usize, 16);
        XHCI.ep1_cycle ^= 1;
        XHCI.ep1_enqueue_idx = 0;
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
            serial_println!("[xHCI Event] type={}, cc={}, slot={}", trb_type, completion_code, slot_id);

            // Advance Dequeue
            advance_event_dequeue();

            if trb_type == 33 { // Command Completion Event
                return Some((completion_code, slot_id));
            }
        }
        core::hint::spin_loop();
    }
    None
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

pub fn poll_keyboard() -> Option<u8> {
    unsafe {
        if !XHCI.initialized || !XHCI.kbd_ready { return None; }

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

        if trb_type == 32 && slot_id == XHCI.kbd_slot && ep_id == 3 {
            // Invalidate cache before reading report
            core::arch::asm!("dc ivac, {}", in(reg) XHCI.report_buf_virt);
            core::arch::asm!("dsb ish");

            let modifier = *XHCI.report_buf_virt;
            let keycode = *XHCI.report_buf_virt.add(2);

            // Re-arm keyboard transfer immediately
            queue_keyboard_trb();

            if keycode != 0 && keycode != XHCI.prev_keycode {
                XHCI.prev_keycode = keycode;
                return hid_to_ascii(modifier, keycode);
            } else if keycode == 0 {
                XHCI.prev_keycode = 0;
            }
        }
        None
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
        0x2A => Some(0x08),  // Backspace
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
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────
// Hardware Scanning
// ─────────────────────────────────────────────────────────────

fn find_xhci_base() -> usize {
    // 1. Scan PCIe ECAM if available
    let ecam_base = unsafe { crate::hal::acpi::ACPI_INFO.ecam_base };
    if let Some(ecam) = ecam_base {
        for bus in 0..4 {
            for dev in 0..32 {
                for func in 0..8 {
                    let dev_addr = ecam + ((bus << 20) | (dev << 15) | (func << 12));
                    unsafe {
                        map_device_page(dev_addr, dev_addr);
                        let id_reg = ptr::read_volatile(dev_addr as *const u32);
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
                            ptr::write_volatile((dev_addr + 4) as *mut u16, cmd | 0b110);

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

    // 2. Parallels Desktop Apple Silicon Known Physical MMIO Base
    0x0217_0000
}
