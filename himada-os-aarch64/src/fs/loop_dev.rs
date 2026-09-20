use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, RwLock};
use crate::hal::device::BlockDevice;
use crate::fs::vfs::{FileOps, NodeKind, VfsNode};

pub const LOOP_SET_FD: u64 = 0x4C00;
pub const LOOP_CLR_FD: u64 = 0x4C01;
pub const LOOP_SET_STATUS: u64 = 0x4C02;
pub const LOOP_GET_STATUS: u64 = 0x4C03;
pub const LOOP_SET_STATUS64: u64 = 0x4C04;
pub const LOOP_GET_STATUS64: u64 = 0x4C05;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LoopInfo64 {
    pub lo_device: u64,
    pub lo_inode: u64,
    pub lo_rdevice: u64,
    pub lo_offset: u64,
    pub lo_sizelimit: u64,
    pub lo_number: u32,
    pub lo_encrypt_type: u32,
    pub lo_encrypt_key_size: u32,
    pub lo_flags: u32,
    pub lo_file_name: [u8; 64],
    pub lo_crypt_name: [u8; 64],
    pub lo_encrypt_key: [u8; 32],
    pub lo_init: [u64; 2],
}

pub struct LoopDevice {
    pub id: usize,
    pub backing_node: Option<Arc<VfsNode>>,
    pub offset: u64,
    pub sizelimit: u64,
    pub flags: u32,
}

impl LoopDevice {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            backing_node: None,
            offset: 0,
            sizelimit: 0,
            flags: 0,
        }
    }
}

impl BlockDevice for LoopDevice {
    fn read_sectors(&self, lba: u64, count: u16, buf: &mut [u8]) -> Result<(), &'static str> {
        let node = match self.backing_node {
            Some(ref n) => n,
            None => return Err("Loop device has no backing file"),
        };

        let file_offset = (self.offset + lba * 512) as usize;
        let total_bytes = (count as usize) * 512;
        let buf_len = buf.len();
        let buf_slice = &mut buf[..total_bytes.min(buf_len)];

        if let Some(ref ops) = node.file_ops {
            let read = ops.read(file_offset, buf_slice);
            if read < buf_slice.len() {
                buf_slice[read..].fill(0);
            }
            return Ok(());
        }

        if let Some(data) = node.static_data {
            let remaining = data.len().saturating_sub(file_offset);
            let to_read = buf_slice.len().min(remaining);
            if to_read > 0 {
                buf_slice[..to_read].copy_from_slice(&data[file_offset..file_offset + to_read]);
            }
            if to_read < buf_slice.len() {
                buf_slice[to_read..].fill(0);
            }
            return Ok(());
        }

        let data = node.data.read();
        let remaining = data.len().saturating_sub(file_offset);
        let to_read = buf_slice.len().min(remaining);
        if to_read > 0 {
            buf_slice[..to_read].copy_from_slice(&data[file_offset..file_offset + to_read]);
        }
        if to_read < buf_slice.len() {
            buf_slice[to_read..].fill(0);
        }
        Ok(())
    }

    fn write_sectors(&self, lba: u64, count: u16, buf: &[u8]) -> Result<(), &'static str> {
        let node = match self.backing_node {
            Some(ref n) => n,
            None => return Err("Loop device has no backing file"),
        };

        let file_offset = (self.offset + lba * 512) as usize;
        let total_bytes = (count as usize) * 512;
        let buf_slice = &buf[..total_bytes.min(buf.len())];

        if let Some(ref ops) = node.file_ops {
            ops.write(file_offset, buf_slice);
            return Ok(());
        }

        let mut data = node.data.write();
        let end = file_offset + buf_slice.len();
        if data.len() < end {
            data.resize(end, 0);
        }
        data[file_offset..end].copy_from_slice(buf_slice);
        Ok(())
    }

    fn capacity(&self) -> u64 {
        if let Some(ref node) = self.backing_node {
            if self.sizelimit > 0 {
                return self.sizelimit;
            }
            let data_len = node.data.read().len() as u64;
            if data_len > 0 {
                return data_len.saturating_sub(self.offset);
            }
            if let Some(data) = node.static_data {
                return (data.len() as u64).saturating_sub(self.offset);
            }
            (node.size as u64).saturating_sub(self.offset)
        } else {
            0
        }
    }
}

lazy_static::lazy_static! {
    pub static ref LOOP_DEVICES: [Arc<Mutex<LoopDevice>>; 4] = [
        Arc::new(Mutex::new(LoopDevice::new(0))),
        Arc::new(Mutex::new(LoopDevice::new(1))),
        Arc::new(Mutex::new(LoopDevice::new(2))),
        Arc::new(Mutex::new(LoopDevice::new(3))),
    ];
}

pub struct LoopFileOps {
    pub dev: Arc<Mutex<LoopDevice>>,
}

impl FileOps for LoopFileOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let dev = self.dev.lock();
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
        let dev = self.dev.lock();
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
    for (i, dev) in LOOP_DEVICES.iter().enumerate() {
        let name = alloc::format!("loop{}", i);
        let path = alloc::format!("dev/{}", name);

        let vfs_node = Arc::new(VfsNode {
            name,
            kind: NodeKind::BlockDevice(dev.clone()),
            size: 0,
            children: RwLock::new(Vec::new()),
            file_ops: Some(Arc::new(LoopFileOps { dev: dev.clone() })),
            data: RwLock::new(Vec::new()),
            static_data: None,
            uid: RwLock::new(0),
            gid: RwLock::new(6), // disk group
            mode: RwLock::new(0o660),
        });

        crate::fs::vfs::add_node(&path, vfs_node);
    }
    crate::serial_println!("[LoopDev] Initialized 4 loop block devices (/dev/loop0..3)");
}
