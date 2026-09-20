import re

with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

# Fix sys_socket VfsNode struct initialization
vfs_node_regex = re.compile(r'let node = alloc::sync::Arc::new\(crate::fs::vfs::VfsNode \{.*?\n\s+dynamic_data:.*?\}\);', re.DOTALL)
vfs_node_new = """let node = alloc::sync::Arc::new(crate::fs::vfs::VfsNode {
                    name: alloc::string::String::from("socket"),
                    kind: crate::fs::vfs::NodeKind::Socket(handle),
                    size: 0,
                    children: spin::RwLock::new(alloc::vec::Vec::new()),
                    file_ops: None,
                    data: spin::RwLock::new(alloc::vec::Vec::new()),
                    static_data: None,
                });"""
content = vfs_node_regex.sub(vfs_node_new, content)

# Fix sys_fstat non-exhaustive pattern
fstat_regex = re.compile(r'match desc\.node\.kind \{\n\s+crate::fs::vfs::NodeKind::Directory => 0x4000,\n\s+crate::fs::vfs::NodeKind::File => 0x8000,\n\s+crate::fs::vfs::NodeKind::CharDevice => 0x2000,\n\s+\}')
fstat_new = """match desc.node.kind {
                crate::fs::vfs::NodeKind::Directory => 0x4000,
                crate::fs::vfs::NodeKind::File => 0x8000,
                crate::fs::vfs::NodeKind::CharDevice => 0x2000,
                crate::fs::vfs::NodeKind::Socket(_) => 0xC000,
            }"""
content = fstat_regex.sub(fstat_new, content)

# Fix handle_id conversion in sys_read, sys_write, sys_listen, sys_accept
content = content.replace("SocketHandle::from(handle_id)", "handle_id")
content = content.replace("let handle = handle_id;", "")
content = content.replace("NodeKind::Socket(handle_id) = node.kind", "NodeKind::Socket(handle) = node.kind")
content = content.replace("NodeKind::Socket(handle_id) = desc.node.kind", "NodeKind::Socket(handle) = desc.node.kind")

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
