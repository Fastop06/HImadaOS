use crate::fs::vfs::{FileOps, NodeKind, VfsNode};
use alloc::format;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

fn read_str_offset(s: &str, offset: usize, buf: &mut [u8]) -> usize {
    let bytes = s.as_bytes();
    if offset >= bytes.len() {
        return 0;
    }
    let to_copy = (bytes.len() - offset).min(buf.len());
    buf[..to_copy].copy_from_slice(&bytes[offset..offset + to_copy]);
    to_copy
}

struct Eth0AddressOps;
impl FileOps for Eth0AddressOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let dev_mgr = crate::hal::device::DEVICE_MANAGER.lock();
        let mac_str = if let Some(net_dev) = dev_mgr.net_devices.first() {
            let m = net_dev.lock().mac_address();
            format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}\n", m[0], m[1], m[2], m[3], m[4], m[5])
        } else {
            format!("52:54:00:12:34:56\n")
        };
        read_str_offset(&mac_str, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct StaticStringOps(&'static str);
impl FileOps for StaticStringOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        read_str_offset(self.0, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

pub fn init() {
    fn make_sys_node(name: &str, ops: Arc<dyn FileOps>) -> Arc<VfsNode> {
        Arc::new(VfsNode {
            name: name.to_string(),
            kind: NodeKind::File,
            size: 0,
            children: RwLock::new(Vec::new()),
            file_ops: Some(ops),
            data: RwLock::new(Vec::new()),
            static_data: None,
            uid: RwLock::new(0),
            gid: RwLock::new(0),
            mode: RwLock::new(0o444),
        })
    }

    crate::fs::vfs::add_node("sys/class/net/eth0/address", make_sys_node("address", Arc::new(Eth0AddressOps)));
    crate::fs::vfs::add_node("sys/class/net/eth0/operstate", make_sys_node("operstate", Arc::new(StaticStringOps("up\n"))));
    crate::fs::vfs::add_node("sys/class/net/eth0/mtu", make_sys_node("mtu", Arc::new(StaticStringOps("1500\n"))));
    crate::fs::vfs::add_node("sys/class/net/eth0/speed", make_sys_node("speed", Arc::new(StaticStringOps("1000\n"))));

    crate::fs::vfs::add_node("sys/class/net/lo/operstate", make_sys_node("operstate", Arc::new(StaticStringOps("unknown\n"))));
    crate::fs::vfs::add_node("sys/class/net/lo/mtu", make_sys_node("mtu", Arc::new(StaticStringOps("65536\n"))));
}
