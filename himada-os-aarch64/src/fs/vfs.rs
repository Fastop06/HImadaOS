use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

pub trait FileOps: Send + Sync {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize;
    fn write(&self, offset: usize, buf: &[u8]) -> usize;
}

pub const PIPE_BUFFER_SIZE: usize = 65536;

pub struct PipeRingBuffer {
    pub buffer: Vec<u8>,
    pub read_pos: usize,
    pub write_pos: usize,
    pub count: usize,
    pub readers: usize,
    pub writers: usize,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            buffer: alloc::vec![0u8; PIPE_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
            readers: 1,
            writers: 1,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        if self.readers == 0 {
            return usize::MAX; // Broken pipe (EPIPE)
        }
        let space = PIPE_BUFFER_SIZE - self.count;
        let to_write = data.len().min(space);
        for i in 0..to_write {
            self.buffer[self.write_pos] = data[i];
            self.write_pos = (self.write_pos + 1) % PIPE_BUFFER_SIZE;
        }
        self.count += to_write;
        to_write
    }

    pub fn read(&mut self, out: &mut [u8]) -> usize {
        let to_read = self.count.min(out.len());
        for i in 0..to_read {
            out[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % PIPE_BUFFER_SIZE;
        }
        self.count -= to_read;
        to_read
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SocketTarget {
    Loopback(smoltcp::iface::SocketHandle),
    Ethernet(smoltcp::iface::SocketHandle),
    Dual {
        lo: smoltcp::iface::SocketHandle,
        eth: smoltcp::iface::SocketHandle,
    },
}

pub enum NodeKind {
    Socket(SocketTarget),
    UdpSocket(smoltcp::iface::SocketHandle, Arc<spin::Mutex<Option<(smoltcp::wire::IpAddress, u16)>>>),
    File,
    Directory,
    CharDevice,
    BlockDevice(Arc<spin::Mutex<dyn crate::hal::device::BlockDevice>>),
    Pipe(Arc<spin::Mutex<PipeRingBuffer>>),
}

pub struct Ext4FileOps {
    pub fs: Arc<spin::Mutex<crate::fs::ext4::Ext4Filesystem>>,
    pub inode_nr: u32,
}

impl FileOps for Ext4FileOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let fs = self.fs.lock();
        if let Ok(inode) = fs.read_inode(self.inode_nr) {
            if let Ok(data) = fs.read_file_data(&inode) {
                let remaining = data.len().saturating_sub(offset);
                let to_copy = buf.len().min(remaining);
                if to_copy > 0 {
                    buf[..to_copy].copy_from_slice(&data[offset..offset + to_copy]);
                }
                return to_copy;
            }
        }
        0
    }

    fn write(&self, offset: usize, buf: &[u8]) -> usize {
        let fs = self.fs.lock();
        if let Ok(inode) = fs.read_inode(self.inode_nr) {
            if let Ok(blocks) = fs.get_inode_blocks(&inode) {
                let block_size = fs.block_size;
                let mut written = 0;
                while written < buf.len() {
                    let curr_offset = offset + written;
                    let blk_idx = curr_offset / block_size;
                    let offset_in_blk = curr_offset % block_size;
                    if blk_idx >= blocks.len() {
                        break;
                    }
                    let phys_block = blocks[blk_idx];
                    let mut block_buf = alloc::vec![0u8; block_size];
                    let _ = fs.read_block(phys_block, &mut block_buf);
                    let chunk = (buf.len() - written).min(block_size - offset_in_blk);
                    block_buf[offset_in_blk..offset_in_blk + chunk].copy_from_slice(&buf[written..written + chunk]);
                    let _ = fs.write_block(phys_block, &block_buf);
                    written += chunk;
                }
                if offset + written > inode.size() as usize {
                    let mut updated_inode = inode;
                    updated_inode.i_size_lo = (offset + written) as u32;
                    let _ = fs.write_inode(self.inode_nr, &updated_inode);
                }
                return written;
            }
        }
        0
    }
}

pub struct MountPoint {
    pub target: String,
    pub source: String,
    pub fstype: String,
    pub fs: Arc<spin::Mutex<crate::fs::ext4::Ext4Filesystem>>,
}

lazy_static::lazy_static! {
    pub static ref MOUNT_POINTS: RwLock<Vec<MountPoint>> = RwLock::new(Vec::new());
}

pub struct VfsNode {
    pub name: String,
    pub kind: NodeKind,
    pub size: usize,
    pub children: RwLock<Vec<Arc<VfsNode>>>,
    pub file_ops: Option<Arc<dyn FileOps>>,
    pub data: RwLock<Vec<u8>>,
    pub static_data: Option<&'static [u8]>,
    pub uid: RwLock<u32>,
    pub gid: RwLock<u32>,
    pub mode: RwLock<u32>,
}

impl VfsNode {
    pub fn new(name: String, kind: NodeKind, uid: u32, gid: u32, mode: u32) -> Self {
        Self {
            name,
            kind,
            size: 0,
            children: RwLock::new(Vec::new()),
            file_ops: None,
            data: RwLock::new(Vec::new()),
            static_data: None,
            uid: RwLock::new(uid),
            gid: RwLock::new(gid),
            mode: RwLock::new(mode),
        }
    }

    pub fn check_permission(&self, uid: u32, gid: u32, want_read: bool, want_write: bool, want_exec: bool) -> bool {
        let mode = *self.mode.read();
        // Superuser (root, uid == 0) bypasses read and write permissions
        if uid == 0 {
            if want_exec {
                return (mode & 0o111) != 0;
            }
            return true;
        }

        let node_uid = *self.uid.read();
        let node_gid = *self.gid.read();

        let mut allowed_read = false;
        let mut allowed_write = false;
        let mut allowed_exec = false;

        if uid == node_uid {
            if (mode & 0o400) != 0 { allowed_read = true; }
            if (mode & 0o200) != 0 { allowed_write = true; }
            if (mode & 0o100) != 0 { allowed_exec = true; }
        } else if gid == node_gid {
            if (mode & 0o040) != 0 { allowed_read = true; }
            if (mode & 0o020) != 0 { allowed_write = true; }
            if (mode & 0o010) != 0 { allowed_exec = true; }
        } else {
            if (mode & 0o004) != 0 { allowed_read = true; }
            if (mode & 0o002) != 0 { allowed_write = true; }
            if (mode & 0o001) != 0 { allowed_exec = true; }
        }

        if want_read && !allowed_read { return false; }
        if want_write && !allowed_write { return false; }
        if want_exec && !allowed_exec { return false; }
        true
    }
}

lazy_static::lazy_static! {
    pub static ref ROOT: Arc<VfsNode> = Arc::new(VfsNode {
        name: "/".to_string(),
        kind: NodeKind::Directory,
        size: 0,
        children: RwLock::new(Vec::new()),
        file_ops: None,
        data: RwLock::new(Vec::new()),
        static_data: None,
        uid: RwLock::new(0),
        gid: RwLock::new(0),
        mode: RwLock::new(0o755),
    });
}

pub fn init() {
    // Force initialization of ROOT
    let _ = ROOT.name;
}

pub fn add_node(path: &str, node: Arc<VfsNode>) {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    let mut current = ROOT.clone();
    
    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        
        if is_last {
            let mut children = current.children.write();
            if let Some(pos) = children.iter().position(|c| c.name == node.name) {
                if matches!(children[pos].kind, NodeKind::Directory) && matches!(node.kind, NodeKind::Directory) {
                    break;
                }
                children[pos] = node.clone();
            } else {
                children.push(node.clone());
            }
            break;
        }
        
        let mut found = None;
        for child in current.children.read().iter() {
            if child.name == *part {
                found = Some(child.clone());
                break;
            }
        }
        
        if let Some(child) = found {
            current = child;
        } else {
            let new_dir = Arc::new(VfsNode::new(part.to_string(), NodeKind::Directory, 0, 0, 0o755));
            current.children.write().push(new_dir.clone());
            current = new_dir;
        }
    }
}

pub fn lookup(path: &str) -> Option<Arc<VfsNode>> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    if parts.is_empty() {
        return Some(ROOT.clone());
    }
    
    // Dynamic procfs lookup (for /proc/self/..., /proc/<pid>/...)
    if parts.len() >= 2 && parts[0] == "proc" {
        if let Some(dyn_node) = crate::fs::procfs::lookup_proc(&parts[1..]) {
            return Some(dyn_node);
        }
    }

    // Dynamic Ext4 mount point lookup
    let mounts = MOUNT_POINTS.read();
    for mp in mounts.iter() {
        let target_parts: Vec<&str> = mp.target.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
        if !target_parts.is_empty() && parts.starts_with(&target_parts) {
            let fs = mp.fs.clone();
            let fs_lock = fs.lock();
            let subparts = &parts[target_parts.len()..];
            if subparts.is_empty() {
                // Mount root directory (inode 2)
                let dir_node = Arc::new(VfsNode::new(
                    target_parts.last().unwrap_or(&"mnt").to_string(),
                    NodeKind::Directory,
                    0, 0, 0o755
                ));
                if let Ok(root_inode) = fs_lock.read_inode(2) {
                    if let Ok(entries) = fs_lock.read_dir_entries(&root_inode) {
                        let mut children = dir_node.children.write();
                        for entry in entries {
                            if entry.name == "." || entry.name == ".." {
                                continue;
                            }
                            if let Ok(child_ino) = fs_lock.read_inode(entry.inode) {
                                if child_ino.is_dir() {
                                    children.push(Arc::new(VfsNode::new(entry.name, NodeKind::Directory, 0, 0, 0o755)));
                                } else {
                                    let mut fnode = VfsNode::new(entry.name, NodeKind::File, 0, 0, 0o644);
                                    fnode.size = child_ino.size() as usize;
                                    fnode.file_ops = Some(Arc::new(Ext4FileOps {
                                        fs: fs.clone(),
                                        inode_nr: entry.inode,
                                    }));
                                    children.push(Arc::new(fnode));
                                }
                            }
                        }
                    }
                }
                return Some(dir_node);
            } else {
                let subpath = subparts.join("/");
                if let Ok(ino) = fs_lock.lookup_path(&subpath) {
                    if let Ok(inode) = fs_lock.read_inode(ino) {
                        let name = subparts.last().unwrap().to_string();
                        if inode.is_dir() {
                            let dir_node = Arc::new(VfsNode::new(name, NodeKind::Directory, 0, 0, 0o755));
                            if let Ok(entries) = fs_lock.read_dir_entries(&inode) {
                                let mut children = dir_node.children.write();
                                for entry in entries {
                                    if entry.name == "." || entry.name == ".." {
                                        continue;
                                    }
                                    if let Ok(child_ino) = fs_lock.read_inode(entry.inode) {
                                        if child_ino.is_dir() {
                                            children.push(Arc::new(VfsNode::new(entry.name, NodeKind::Directory, 0, 0, 0o755)));
                                        } else {
                                            let mut fnode = VfsNode::new(entry.name, NodeKind::File, 0, 0, 0o644);
                                            fnode.size = child_ino.size() as usize;
                                            fnode.file_ops = Some(Arc::new(Ext4FileOps {
                                                fs: fs.clone(),
                                               inode_nr: entry.inode,
                                            }));
                                            children.push(Arc::new(fnode));
                                        }
                                    }
                                }
                            }
                            return Some(dir_node);
                        } else {
                            let mut fnode = VfsNode::new(name, NodeKind::File, 0, 0, 0o644);
                            fnode.size = inode.size() as usize;
                            fnode.file_ops = Some(Arc::new(Ext4FileOps {
                                fs: fs.clone(),
                                inode_nr: ino,
                            }));
                            return Some(Arc::new(fnode));
                        }
                    }
                }
            }
        }
    }
    
    let mut current = ROOT.clone();
    
    for part in parts {
        let mut found = None;
        // Handle . and .. logic if needed here (simplified)
        for child in current.children.read().iter() {
            if child.name == part {
                found = Some(child.clone());
                break;
            }
        }
        
        if let Some(child) = found {
            current = child;
        } else {
            return None;
        }
    }
    
    Some(current)
}
