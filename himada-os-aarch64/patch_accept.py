import sys
with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

accept_old = """                if socket.is_active() {
                    // Connection established!
                    // Return a new fd for the connection? 
                    // For the simplest Apache stub, accept() can just return the SAME fd.
                    return fd;
                }"""
accept_new = """                if socket.is_active() {
                    // Create a new FD for the established connection
                    for (i, new_fd) in unsafe { FD_TABLE.iter_mut() }.enumerate() {
                        if new_fd.is_none() {
                            let new_node = alloc::sync::Arc::new(crate::fs::vfs::VfsNode {
                                name: alloc::string::String::from("socket_conn"),
                                kind: crate::fs::vfs::NodeKind::Socket(handle),
                                size: 0,
                                children: spin::RwLock::new(alloc::vec::Vec::new()),
                                file_ops: None,
                                data: spin::RwLock::new(alloc::vec::Vec::new()),
                                static_data: None,
                            });
                            *new_fd = Some(crate::sys::linux_abi::FileDescriptor { node: new_node, offset: 0 });
                            return i as u64;
                        }
                    }
                    return fd;
                }"""

content = content.replace(accept_old, accept_new)
with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
