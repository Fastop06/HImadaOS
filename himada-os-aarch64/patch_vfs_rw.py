with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

# Update sys_openat
openat_old = """        if let Ok(path_str) = core::str::from_utf8(path_bytes) {
            if let Some(node) = crate::fs::vfs::lookup(path_str) {
                for i in 3..64 {
                    if FD_TABLE[i].is_none() {
                        FD_TABLE[i] = Some(FileDescriptor { node, offset: 0 });
                        return i as u64;
                    }
                }
                return !0;
            }
        }"""

openat_new = """        if let Ok(path_str) = core::str::from_utf8(path_bytes) {
            let node = if let Some(n) = crate::fs::vfs::lookup(path_str) {
                n
            } else if (_flags & 64) != 0 || (_flags & 1) != 0 || (_flags & 2) != 0 {
                // O_CREAT or O_WRONLY or O_RDWR: create new file in VFS
                let parts: alloc::vec::Vec<&str> = path_str.split('/').filter(|s| !s.is_empty()).collect();
                let fname = parts.last().unwrap_or(&"file");
                let new_node = alloc::sync::Arc::new(crate::fs::vfs::VfsNode {
                    name: alloc::string::ToString::to_string(*fname),
                    kind: crate::fs::vfs::NodeKind::File,
                    size: 0,
                    children: spin::RwLock::new(alloc::vec::Vec::new()),
                    file_ops: None,
                    data: spin::RwLock::new(alloc::vec::Vec::new()),
                    static_data: None,
                });
                crate::fs::vfs::add_node(path_str, new_node.clone());
                new_node
            } else {
                return !0;
            };

            for i in 3..64 {
                if FD_TABLE[i].is_none() {
                    FD_TABLE[i] = Some(FileDescriptor { node, offset: 0 });
                    return i as u64;
                }
            }
            return !0;
        }"""

content = content.replace(openat_old, openat_new)

# Update sys_read to read from node.data
read_old = """            if let Some(data) = desc.node.static_data {"""
read_new = """            let dyn_data = desc.node.data.read();
            if !dyn_data.is_empty() {
                let remaining = dyn_data.len().saturating_sub(desc.offset);
                let to_read = core::cmp::min(remaining, count as usize);
                if to_read > 0 {
                    let src = &dyn_data[desc.offset..desc.offset + to_read];
                    buf_slice[..to_read].copy_from_slice(src);
                    desc.offset += to_read;
                }
                return to_read as u64;
            }
            drop(dyn_data);
            if let Some(data) = desc.node.static_data {"""
content = content.replace(read_old, read_new)

# Update sys_write to write to node.data for NodeKind::File
write_old = """                    return !0;
                }
            }
        }
    }
    !0
}"""

write_new = """                    return !0;
                } else if let crate::fs::vfs::NodeKind::File = desc.node.kind {
                    let mut data = desc.node.data.write();
                    if desc.offset + count as usize > data.len() {
                        data.resize(desc.offset + count as usize, 0);
                    }
                    data[desc.offset..desc.offset + count as usize].copy_from_slice(buf_slice);
                    desc.offset += count as usize;
                    return count;
                }
            }
        }
    }
    !0
}"""

content = content.replace(write_old, write_new)

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
