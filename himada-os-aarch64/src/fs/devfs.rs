use crate::fs::vfs::{FileOps, NodeKind, VfsNode};
use crate::hal::device::DEVICE_MANAGER;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

struct DevNull;
impl FileOps for DevNull {
    fn read(&self, _offset: usize, _buf: &mut [u8]) -> usize { 0 }
    fn write(&self, _offset: usize, buf: &[u8]) -> usize { buf.len() }
}

struct DevZero;
impl FileOps for DevZero {
    fn read(&self, _offset: usize, buf: &mut [u8]) -> usize {
        unsafe {
            crate::sysmem::page_zero(buf.as_mut_ptr(), buf.len());
        }
        buf.len()
    }
    fn write(&self, _offset: usize, buf: &[u8]) -> usize { buf.len() }
}

struct DevUrandom;
impl FileOps for DevUrandom {
    fn read(&self, _offset: usize, buf: &mut [u8]) -> usize {
        let mut seed: u64 = 0;
        unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) seed); }
        for b in buf.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *b = (seed >> 33) as u8;
        }
        buf.len()
    }
    fn write(&self, _offset: usize, buf: &[u8]) -> usize { buf.len() }
}

pub struct VirtioBlkFileOps {
    pub blk: Arc<spin::Mutex<dyn crate::hal::device::BlockDevice>>,
}

impl FileOps for VirtioBlkFileOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let dev = self.blk.lock();
        let lba = (offset / 512) as u64;
        let sector_offset = offset % 512;
        let sectors_needed = ((sector_offset + buf.len() + 511) / 512) as u16;

        let mut temp_buf = alloc::vec![0u8; (sectors_needed as usize) * 512];
        if dev.read_sectors(lba, sectors_needed, &mut temp_buf).is_ok() {
            let to_copy = buf.len().min(temp_buf.len() - sector_offset);
            buf[..to_copy].copy_from_slice(&temp_buf[sector_offset..sector_offset + to_copy]);
            to_copy
        } else {
            0
        }
    }

    fn write(&self, offset: usize, buf: &[u8]) -> usize {
        let dev = self.blk.lock();
        let lba = (offset / 512) as u64;
        let sector_offset = offset % 512;
        let sectors_needed = ((sector_offset + buf.len() + 511) / 512) as u16;

        let mut temp_buf = alloc::vec![0u8; (sectors_needed as usize) * 512];
        let _ = dev.read_sectors(lba, sectors_needed, &mut temp_buf);

        let to_copy = buf.len().min(temp_buf.len() - sector_offset);
        temp_buf[sector_offset..sector_offset + to_copy].copy_from_slice(&buf[..to_copy]);

        if dev.write_sectors(lba, sectors_needed, &temp_buf).is_ok() {
            to_copy
        } else {
            0
        }
    }
}

pub fn init() {
    let dev_null = Arc::new(VfsNode {
        name: "null".to_string(),
        kind: NodeKind::CharDevice,
        size: 0,
        children: RwLock::new(Vec::new()),
        file_ops: Some(Arc::new(DevNull)),
        data: RwLock::new(Vec::new()),
        static_data: None,
        uid: RwLock::new(0),
        gid: RwLock::new(0),
        mode: RwLock::new(0o666),
    });
    
    let dev_zero = Arc::new(VfsNode {
        name: "zero".to_string(),
        kind: NodeKind::CharDevice,
        size: 0,
        children: RwLock::new(Vec::new()),
        file_ops: Some(Arc::new(DevZero)),
        data: RwLock::new(Vec::new()),
        static_data: None,
        uid: RwLock::new(0),
        gid: RwLock::new(0),
        mode: RwLock::new(0o666),
    });

    let dev_urandom = Arc::new(VfsNode {
        name: "urandom".to_string(),
        kind: NodeKind::CharDevice,
        size: 0,
        children: RwLock::new(Vec::new()),
        file_ops: Some(Arc::new(DevUrandom)),
        data: RwLock::new(Vec::new()),
        static_data: None,
        uid: RwLock::new(0),
        gid: RwLock::new(0),
        mode: RwLock::new(0o666),
    });
    
    crate::fs::vfs::add_node("dev/null", dev_null);
    crate::fs::vfs::add_node("dev/zero", dev_zero);
    crate::fs::vfs::add_node("dev/urandom", dev_urandom);

    // Register VirtIO Block Devices (/dev/vda, /dev/vda1)
    let dm = DEVICE_MANAGER.lock();
    if let Some(blk) = dm.block_devices.first().cloned() {
        let capacity = blk.lock().capacity() as usize;

        let dev_vda = Arc::new(VfsNode {
            name: "vda".to_string(),
            kind: NodeKind::BlockDevice(blk.clone()),
            size: capacity,
            children: RwLock::new(Vec::new()),
            file_ops: Some(Arc::new(VirtioBlkFileOps { blk: blk.clone() })),
            data: RwLock::new(Vec::new()),
            static_data: None,
            uid: RwLock::new(0),
            gid: RwLock::new(6), // disk group
            mode: RwLock::new(0o660),
        });

        let dev_vda1 = Arc::new(VfsNode {
            name: "vda1".to_string(),
            kind: NodeKind::BlockDevice(blk.clone()),
            size: capacity,
            children: RwLock::new(Vec::new()),
            file_ops: Some(Arc::new(VirtioBlkFileOps { blk: blk.clone() })),
            data: RwLock::new(Vec::new()),
            static_data: None,
            uid: RwLock::new(0),
            gid: RwLock::new(6),
            mode: RwLock::new(0o660),
        });

        crate::fs::vfs::add_node("dev/vda", dev_vda);
        crate::fs::vfs::add_node("dev/vda1", dev_vda1);
        crate::serial_println!("[devfs] Registered /dev/vda and /dev/vda1 ({} MB)", capacity / (1024 * 1024));
    }
    drop(dm);

    // Initialize loop devices (/dev/loop0..3)
    crate::fs::loop_dev::init();
}
