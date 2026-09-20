code = '''use core::fmt::Write;
use crate::serial_println;
use alloc::sync::Arc;
use crate::fs::vfs::{VfsNode, NodeKind};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer, State};
use smoltcp::iface::SocketHandle;

pub const SYS_READ: u64 = 63;
pub const SYS_WRITE: u64 = 64;
pub const SYS_WRITEV: u64 = 66;
pub const SYS_OPENAT: u64 = 56;
pub const SYS_CLOSE: u64 = 57;
pub const SYS_FSTAT: u64 = 80;
pub const SYS_MMAP: u64 = 222;
pub const SYS_BRK: u64 = 214;
pub const SYS_EXIT: u64 = 93;
pub const SYS_EXIT_GROUP: u64 = 94;
pub const SYS_EXECVE: u64 = 221;
pub const SYS_CLONE: u64 = 220;
pub const SYS_SET_TID_ADDRESS: u64 = 96;
pub const SYS_CLOCK_GETTIME: u64 = 113;
pub const SYS_IOCTL: u64 = 29;
pub const SYS_SOCKET: u64 = 198;
pub const SYS_BIND: u64 = 200;
pub const SYS_LISTEN: u64 = 201;
pub const SYS_ACCEPT: u64 = 202;
pub const SYS_CONNECT: u64 = 203;
pub const SYS_GET_FB_INFO: u64 = 1000;

pub fn dispatch_syscall(sys_no: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    match sys_no {
        SYS_READ => sys_read(arg0, arg1, arg2),
        SYS_WRITE => sys_write(arg0, arg1, arg2),
        SYS_WRITEV => sys_writev(arg0, arg1, arg2),
        SYS_OPENAT => sys_openat(arg0, arg1, arg2),
        SYS_CLOSE => sys_close(arg0),
        SYS_FSTAT => sys_fstat(arg0, arg1),
        SYS_MMAP => sys_mmap(arg0, arg1, arg2),
        SYS_BRK => sys_brk(arg0),
        SYS_EXIT | SYS_EXIT_GROUP => sys_exit(arg0),
        SYS_EXECVE => sys_execve(arg0, arg1, arg2),
        SYS_CLONE => sys_clone(arg0),
        SYS_SET_TID_ADDRESS => sys_set_tid_address(arg0),
        SYS_CLOCK_GETTIME => sys_clock_gettime(arg0, arg1),
        SYS_IOCTL => sys_ioctl(arg0, arg1, arg2),
        SYS_SOCKET => sys_socket(arg0, arg1, arg2),
        SYS_BIND => sys_bind(arg0, arg1, arg2),
        SYS_LISTEN => sys_listen(arg0, arg1),
        SYS_ACCEPT => sys_accept(arg0, arg1, arg2),
        SYS_CONNECT => sys_connect(arg0, arg1, arg2),
        SYS_GET_FB_INFO => sys_get_fb_info(arg0),
        _ => {
            serial_println!("[Syscall] Unimplemented syscall: {}", sys_no);
            !0 // -ENOSYS
        }
    }
}

pub struct FileDescriptor {
    pub node: Arc<VfsNode>,
    pub offset: usize,
}

pub static mut FD_TABLE: [Option<FileDescriptor>; 64] = [
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
];

fn sys_openat(_dirfd: u64, pathname: u64, flags: u64) -> u64 {
    unsafe {
        let mut len = 0;
        let mut ptr = pathname as *const u8;
        while *ptr != 0 {
            len += 1;
            ptr = ptr.add(1);
        }
        
        let path_bytes = core::slice::from_raw_parts(pathname as *const u8, len);
        if let Ok(path_str) = core::str::from_utf8(path_bytes) {
            let node = if let Some(n) = crate::fs::vfs::lookup(path_str) {
                n
            } else if (flags & 64) != 0 || (flags & 1) != 0 || (flags & 2) != 0 {
                let parts: alloc::vec::Vec<&str> = path_str.split('/').filter(|s| !s.is_empty()).collect();
                let fname = parts.last().unwrap_or(&"file");
                let new_node = alloc::sync::Arc::new(VfsNode {
                    name: alloc::string::ToString::to_string(*fname),
                    kind: NodeKind::File,
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
        }
    }
    !0
}

fn sys_read(fd: u64, buf: u64, count: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref mut desc) = FD_TABLE[fd as usize] {
            if let NodeKind::Socket(handle) = desc.node.kind {
                let start = crate::net::socket::now();
                while (crate::net::socket::now() - start).total_millis() < 4000 {
                    crate::net::socket::poll();
                    let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                    let socket = sockets.get_mut::<TcpSocket>(handle);
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
                    for _ in 0..10_000 { core::hint::spin_loop(); }
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
            } else {
                let data = desc.node.data.read();
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
}

fn sys_close(fd: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(desc) = FD_TABLE[fd as usize].take() {
            if let NodeKind::Socket(handle) = desc.node.kind {
                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<TcpSocket>(handle);
                socket.close();
                drop(sockets);
                for _ in 0..10 {
                    crate::net::socket::poll();
                }
            }
            return 0;
        }
    }
    !0
}

fn sys_fstat(fd: u64, statbuf: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref desc) = FD_TABLE[fd as usize] {
            #[repr(C)]
            struct Stat {
                st_dev: u64, st_ino: u64, st_mode: u32, st_nlink: u32,
                st_uid: u32, st_gid: u32, st_rdev: u64, __pad1: u64,
                st_size: i64, st_blksize: i32, __pad2: i32,
                st_blocks: i64,
            }
            
            core::ptr::write_bytes(statbuf as *mut u8, 0, core::mem::size_of::<Stat>());
            let stat = &mut *(statbuf as *mut Stat);
            
            stat.st_size = desc.node.size as i64;
            match desc.node.kind {
                NodeKind::File => stat.st_mode = 0o100777,
                NodeKind::Directory => stat.st_mode = 0o040777,
                NodeKind::CharDevice => stat.st_mode = 0o020777,
                NodeKind::Socket(_) => stat.st_mode = 0o140777,
            }
            return 0;
        }
    }
    !0
}

static mut LINUX_Y: usize = 300;

fn sys_write(fd: u64, buf: u64, count: u64) -> u64 {
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
                if let NodeKind::Socket(handle) = desc.node.kind {
                    let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                    let socket = sockets.get_mut::<TcpSocket>(handle);
                    if socket.can_send() {
                        let send_result = socket.send_slice(buf_slice);
                        drop(sockets);
                        for _ in 0..5 { crate::net::socket::poll(); }
                        if let Ok(bytes) = send_result {
                            return bytes as u64;
                        }
                    }
                    return !0;
                } else if let Some(ref ops) = desc.node.file_ops {
                    let write_bytes = ops.write(desc.offset, buf_slice);
                    desc.offset += write_bytes;
                    return write_bytes as u64;
                } else {
                    let mut data = desc.node.data.write();
                    data.extend_from_slice(buf_slice);
                    desc.offset += count as usize;
                    return count;
                }
            }
        }
    }
    !0
}

fn sys_writev(fd: u64, iov_ptr: u64, iovcnt: u64) -> u64 {
    if iov_ptr == 0 || iovcnt == 0 { return 0; }
    let mut total_written: u64 = 0;
    #[repr(C)]
    struct IoVec {
        base: u64,
        len: u64,
    }
    for i in 0..iovcnt {
        unsafe {
            let iov = core::ptr::read_unaligned((iov_ptr + i * core::mem::size_of::<IoVec>() as u64) as *const IoVec);
            let written = sys_write(fd, iov.base, iov.len);
            if written == !0 {
                return if total_written > 0 { total_written } else { !0 };
            }
            total_written += written;
        }
    }
    total_written
}

static mut HEAP_BRK: usize = 0x2000_0000;

fn sys_brk(new_brk: u64) -> u64 {
    unsafe {
        if new_brk == 0 {
            return HEAP_BRK as u64;
        }
        let target = new_brk as usize;
        if target > HEAP_BRK {
            let start_page = (HEAP_BRK + 4095) / 4096;
            let end_page = (target + 4095) / 4096;
            for p in start_page..end_page {
                let paddr = crate::mm::pmm::alloc_frame().expect("OOM");
                crate::mm::vmm::map_user_page(p * 4096, paddr);
            }
            HEAP_BRK = target;
        }
        HEAP_BRK as u64
    }
}

fn sys_ioctl(_fd: u64, _req: u64, _arg: u64) -> u64 { 0 }
fn sys_set_tid_address(_tidptr: u64) -> u64 { 1 }

fn sys_clock_gettime(_clock_id: u64, tp_ptr: u64) -> u64 {
    if tp_ptr == 0 { return !0; }
    let mut count: u64 = 0;
    let mut freq: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {0}, cntvct_el0", out(reg) count);
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) freq);
        if freq > 0 {
            let sec = count / freq;
            let nsec = ((count % freq) * 1_000_000_000) / freq;
            let ptr = tp_ptr as *mut u64;
            *ptr.add(0) = sec;
            *ptr.add(1) = nsec;
        }
    }
    0
}

pub fn sys_exit(_code: u64) -> u64 {
    crate::serial_println!("[Syscall] sys_exit called");
    !0
}

fn sys_execve(_filename: u64, _argv: u64, _envp: u64) -> u64 { !0 }
fn sys_clone(_flags: u64) -> u64 { !0 }

fn sys_mmap(addr: u64, len: u64, _prot: u64) -> u64 {
    let pages = (len + 4095) / 4096;
    let phys = crate::mm::pmm::alloc_frames(pages as usize).unwrap();
    unsafe {
        crate::sysmem::page_zero(crate::mm::vmm::phys_to_virt(phys) as *mut u8, (pages * 4096) as usize);
    }
    let virt = if addr == 0 { 0x4000_0000_0000 } else { addr };
    for i in 0..pages {
        let offset = (i * 4096) as usize;
        unsafe { crate::mm::vmm::map_user_page((virt + offset as u64) as usize, phys + offset); }
    }
    virt
}

fn sys_get_fb_info(_ptr: u64) -> u64 { 0 }

fn sys_socket(domain: u64, ty: u64, _protocol: u64) -> u64 {
    if domain == 2 && ty == 1 {
        let rx_buffer = SocketBuffer::new(alloc::vec![0; 4096]);
        let tx_buffer = SocketBuffer::new(alloc::vec![0; 4096]);
        let socket = TcpSocket::new(rx_buffer, tx_buffer);
        
        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
        let handle = sockets.add(socket);
        
        unsafe {
            for i in 3..64 {
                if FD_TABLE[i].is_none() {
                    let node = alloc::sync::Arc::new(VfsNode {
                        name: alloc::string::String::from("socket"),
                        kind: NodeKind::Socket(handle),
                        size: 0,
                        children: spin::RwLock::new(alloc::vec::Vec::new()),
                        file_ops: None,
                        data: spin::RwLock::new(alloc::vec::Vec::new()),
                        static_data: None,
                    });
                    FD_TABLE[i] = Some(FileDescriptor { node, offset: 0 });
                    return i as u64;
                }
            }
        }
    }
    !0
}

fn sys_bind(fd: u64, _sockaddr: u64, _addrlen: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    0
}

fn sys_listen(fd: u64, _backlog: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref desc) = FD_TABLE[fd as usize] {
            if let NodeKind::Socket(handle) = desc.node.kind {
                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<TcpSocket>(handle);
                if socket.listen(80).is_ok() {
                    return 0;
                }
            }
        }
    }
    !0
}

fn sys_accept(fd: u64, _sockaddr: u64, _addrlen: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    loop {
        crate::net::socket::poll();
        unsafe {
            if let Some(ref desc) = FD_TABLE[fd as usize] {
                if let NodeKind::Socket(handle) = desc.node.kind {
                    let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                    let socket = sockets.get_mut::<TcpSocket>(handle);
                    if socket.state() == State::Established {
                        for i in 3..64 {
                            if FD_TABLE[i].is_none() {
                                let new_node = alloc::sync::Arc::new(VfsNode {
                                    name: alloc::string::String::from("socket_conn"),
                                    kind: NodeKind::Socket(handle),
                                    size: 0,
                                    children: spin::RwLock::new(alloc::vec::Vec::new()),
                                    file_ops: None,
                                    data: spin::RwLock::new(alloc::vec::Vec::new()),
                                    static_data: None,
                                });
                                FD_TABLE[i] = Some(FileDescriptor { node: new_node, offset: 0 });
                                return i as u64;
                            }
                        }
                        return fd;
                    }
                }
            }
        }
        for _ in 0..20_000 { core::hint::spin_loop(); }
    }
}

fn sys_connect(fd: u64, sockaddr: u64, addrlen: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref desc) = FD_TABLE[fd as usize] {
            if let NodeKind::Socket(handle) = desc.node.kind {
                if addrlen >= 8 && sockaddr != 0 {
                    let port_be = core::ptr::read_unaligned((sockaddr + 2) as *const u16);
                    let port = u16::from_be(port_be);
                    let ip = core::ptr::read_unaligned((sockaddr + 4) as *const [u8; 4]);
                    if crate::net::socket::connect(handle, ip, port).is_ok() {
                        return 0;
                    }
                }
            }
        }
    }
    !0
}
'''

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(code)
