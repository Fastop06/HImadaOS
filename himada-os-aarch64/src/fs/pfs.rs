
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::RwLock;
use alloc::string::ToString;
use crate::fs::vfs::{VfsNode, NodeKind, ROOT};
use crate::hal::device::BlockDevice;

const MAGIC: [u8; 4] = *b"PFS1";

pub fn save_to_disk(blk: &dyn BlockDevice) {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    
    // Placeholder for total length
    out.extend_from_slice(&[0u8; 4]); 

    serialize_node(&ROOT, &mut out);

    let total_len = out.len() as u32;
    out[4..8].copy_from_slice(&total_len.to_le_bytes());

    // Pad to 512 byte sectors
    let pad = (512 - (out.len() % 512)) % 512;
    out.extend(alloc::vec![0; pad]);

    let sectors = (out.len() / 512) as u16;
    if let Err(e) = blk.write_sectors(0, sectors, &out) {
        crate::serial_println!("[PFS] Failed to save: {}", e);
    } else {
        crate::serial_println!("[PFS] Saved RamFS to disk! ({} bytes, {} sectors)", total_len, sectors);
    }
}

fn serialize_node(node: &Arc<VfsNode>, out: &mut Vec<u8>) {
    out.push(match node.kind {
        NodeKind::Directory => 1,
        NodeKind::File => 2,
        _ => 0,
    });
    let name_bytes = node.name.as_bytes();
    out.push(name_bytes.len() as u8);
    out.extend_from_slice(name_bytes);

    let data = node.data.read();
    let dlen = data.len() as u32;
    out.extend_from_slice(&dlen.to_le_bytes());
    out.extend_from_slice(&data);

    let children = node.children.read();
    let clen = children.len() as u32;
    out.extend_from_slice(&clen.to_le_bytes());

    for child in children.iter() {
        if matches!(child.kind, NodeKind::Directory | NodeKind::File) {
            serialize_node(child, out);
        }
    }
}

pub fn load_from_disk(blk: &dyn BlockDevice) -> bool {
    let mut header = [0u8; 512];
    if blk.read_sectors(0, 1, &mut header).is_err() {
        return false;
    }
    if &header[0..4] != &MAGIC {
        crate::serial_println!("[PFS] No valid PFS signature found.");
        return false;
    }
    let total_len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    let sectors = ((total_len + 511) / 512) as u16;
    
    let mut buf = alloc::vec![0u8; (sectors as usize) * 512];
    if blk.read_sectors(0, sectors, &mut buf).is_err() {
        return false;
    }

    let mut offset = 8;
    if let Some(new_root) = deserialize_node(&buf, &mut offset) {
        *ROOT.children.write() = new_root.children.read().clone();
        crate::serial_println!("[PFS] Successfully loaded RamFS from disk!");
        true
    } else {
        false
    }
}

fn deserialize_node(data: &[u8], offset: &mut usize) -> Option<Arc<VfsNode>> {
    if *offset >= data.len() { return None; }
    
    let kind_val = data[*offset];
    *offset += 1;
    let kind = match kind_val {
        1 => NodeKind::Directory,
        2 => NodeKind::File,
        _ => return None,
    };

    let name_len = data[*offset] as usize;
    *offset += 1;
    
    let name = core::str::from_utf8(&data[*offset..*offset+name_len]).unwrap_or("unknown").to_string();
    *offset += name_len;

    let dlen = u32::from_le_bytes(data[*offset..*offset+4].try_into().unwrap()) as usize;
    *offset += 4;
    
    let mut file_data = Vec::new();
    file_data.extend_from_slice(&data[*offset..*offset+dlen]);
    *offset += dlen;

    let clen = u32::from_le_bytes(data[*offset..*offset+4].try_into().unwrap()) as usize;
    *offset += 4;

    let default_mode = match kind {
        NodeKind::Directory => 0o755,
        _ => 0o644,
    };

    let node = Arc::new(VfsNode {
        name,
        kind,
        size: file_data.len(),
        children: RwLock::new(Vec::new()),
        file_ops: None,
        data: RwLock::new(file_data),
        static_data: None,
        uid: RwLock::new(0),
        gid: RwLock::new(0),
        mode: RwLock::new(default_mode),
    });

    for _ in 0..clen {
        if let Some(child) = deserialize_node(data, offset) {
            node.children.write().push(child);
        }
    }
    Some(node)
}
