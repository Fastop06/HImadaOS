extern crate alloc;
use virtio_drivers::{Hal, BufferDirection, PhysAddr};
use core::ptr::NonNull;
use crate::mm::{pmm, vmm};
use spin::Mutex;
use virtio_drivers::transport::pci::{bus::{ConfigurationAccess, DeviceFunction, PciRoot}, PciTransport};
use virtio_drivers::transport::Transport;
use virtio_drivers::device::gpu::VirtIOGpu;
use alloc::vec::Vec;
use fdt::Fdt;

use crate::hal::virtio::VirtioHal;

// ======================= PCI ECAM =======================

pub struct Ecam {
    base: usize,
}

impl Ecam {
    pub const fn new(base: usize) -> Self {
        Self { base }
    }
    
    pub fn addr_paddr(&self, df: DeviceFunction, offset: u8) -> usize {
        self.base 
            + ((df.bus as usize) << 20) 
            + ((df.device as usize) << 15) 
            + ((df.function as usize) << 12) 
            + (offset as usize & !3)
    }

    fn addr(&self, df: DeviceFunction, offset: u8) -> *mut u32 {
        let addr = self.addr_paddr(df, offset);
        let page = addr & !0xFFF;
        unsafe {
            crate::mm::vmm::map_device_page(page, page);
        }
        addr as *mut u32
    }
}

impl ConfigurationAccess for Ecam {
    fn read_word(&self, device_function: DeviceFunction, register_offset: u8) -> u32 {
        let paddr = self.addr_paddr(device_function, register_offset);
        let page = paddr & !0xFFF;
        unsafe {
            crate::mm::vmm::map_device_page(page, page);
            crate::hal::exceptions::safe_read_u32(paddr).unwrap_or(0xFFFF_FFFF)
        }
    }

    fn write_word(&mut self, device_function: DeviceFunction, register_offset: u8, data: u32) {
        let paddr = self.addr_paddr(device_function, register_offset);
        let page = paddr & !0xFFF;
        unsafe {
            crate::mm::vmm::map_device_page(page, page);
            crate::hal::exceptions::safe_write_u32(paddr, data);
        }
    }

    unsafe fn unsafe_clone(&self) -> Self {
        Ecam::new(self.base)
    }
}

// ======================= GPU STATE =======================

pub static GPU: Mutex<Option<VirtIOGpu<VirtioHal, PciTransport>>> = Mutex::new(None);
pub static GPU_FB_ADDR: Mutex<Option<usize>> = Mutex::new(None);

pub fn init(fdt_vaddr: usize) {
    if fdt_vaddr == 0 {
        return;
    }
    crate::serial_println!("[GPU] FDT VAddr: {:#X}", fdt_vaddr);
    let vaddr = fdt_vaddr as *const u8;
    
    let fdt = match unsafe { Fdt::from_ptr(vaddr) } {
        Ok(f) => f,
        Err(_) => {
            crate::serial_println!("[GPU] Failed to parse FDT!");
            return;
        }
    };
    
    let mut ecam_base: Option<usize> = None;
    for node in fdt.all_nodes() {
        if let Some(prop) = node.property("device_type") {
            if prop.as_str() == Some("pci") {
                if let Some(reg) = node.property("reg") {
                    let mut addr: u64 = 0;
                    if reg.value.len() >= 8 {
                        addr = u64::from_be_bytes(reg.value[0..8].try_into().unwrap());
                        ecam_base = Some(addr as usize);
                        break;
                    }
                }
            }
        }
    }
    
    let ecam_base = match ecam_base {
        Some(base) => base,
        None => return,
    };
    crate::serial_println!("[GPU] PCI ECAM Base: {:#X}", ecam_base);
    
    let ecam = Ecam::new(ecam_base);
    let mut root = PciRoot::new(ecam);
    
    for bus in 0..=255 {
        for device in 0..32 {
            let df = DeviceFunction { bus, device, function: 0 };
            // We use a fresh ecam instead of root.configuration_access
            let vendor = Ecam::new(ecam_base).read_word(df, 0);
            if vendor == 0xFFFFFFFF { continue; }
            
            let vendor_id = (vendor & 0xFFFF) as u16;
            let device_id = (vendor >> 16) as u16;
            
            // Virtio Vendor = 0x1AF4
            if vendor_id == 0x1AF4 && (device_id == 0x1050 || (device_id >= 0x1040 && device_id <= 0x105F)) {
                crate::serial_println!("[GPU] Found Virtio device at {:02x}:{:02x}.0", bus, device);
                
                if let Ok(transport) = PciTransport::new::<VirtioHal, _>(&mut root, df) {
                    if transport.device_type() == virtio_drivers::transport::DeviceType::GPU {
                        crate::serial_println!("[GPU] Transport initialized!");
                        if let Ok(mut gpu) = VirtIOGpu::<VirtioHal, PciTransport>::new(transport) {
                            crate::serial_println!("[GPU] VirtIOGpu instance created!");
                            
                            if let Ok((width, height)) = gpu.resolution() {
                                crate::serial_println!("[GPU] Resolution: {}x{}", width, height);
                                if let Ok(fb) = gpu.setup_framebuffer() {
                                    let fb_ptr = fb.as_mut_ptr();
                                    *GPU_FB_ADDR.lock() = Some(fb_ptr as usize);
                                    
                                    *GPU.lock() = Some(gpu);
                                    crate::serial_println!("[GPU] Setup complete.");
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    crate::serial_println!("[GPU] Virtio GPU not found on PCI bus.");
}

pub fn flush() {
    if let Some(gpu) = GPU.lock().as_mut() {
        let _ = gpu.flush();
    }
}
