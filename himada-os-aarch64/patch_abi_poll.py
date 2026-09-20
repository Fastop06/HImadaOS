import re

with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

# Update sys_read socket branch
sys_read_sock_old = re.compile(r'if let crate::fs::vfs::NodeKind::Socket\(handle\) = desc\.node\.kind \{.*?\n\s+return !0;\n\s+\}', re.DOTALL)
sys_read_sock_new = """if let crate::fs::vfs::NodeKind::Socket(handle) = desc.node.kind {
                let start = crate::net::socket::now();
                while (crate::net::socket::now() - start).total_millis() < 3000 {
                    crate::net::socket::poll();
                    let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                    let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                    if socket.can_recv() {
                        let buf_slice = core::slice::from_raw_parts_mut(buf as *mut u8, count as usize);
                        let recv_result = socket.recv(|data| {
                            let to_copy = core::cmp::min(data.len(), count as usize);
                            buf_slice[..to_copy].copy_from_slice(&data[..to_copy]);
                            (to_copy, to_copy)
                        });
                        if let Ok(bytes) = recv_result {
                            return bytes as u64;
                        }
                    } else if !socket.is_active() {
                        return 0; // EOF
                    }
                    drop(sockets);
                    for _ in 0..20_000 { core::hint::spin_loop(); }
                }
                return !0;
            }"""

content = sys_read_sock_old.sub(sys_read_sock_new, content)

# Update sys_accept Established check
content = content.replace("if socket.is_active() {", "if socket.state() == smoltcp::socket::tcp::State::Established {")

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
