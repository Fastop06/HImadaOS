import sys
with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

read_snippet = """
            if let crate::fs::vfs::NodeKind::Socket(handle_id) = desc.node.kind {
                let handle = smoltcp::iface::SocketHandle::from(handle_id);
                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                crate::net::socket::poll();
                if socket.can_recv() {
                    let recv_result = socket.recv(|data| {
                        let to_copy = core::cmp::min(data.len(), count as usize);
                        buf_slice[..to_copy].copy_from_slice(&data[..to_copy]);
                        (to_copy, to_copy)
                    });
                    if let Ok(bytes) = recv_result {
                        return bytes as u64;
                    }
                }
                return !0; // EAGAIN or error
            }
"""

content = content.replace("            if let Some(ref ops) = desc.node.file_ops {", read_snippet + "\n            if let Some(ref ops) = desc.node.file_ops {")

write_snippet = """
            if let crate::fs::vfs::NodeKind::Socket(handle_id) = desc.node.kind {
                let handle = smoltcp::iface::SocketHandle::from(handle_id);
                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                if socket.can_send() {
                    let send_result = socket.send_slice(buf_slice);
                    crate::net::socket::poll();
                    if let Ok(bytes) = send_result {
                        return bytes as u64;
                    }
                }
                return !0; // EAGAIN or error
            }
"""

content = content.replace("        if let Some(ref mut desc) = FD_TABLE[fd as usize] {", "        if let Some(ref mut desc) = FD_TABLE[fd as usize] {\n            let buf_slice = core::slice::from_raw_parts(buf as *const u8, count as usize);\n" + write_snippet)

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
