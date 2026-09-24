use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use core::fmt;
use fdt::Fdt;
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
use virtio_drivers::transport::{DeviceType, Transport};
use virtio_drivers::device::net::VirtIONet;
use virtio_drivers::device::blk::VirtIOBlk;
use crate::hal::virtio::VirtioHal;
use crate::mm::vmm;
use core::ptr::NonNull;

pub trait BlockDevice: Send + Sync {
    fn read_sectors(&self, lba: u64, count: u16, buf: &mut [u8]) -> Result<(), &'static str>;
    fn write_sectors(&self, lba: u64, count: u16, buf: &[u8]) -> Result<(), &'static str>;
    fn capacity(&self) -> u64;
}

pub trait NetDevice: Send + Sync {
    fn mac_address(&self) -> [u8; 6];
    fn transmit(&mut self, packet: &[u8]) -> Result<(), &'static str>;
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize, &'static str>;
}

pub trait UsbController: Send + Sync {
    fn enumerate(&self) -> Result<(), &'static str>;
}

pub struct DeviceManager {
    pub block_devices: Vec<Arc<Mutex<dyn BlockDevice>>>,
    pub net_devices: Vec<Arc<Mutex<dyn NetDevice>>>,
    pub usb_controllers: Vec<Arc<Mutex<dyn UsbController>>>,
}

lazy_static::lazy_static! {
    pub static ref DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager {
        block_devices: Vec::new(),
        net_devices: Vec::new(),
        usb_controllers: Vec::new(),
    });
}

pub struct VirtIONetWrapper<'a> {
    pub inner: VirtIONet<VirtioHal, MmioTransport<'a>, 32>,
}

impl<'a> NetDevice for VirtIONetWrapper<'a> {
    fn mac_address(&self) -> [u8; 6] {
        self.inner.mac_address()
    }
    fn transmit(&mut self, packet: &[u8]) -> Result<(), &'static str> {
        let mut tx_buf = self.inner.new_tx_buffer(packet.len());
        tx_buf.packet_mut().copy_from_slice(packet);
        if let Err(e) = self.inner.send(tx_buf) {
            crate::serial_println!("[VirtioNet] transmit send error: {:?}", e);
            return Err("send failed");
        }
        Ok(())
    }
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        match self.inner.receive() {
            Ok(rx_buf) => {
                let pkt = rx_buf.packet();
                let len = pkt.len().min(buf.len());
                buf[..len].copy_from_slice(&pkt[..len]);
                let _ = self.inner.recycle_rx_buffer(rx_buf);
                Ok(len)
            }
            Err(_) => Err("Receive failed"),
        }
    }
}

use virtio_drivers::transport::pci::{bus::{ConfigurationAccess, DeviceFunction, PciRoot}, PciTransport};

pub struct VirtIONetPciWrapper {
    pub inner: VirtIONet<VirtioHal, PciTransport, 32>,
}

impl NetDevice for VirtIONetPciWrapper {
    fn mac_address(&self) -> [u8; 6] {
        self.inner.mac_address()
    }
    fn transmit(&mut self, packet: &[u8]) -> Result<(), &'static str> {
        let mut tx_buf = self.inner.new_tx_buffer(packet.len());
        tx_buf.packet_mut().copy_from_slice(packet);
        if let Err(e) = self.inner.send(tx_buf) {
            crate::serial_println!("[VirtioNet-PCI] transmit send error: {:?}", e);
            return Err("send failed");
        }
        Ok(())
    }
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        match self.inner.receive() {
            Ok(rx_buf) => {
                let pkt = rx_buf.packet();
                let len = pkt.len().min(buf.len());
                buf[..len].copy_from_slice(&pkt[..len]);
                let _ = self.inner.recycle_rx_buffer(rx_buf);
                Ok(len)
            }
            Err(_) => Err("Receive failed"),
        }
    }
}

pub struct VirtIOBlkPciWrapper {
    pub inner: Mutex<VirtIOBlk<VirtioHal, PciTransport>>,
}

impl BlockDevice for VirtIOBlkPciWrapper {
    fn read_sectors(&self, lba: u64, _count: u16, buf: &mut [u8]) -> Result<(), &'static str> {
        let mut inner = self.inner.lock();
        let bounce_len = if buf.len() % 512 == 0 { buf.len() } else { (buf.len() + 511) & !511 };
        let mut bounce = alloc::vec![0u8; bounce_len];
        match inner.read_blocks(lba as usize, &mut bounce) {
            Ok(_) => {
                buf.copy_from_slice(&bounce[..buf.len()]);
                Ok(())
            }
            Err(e) => {
                crate::serial_println!("[virtio_blk_pci] read_blocks failed: {:?}", e);
                Err("VirtIO Blk Read failed")
            }
        }
    }

    fn write_sectors(&self, lba: u64, _count: u16, buf: &[u8]) -> Result<(), &'static str> {
        let mut inner = self.inner.lock();
        let bounce_len = if buf.len() % 512 == 0 { buf.len() } else { (buf.len() + 511) & !511 };
        let mut bounce = alloc::vec![0u8; bounce_len];
        bounce[..buf.len()].copy_from_slice(buf);
        match inner.write_blocks(lba as usize, &bounce) {
            Ok(_) => Ok(()),
            Err(e) => {
                crate::serial_println!("[virtio_blk_pci] write_blocks failed: {:?}", e);
                Err("VirtIO Blk Write failed")
            }
        }
    }

    fn capacity(&self) -> u64 {
        let inner = self.inner.lock();
        inner.capacity() * 512
    }
}

pub fn probe_pci_bus(ecam_base: usize) -> bool {
    let ecam = crate::sys::gpu::Ecam::new(ecam_base);
    let mut root = PciRoot::new(ecam);
    let mut found = false;

    for bus in 0..=3 {
        for device in 0..32 {
            for function in 0..8 {
                let df = DeviceFunction { bus, device, function };
                let vendor = crate::sys::gpu::Ecam::new(ecam_base).read_word(df, 0);
                if vendor == 0xFFFF_FFFF || vendor == 0 {
                    if function == 0 { break; }
                    continue;
                }

                let vendor_id = (vendor & 0xFFFF) as u16;
                let device_id = (vendor >> 16) as u16;

                if vendor_id == 0x1AF4 {
                    // Enable Memory Space (bit 1) and Bus Master (bit 2) in PCI Command Register
                    let mut ecam_cmd = crate::sys::gpu::Ecam::new(ecam_base);
                    let cmd_stat = ecam_cmd.read_word(df, 4);
                    ecam_cmd.write_word(df, 4, cmd_stat | 0b111);

                    if let Ok(transport) = PciTransport::new::<VirtioHal, _>(&mut root, df) {
                        match transport.device_type() {
                            DeviceType::Network => {
                                if DEVICE_MANAGER.lock().net_devices.is_empty() {
                                    if let Ok(net) = VirtIONet::<VirtioHal, PciTransport, 32>::new(transport, 2048) {
                                        let wrapper = VirtIONetPciWrapper { inner: net };
                                        DEVICE_MANAGER.lock().net_devices.push(Arc::new(Mutex::new(wrapper)));
                                        crate::serial_println!("[Device Manager] Registered VirtIO Net (PCI) at {:02x}:{:02x}.{}!", bus, device, function);
                                        found = true;
                                    }
                                }
                            }
                            DeviceType::Block => {
                                if DEVICE_MANAGER.lock().block_devices.is_empty() {
                                    if let Ok(blk) = VirtIOBlk::<VirtioHal, PciTransport>::new(transport) {
                                        let wrapper = VirtIOBlkPciWrapper { inner: Mutex::new(blk) };
                                        DEVICE_MANAGER.lock().block_devices.push(Arc::new(Mutex::new(wrapper)));
                                        crate::serial_println!("[Device Manager] Registered VirtIO Block (PCI) at {:02x}:{:02x}.{}!", bus, device, function);
                                        found = true;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    found
}

pub fn init(fdt_vaddr: usize) {
    crate::serial_println!("[Device Manager] Initializing Subsystem...");
    
    // 1. Probe PCI buses (VirtIO Net PCI & VirtIO Blk PCI)
    let mut ecam_candidates: Vec<usize> = Vec::new();
    if let Some(ecam) = unsafe { crate::hal::acpi::ACPI_INFO.ecam_base } {
        ecam_candidates.push(ecam);
    }
    if fdt_vaddr != 0 {
        let vaddr = fdt_vaddr as *const u8;
        if let Ok(fdt) = unsafe { Fdt::from_ptr(vaddr) } {
            for node in fdt.all_nodes() {
                if let Some(prop) = node.property("device_type") {
                    if prop.as_str() == Some("pci") {
                        if let Some(reg) = node.property("reg") {
                            if reg.value.len() >= 8 {
                                let addr = u64::from_be_bytes(reg.value[0..8].try_into().unwrap()) as usize;
                                if !ecam_candidates.contains(&addr) {
                                    ecam_candidates.push(addr);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for &fallback in &[0x40_1000_0000usize, 0x3f00_0000usize, 0x1000_0000usize] {
        if !ecam_candidates.contains(&fallback) {
            ecam_candidates.push(fallback);
        }
    }

    for ecam in ecam_candidates {
        if probe_pci_bus(ecam) {
            crate::serial_println!("[Device Manager] VirtIO PCI devices enumerated from ECAM {:#X}", ecam);
            break;
        }
    }

    // 2. Probe MMIO devices if needed
    let mut scanned_fdt = false;
    if fdt_vaddr != 0 {
        let vaddr = fdt_vaddr as *const u8;
        if let Ok(fdt) = unsafe { Fdt::from_ptr(vaddr) } {
            scanned_fdt = true;
            for node in fdt.all_nodes() {
                if let Some(compatible) = node.property("compatible") {
                    if compatible.as_str() == Some("virtio,mmio") {
                        if let Some(reg) = node.property("reg") {
                            if reg.value.len() >= 8 {
                                let addr = u64::from_be_bytes(reg.value[0..8].try_into().unwrap());
                                probe_mmio_slot(addr as usize);
                            }
                        }
                    }
                }
            }
        }
    }

    if !scanned_fdt || DEVICE_MANAGER.lock().net_devices.is_empty() {
        crate::serial_println!("[Device Manager] Probing QEMU VirtIO MMIO slots (0x0a000000..0x0a004000)...");
        for slot in 0..32 {
            let addr = 0x0a00_0000 + slot * 0x200;
            probe_mmio_slot(addr);
        }
    }

    crate::serial_println!("[Device Manager] Scan complete.");
}

fn probe_mmio_slot(addr: usize) {
    unsafe {
        crate::mm::vmm::map_device_page(addr & !0xFFF, addr & !0xFFF);
    }
    let vaddr = addr;
    let magic = unsafe { core::ptr::read_volatile(vaddr as *const u32) };
    if magic != 0x74726976 { // 'virt'
        return;
    }
    let header = match NonNull::new(vaddr as *mut VirtIOHeader) {
        Some(h) => h,
        None => return,
    };
    if let Ok(transport) = unsafe { MmioTransport::new(header, 0x200) } {
        match transport.device_type() {
            DeviceType::Network => {
                if DEVICE_MANAGER.lock().net_devices.is_empty() {
                    if let Ok(net) = VirtIONet::<VirtioHal, MmioTransport, 32>::new(transport, 2048) {
                        let wrapper = VirtIONetWrapper { inner: net };
                        DEVICE_MANAGER.lock().net_devices.push(Arc::new(Mutex::new(wrapper)));
                        crate::serial_println!("[Device Manager] Registered VirtIO Net at 0x{:x}!", addr);
                    }
                }
            }
            DeviceType::Input => {
                crate::hal::virtio_input::try_init(addr);
            }
            DeviceType::Block => {
                crate::hal::virtio_blk::try_init(transport);
            }
            _ => {}
        }
    }
}
