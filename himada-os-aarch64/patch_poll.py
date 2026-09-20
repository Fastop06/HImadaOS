import sys
with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

write_old = """                    if socket.can_send() {
                        let send_result = socket.send_slice(buf_slice);
                        crate::net::socket::poll();
                        if let Ok(bytes) = send_result {
                            return bytes as u64;
                        }
                    }"""
write_new = """                    if socket.can_send() {
                        let send_result = socket.send_slice(buf_slice);
                        // Poll multiple times to ensure TX
                        for _ in 0..10 { crate::net::socket::poll(); }
                        if let Ok(bytes) = send_result {
                            return bytes as u64;
                        }
                    }"""
content = content.replace(write_old, write_new)

close_old = """                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                socket.close();
                drop(sockets);
                crate::net::socket::poll();"""
close_new = """                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                socket.close();
                drop(sockets);
                // Poll repeatedly to handle FIN / ACK
                for _ in 0..20 { crate::net::socket::poll(); for _ in 0..1000 { core::hint::spin_loop(); } }"""
content = content.replace(close_old, close_new)

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
