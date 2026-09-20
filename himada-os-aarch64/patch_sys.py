import sys
import re

with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

# I will just write a regex to replace the entire sys_read and sys_write functions
sys_read_regex = re.compile(r'fn sys_read\(fd: u64, buf: u64, count: u64\) -> u64 \{.*?\n\}', re.DOTALL)
sys_write_regex = re.compile(r'fn sys_write\(fd: u64, buf: u64, count: u64\) -> u64 \{.*?\n\}', re.DOTALL)

sys_read_new = """fn sys_read(fd: u64, buf: u64, count: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref mut desc) = FD_TABLE[fd as usize] {
            if let crate::fs::vfs::NodeKind::Socket(handle_id) = desc.node.kind {
                let handle = smoltcp::iface::SocketHandle::from(handle_id);
                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                crate::net::socket::poll();
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
                }
                return !0;
            }

            let buf_slice = core::slice::from_raw_parts_mut(buf as *mut u8, count as usize);
            if let Some(ref ops) = desc.node.file_ops {
                let read_bytes = ops.read(desc.offset, buf_slice);
                desc.offset += read_bytes;
                return read_bytes as u64;
            }
            if let Some(data) = desc.node.static_data {
                let remaining = data.len().saturating_sub(desc.offset);
                let to_read = core::cmp::min(remaining, count as usize);
                if to_read > 0 {
                    let src = &data[desc.offset..desc.offset + to_read];
                    buf_slice[..to_read].copy_from_slice(src);
                    desc.offset += to_read;
                }
                return to_read as u64;
            }
        }
    }
    !0
}"""

sys_write_new = """fn sys_write(fd: u64, buf: u64, count: u64) -> u64 {
    if fd == 1 || fd == 2 {
        unsafe {
            let slice = core::slice::from_raw_parts(buf as *const u8, count as usize);
            if let Ok(s) = core::str::from_utf8(slice) {
                if let Some(req) = crate::FRAMEBUFFER_REQUEST.response() {
                    if let Some(fb) = req.framebuffers().first() {
                        let text_color = crate::graphics::Color { r: 50, g: 255, b: 50, a: 255 };
                        crate::graphics::draw_string(&crate::FB_INFO, fb.address() as *mut u8, 10, LINUX_Y, s, text_color, 2);
                        crate::graphics::clean_dcache_range(fb.address() as usize + (LINUX_Y * crate::FB_INFO.pitch as usize), (crate::FB_INFO.pitch * 32) as usize);
                        LINUX_Y += 30;
                    }
                }
                return count;
            }
        }
    } else {
        unsafe {
            if let Some(ref mut desc) = FD_TABLE[fd as usize] {
                let buf_slice = core::slice::from_raw_parts(buf as *const u8, count as usize);
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
                    return !0;
                }
            }
        }
    }
    !0
}"""

content = sys_read_regex.sub(sys_read_new, content)
content = sys_write_regex.sub(sys_write_new, content)

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)

