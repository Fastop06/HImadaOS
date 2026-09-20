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

pub fn init(fdt_vaddr: usize) {
    crate::serial_println!("[Device Manager] Initializing Subsystem...");
    
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
