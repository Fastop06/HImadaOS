import re

with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

content = content.replace("let handle = smoltcp::iface::handle_id;", "")
content = content.replace("let handle = handle_id;", "")
content = content.replace("node.kind", "node.node.kind")
content = content.replace("desc.node.node.kind", "desc.node.kind")

# Fix fstat regex which failed because I missed the spacing
fstat_regex = re.compile(r'match desc\.node\.kind \{\n\s+crate::fs::vfs::NodeKind::Directory => 0x4000,\n\s+crate::fs::vfs::NodeKind::File => 0x8000,\n\s+crate::fs::vfs::NodeKind::CharDevice => 0x2000,\n\s+\}')
fstat_new = """match desc.node.kind {
                crate::fs::vfs::NodeKind::Directory => 0x4000,
                crate::fs::vfs::NodeKind::File => 0x8000,
                crate::fs::vfs::NodeKind::CharDevice => 0x2000,
                crate::fs::vfs::NodeKind::Socket(_) => 0xC000,
            }"""
if fstat_regex.search(content):
    content = fstat_regex.sub(fstat_new, content)
else:
    # manual replacement if regex failed
    content = content.replace("""match desc.node.kind {
                crate::fs::vfs::NodeKind::Directory => 0x4000,
                crate::fs::vfs::NodeKind::File => 0x8000,
                crate::fs::vfs::NodeKind::CharDevice => 0x2000,
            }""", fstat_new)

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
