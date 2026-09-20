#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_const)]

extern crate alloc;

mod graphics;


pub mod hal;
pub mod mm;
pub mod sys;
pub mod sysmem;
pub mod fs;
pub mod net;
pub mod font;

use limine::request::{FramebufferRequest, DtbRequest, ModulesRequest, StackSizeRequest, MpRequest};
use limine::BaseRevision;

#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".requests"]
pub static STACK_SIZE_REQUEST: StackSizeRequest = StackSizeRequest::new(1024 * 1024);

#[used]
#[link_section = ".requests"]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
pub static DTB_REQUEST: DtbRequest = DtbRequest::new();

#[used]
#[link_section = ".requests"]
pub static MODULE_REQUEST: ModulesRequest = ModulesRequest::new();

#[used]
#[link_section = ".requests"]
pub static MP_REQUEST: MpRequest = MpRequest::new(0);

#[derive(Copy, Clone)]
pub struct FbInfo {
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u64,
    pub addr: u64,
}

pub static mut FB_INFO: FbInfo = FbInfo { width: 0, height: 0, pitch: 0, bpp: 0, addr: 0 };

pub mod align;

pub static mut TERM_Y: usize = 10;

pub fn log_step(msg: &str, status: Option<&str>) {
    unsafe {
        if FB_INFO.addr != 0 {
            if TERM_Y + 14 >= FB_INFO.height as usize {
                return;
            }
            let text_color = graphics::Color { r: 200, g: 200, b: 200, a: 255 };
            let ok_color = graphics::Color { r: 50, g: 255, b: 50, a: 255 };
            let fail_color = graphics::Color { r: 255, g: 50, b: 50, a: 255 };
            if let Some(st) = status {
                let col = if st == "OK" { ok_color } else { fail_color };
                graphics::draw_string(&FB_INFO, FB_INFO.addr as *mut u8, 10, TERM_Y, "[", text_color, 1);
                graphics::draw_string(&FB_INFO, FB_INFO.addr as *mut u8, 18, TERM_Y, st, col, 1);
                graphics::draw_string(&FB_INFO, FB_INFO.addr as *mut u8, 38, TERM_Y, "]", text_color, 1);
                graphics::draw_string(&FB_INFO, FB_INFO.addr as *mut u8, 50, TERM_Y, msg, text_color, 1);
            } else {
                graphics::draw_string(&FB_INFO, FB_INFO.addr as *mut u8, 10, TERM_Y, msg, text_color, 1);
            }
            let row_bytes = (FB_INFO.pitch * 10) as usize;
            let start = TERM_Y * FB_INFO.pitch as usize;
            graphics::clean_dcache_range((FB_INFO.addr as usize) + start, row_bytes);
            TERM_Y += 12;
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    crate::serial_println!("[KERNEL PANIC] {:?}", info);
    log_step("KERNEL PANIC!", Some("FAIL"));
    if let Some(loc) = info.location() {
        log_step(loc.file(), None);
    }
    loop {
        unsafe { core::arch::asm!("wfe"); }
    }
}

// ─────────────────────────────────────────────────────────────
// Color palette (Catppuccin Mocha – the same one real Hyprland
// rice screenshots use on r/unixporn)
// ─────────────────────────────────────────────────────────────
const C_BASE:    graphics::Color = graphics::Color { r: 24,  g: 24,  b: 37,  a: 255 }; // #181825
const C_MANTLE:  graphics::Color = graphics::Color { r: 17,  g: 17,  b: 27,  a: 255 }; // #11111b
const C_SURFACE: graphics::Color = graphics::Color { r: 49,  g: 50,  b: 68,  a: 255 }; // #313244
const C_OVERLAY: graphics::Color = graphics::Color { r: 108, g: 112, b: 134, a: 255 }; // #6c7086
const C_TEXT:    graphics::Color = graphics::Color { r: 205, g: 214, b: 244, a: 255 }; // #cdd6f4
const C_SUBTEXT: graphics::Color = graphics::Color { r: 166, g: 173, b: 200, a: 255 }; // #a6adc8
const C_PINK:    graphics::Color = graphics::Color { r: 245, g: 194, b: 231, a: 255 }; // #f5c2e7
const C_MAUVE:   graphics::Color = graphics::Color { r: 203, g: 166, b: 247, a: 255 }; // #cba6f7
const C_BLUE:    graphics::Color = graphics::Color { r: 137, g: 180, b: 250, a: 255 }; // #89b4fa
const C_GREEN:   graphics::Color = graphics::Color { r: 166, g: 227, b: 161, a: 255 }; // #a6e3a1
const C_YELLOW:  graphics::Color = graphics::Color { r: 249, g: 226, b: 175, a: 255 }; // #f9e2af
const C_RED:     graphics::Color = graphics::Color { r: 243, g: 139, b: 168, a: 255 }; // #f38ba8
const C_TEAL:    graphics::Color = graphics::Color { r: 148, g: 226, b: 213, a: 255 }; // #94e2d5
const C_BLACK:   graphics::Color = graphics::Color { r: 0,   g: 0,   b: 0,   a: 180 }; // shadow

// Border gradient: Mauve → Blue
const ACTIVE_BORDER: graphics::Border = graphics::Border {
    width: 3,
    color_tl: C_MAUVE,
    color_br: C_BLUE,
};
const INACTIVE_BORDER: graphics::Border = graphics::Border {
    width: 2,
    color_tl: C_SURFACE,
    color_br: C_SURFACE,
};

#[inline(always)]
fn s(fb: &FbInfo, buf: *mut u8, x: usize, y: usize, text: &str, col: graphics::Color, scale: usize) {
    graphics::draw_string(fb, buf, x, y, text, col, scale);
}

#[repr(C, align(65536))]
pub struct BootKernelStack {
    pub stack: [u8; 1024 * 1024],
    pub guard: [u8; 65536],
}

#[used]
#[no_mangle]
pub static mut BOOT_STACK: BootKernelStack = BootKernelStack {
    stack: [0; 1024 * 1024],
    guard: [0; 65536],
};

#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "msr spsel, #1",
        "ldr x0, =BOOT_STACK",
        "mov x1, 0x100000",
        "add x0, x0, x1",
        "mov sp, x0",
        "msr sp_el0, x0",
        "b kernel_main"
    );
}

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // Enable FP/SIMD in CPACR_EL1 (Bits 20:21 = 0b11)
    unsafe {
        let mut cpacr: u64;
        core::arch::asm!("mrs {}, cpacr_el1", out(reg) cpacr);
        cpacr |= (3 << 20);
        core::arch::asm!("msr cpacr_el1, {}", in(reg) cpacr);
        core::arch::asm!("isb");
    }

    let mut fb_addr: *mut u8 = core::ptr::null_mut();
    let mut pitch = 0;
    
    if let Some(response) = FRAMEBUFFER_REQUEST.response() {
        if let Some(fb) = response.framebuffers().first() {
            fb_addr = fb.address() as *mut u8;
            pitch = fb.pitch as usize;
            unsafe {
                FB_INFO.width  = fb.width;
                FB_INFO.height = fb.height;
                FB_INFO.pitch  = fb.pitch;
                FB_INFO.bpp    = fb.bpp as u64;
                FB_INFO.addr   = fb_addr as u64;
            }
        }
    }

    // 1. CLEAR SCREEN (Black Terminal)
    let mut dtb_ptr = 0;
    if let Some(resp) = DTB_REQUEST.response() {
        dtb_ptr = resp.dtb_ptr as usize;
    }
    if !fb_addr.is_null() {
        unsafe {
            graphics::draw_background(&FB_INFO, fb_addr, 
                graphics::Color { r: 0, g: 0, b: 0, a: 255 }, 
                graphics::Color { r: 0, g: 0, b: 0, a: 255 }
            );
            graphics::clean_dcache_range(fb_addr as usize, (FB_INFO.height * FB_INFO.pitch) as usize);
        }
    }

    // A simple terminal logger
    let mut term_y = 10;
    let text_color = graphics::Color { r: 200, g: 200, b: 200, a: 255 };
    let ok_color = graphics::Color { r: 50, g: 255, b: 50, a: 255 };

    log_step("Booting HimadaOS (AArch64) Terminal Server...", None);
    log_step("Initializing Hardware Abstraction Layer (HAL)...", None);
    hal::init();
    log_step("HAL initialized successfully.", Some("OK"));

    log_step("Initializing Memory Manager (MM)...", None);
    mm::init();
    crate::hal::acpi::init();
    crate::hal::serial::init(dtb_ptr as usize);
    crate::hal::xhci::init();
    // KMI (PL050 PS/2) init is deferred — only enabled when confirmed via ACPI MADT
    // crate::hal::kmi::init();
    if dtb_ptr != 0 {
        log_step("FDT Device Tree available.", Some("OK"));
    } else {
        log_step("Running under ACPI / Hypervisor environment.", None);
    }
    log_step("Physical and Virtual Memory Maps initialized.", Some("OK"));
    
    crate::sys::gpu::init(dtb_ptr as usize);
    crate::hal::device::init(dtb_ptr as usize);
    log_step("KMI, xHCI & Devices Initialized.", Some("OK"));
    crate::net::socket::init();
    crate::net::dhcp::init();
    log_step("TCP/IP Stack Initialized.", Some("OK"));

    log_step("Initializing System Subsystems (Linux ABI)...", None);
    sys::init();
    log_step("Linux ABI translation layer active.", Some("OK"));
    
    // Look for initramfs and ELF payload
    if let Some(mod_req) = MODULE_REQUEST.response() {
        log_step("Found modules from bootloader.", Some("OK"));
        let mut payload_slice = None;
        
        for (i, module) in mod_req.modules().iter().enumerate() {
            let slice = module.data();
            if i == 0 {
                // Initramfs
                log_step("Parsing initramfs...", None);
                crate::fs::vfs::init();
                crate::fs::devfs::init(); // Initialize /dev/null and /dev/zero
                crate::fs::procfs::init(); // Initialize /proc filesystem
                crate::fs::sysfs::init(); // Initialize /sys filesystem
                crate::fs::cpio::parse(slice);
                log_step("VFS initialized with /dev, /proc and /sys trees.", Some("OK"));
            } else if i == 1 {
                payload_slice = Some(slice);
            }
        }
        
        // Multi-Core SMP Bringup after VFS, drivers, and network are initialized
        crate::hal::smp::init();

        if let Some(slice) = payload_slice {
            log_step("Attempting to load ELF payload...", None);
            crate::sys::elf::load_and_run(slice);
        }
    } else {
        crate::hal::smp::init();
        log_step("No modules found by bootloader.", None);
    }

    log_step("System ready.", None);
    log_step("himada-os login: _", None);

    // Halt the kernel
    loop { unsafe { core::arch::asm!("wfe"); } }
}
