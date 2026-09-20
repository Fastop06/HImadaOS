use core::str;
use alloc::sync::Arc;
use alloc::string::ToString;
use alloc::vec::Vec;
use spin::RwLock;
use crate::fs::vfs::{VfsNode, NodeKind};

pub fn parse(archive: &'static [u8]) {
    let mut offset = 0;
    
    while offset + 110 <= archive.len() {
        let magic = &archive[offset..offset + 6];
        if magic != b"070701" {
            break;
        }
        
        let parse_hex = |start: usize, len: usize| -> usize {
            let s = str::from_utf8(&archive[start..start + len]).unwrap_or("0");
            usize::from_str_radix(s, 16).unwrap_or(0)
        };
        
        let namesize = parse_hex(offset + 94, 8);
        let filesize = parse_hex(offset + 54, 8);
        let mode = parse_hex(offset + 14, 8); // Mode
        
        offset += 110;
        
        let name_bytes = &archive[offset..offset + namesize - 1]; // -1 for null terminator
        let name = str::from_utf8(name_bytes).unwrap_or("UNKNOWN");
        
        if name == "TRAILER!!!" {
            break;
        }
        
        // align offset to 4 bytes boundary for data
        offset += namesize;
        if offset % 4 != 0 {
            offset += 4 - (offset % 4);
        }
        
        let is_dir = (mode & 0o040000) != 0;
        
        let node_name = name.split('/').last().unwrap_or(name);
        
        let clean_name = name.trim_start_matches('.').trim_start_matches('/');
        let file_mode = if (mode & 0o777) != 0 { (mode & 0o777) as u32 } else { 0o644 };
        if filesize > 0 {
            let data = &archive[offset..offset + filesize];
            let node = Arc::new(VfsNode {
                name: node_name.to_string(),
                kind: NodeKind::File,
                size: filesize,
                children: RwLock::new(Vec::new()),
                file_ops: None,
                data: RwLock::new(Vec::new()),
                static_data: Some(data),
                uid: RwLock::new(0),
                gid: RwLock::new(0),
                mode: RwLock::new(file_mode),
            });
            crate::fs::vfs::add_node(clean_name, node);
        } else if !is_dir {
            let node = Arc::new(VfsNode {
                name: node_name.to_string(),
                kind: NodeKind::File,
                size: 0,
                children: RwLock::new(Vec::new()),
                file_ops: None,
                data: RwLock::new(Vec::new()),
                static_data: None,
                uid: RwLock::new(0),
                gid: RwLock::new(0),
                mode: RwLock::new(file_mode),
            });
            crate::fs::vfs::add_node(clean_name, node);
        } else if is_dir && !clean_name.is_empty() {
            let dir_mode = if (mode & 0o777) != 0 { (mode & 0o777) as u32 } else { 0o755 };
            let node = Arc::new(VfsNode {
                name: node_name.to_string(),
                kind: NodeKind::Directory,
                size: 0,
                children: RwLock::new(Vec::new()),
                file_ops: None,
                data: RwLock::new(Vec::new()),
                static_data: None,
                uid: RwLock::new(0),
                gid: RwLock::new(0),
                mode: RwLock::new(dir_mode),
            });
            crate::fs::vfs::add_node(clean_name, node);
        }
        
        offset += filesize;
        if offset % 4 != 0 {
            offset += 4 - (offset % 4);
        }
    }
}
