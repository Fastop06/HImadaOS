use alloc::sync::Arc;
use spin::Mutex;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::mmio::MmioTransport;
use crate::hal::virtio::VirtioHal;
use crate::hal::device::{BlockDevice, DEVICE_MANAGER};

pub struct VirtIOBlkWrapper<'a> {
    pub inner: Mutex<VirtIOBlk<VirtioHal, MmioTransport<'a>>>,
}

impl<'a> BlockDevice for VirtIOBlkWrapper<'a> {
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
                crate::serial_println!("[virtio_blk] read_blocks(lba={}, len={}) failed: {:?}", lba, buf.len(), e);
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
                crate::serial_println!("[virtio_blk] write_blocks(lba={}, len={}) failed: {:?}", lba, buf.len(), e);
                Err("VirtIO Blk Write failed")
            }
        }
    }

    fn capacity(&self) -> u64 {
        let inner = self.inner.lock();
        inner.capacity() * 512
    }
}

pub fn try_init(transport: MmioTransport<'static>) {
    if let Ok(blk) = VirtIOBlk::<VirtioHal, MmioTransport>::new(transport) {
        let capacity = blk.capacity();
        let wrapper = VirtIOBlkWrapper { inner: Mutex::new(blk) };
        DEVICE_MANAGER.lock().block_devices.push(Arc::new(Mutex::new(wrapper)));
        crate::serial_println!("[Device Manager] Registered VirtIO Block Device! Capacity: {} bytes", capacity * 512);
    }
}
