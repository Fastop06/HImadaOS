use core::fmt::Write;
use crate::serial_println;
use alloc::sync::Arc;
use crate::fs::vfs::{VfsNode, NodeKind};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer, State};
use smoltcp::iface::SocketHandle;
use crate::hal::device::BlockDevice;

pub const SYS_GETCWD: u64 = 17;
pub const SYS_EVENTFD2: u64 = 19;
pub const SYS_EPOLL_CREATE1: u64 = 20;
pub const SYS_EPOLL_CTL: u64 = 21;
pub const SYS_EPOLL_PWAIT: u64 = 22;
pub const SYS_DUP: u64 = 23;
pub const SYS_DUP3: u64 = 24;
pub const SYS_FCNTL: u64 = 25;
pub const SYS_IOCTL: u64 = 29;
pub const SYS_MKDIRAT: u64 = 34;
pub const SYS_UNLINKAT: u64 = 35;
pub const SYS_SYMLINKAT: u64 = 36;
pub const SYS_UMOUNT2: u64 = 39;
pub const SYS_MOUNT: u64 = 40;
pub const SYS_FACCESSAT: u64 = 48;
pub const SYS_CHDIR: u64 = 49;
pub const SYS_FCHMODAT: u64 = 53;
pub const SYS_FCHOWNAT: u64 = 54;
pub const SYS_OPENAT: u64 = 56;
pub const SYS_CLOSE: u64 = 57;
pub const SYS_PIPE2: u64 = 59;
pub const SYS_GETDENTS64: u64 = 61;
pub const SYS_LSEEK: u64 = 62;
pub const SYS_READ: u64 = 63;
pub const SYS_WRITE: u64 = 64;
pub const SYS_WRITEV: u64 = 66;
pub const SYS_PPOLL: u64 = 73;
pub const SYS_READLINKAT: u64 = 78;
pub const SYS_NEWFSTATAT: u64 = 79;
pub const SYS_FSTAT: u64 = 80;
pub const SYS_EXIT: u64 = 93;
pub const SYS_EXIT_GROUP: u64 = 94;
pub const SYS_SET_TID_ADDRESS: u64 = 96;
pub const SYS_FUTEX: u64 = 98;
pub const SYS_SET_ROBUST_LIST: u64 = 99;
pub const SYS_NANOSLEEP: u64 = 101;
pub const SYS_CLOCK_GETTIME: u64 = 113;
pub const SYS_SCHED_YIELD: u64 = 124;
pub const SYS_KILL: u64 = 129;
pub const SYS_TKILL: u64 = 130;
pub const SYS_TGKILL: u64 = 131;
pub const SYS_SIGALTSTACK: u64 = 132;
pub const SYS_RT_SIGACTION: u64 = 134;
pub const SYS_RT_SIGPROCMASK: u64 = 135;
pub const SYS_SETGID: u64 = 144;
pub const SYS_SETUID: u64 = 146;
pub const SYS_UNAME: u64 = 160;
pub const SYS_UMASK: u64 = 166;
pub const SYS_PRCTL: u64 = 167;
pub const SYS_GETTIMEOFDAY: u64 = 169;
pub const SYS_GETPID: u64 = 172;
pub const SYS_GETPPID: u64 = 173;
pub const SYS_GETUID: u64 = 174;
pub const SYS_GETEUID: u64 = 175;
pub const SYS_GETGID: u64 = 176;
pub const SYS_GETEGID: u64 = 177;
pub const SYS_GETTID: u64 = 178;
pub const SYS_SOCKET: u64 = 198;
pub const SYS_BIND: u64 = 200;
pub const SYS_LISTEN: u64 = 201;
pub const SYS_ACCEPT: u64 = 202;
pub const SYS_CONNECT: u64 = 203;
pub const SYS_BRK: u64 = 214;
pub const SYS_MUNMAP: u64 = 215;
pub const SYS_MREMAP: u64 = 216;
pub const SYS_CLONE: u64 = 220;
pub const SYS_EXECVE: u64 = 221;
pub const SYS_MMAP: u64 = 222;
pub const SYS_MPROTECT: u64 = 226;
pub const SYS_MADVISE: u64 = 233;
pub const SYS_WAIT4: u64 = 260;
pub const SYS_PRLIMIT64: u64 = 261;
pub const SYS_GETRANDOM: u64 = 278;
pub const SYS_EPOLL_PWAIT2: u64 = 281;
pub const SYS_RSEQ: u64 = 293;
pub const SYS_GET_FB_INFO: u64 = 1000;

pub fn dispatch_syscall_ctx(context: &mut crate::hal::exceptions::ExceptionContext) -> u64 {
    static mut FIRST: bool = false;
    unsafe {
        if !FIRST {
            FIRST = true;
            crate::log_step("First userspace syscall received!", Some("OK"));
        }
    }
    let sys_no = context.x[8];
    let arg0 = context.x[0];
    let arg1 = context.x[1];
    let arg2 = context.x[2];
    match sys_no {
        SYS_GETCWD => sys_getcwd(arg0, arg1),
        SYS_EVENTFD2 => sys_eventfd2(arg0, arg1),
        SYS_EPOLL_CREATE1 => sys_epoll_create1(arg0),
        SYS_EPOLL_CTL => sys_epoll_ctl(arg0, arg1, arg2, context.x[3]),
        SYS_EPOLL_PWAIT => sys_epoll_pwait(arg0, arg1, arg2, context.x[3] as i64, context.x[4]),
        SYS_EPOLL_PWAIT2 => sys_epoll_pwait2(arg0, arg1, arg2, context.x[3], context.x[4]),
        SYS_CHDIR => sys_chdir(arg0),
        SYS_DUP => sys_dup(arg0),
        SYS_DUP3 => sys_dup3(arg0, arg1, arg2),
        SYS_FCNTL => sys_fcntl(arg0, arg1, arg2),
        SYS_IOCTL => sys_ioctl(arg0, arg1, arg2),
        SYS_MKDIRAT => sys_mkdirat(arg0, arg1, arg2),
        SYS_UNLINKAT => sys_unlinkat(arg0, arg1, arg2),
        SYS_SYMLINKAT => sys_symlinkat(arg0, arg1 as i64, arg2),
        SYS_FCHMODAT => sys_fchmodat(arg0, arg1, arg2),
        SYS_FCHOWNAT => sys_fchownat(arg0, arg1, arg2, context.x[3], context.x[4]),
        SYS_FACCESSAT => sys_faccessat(arg0 as i64, arg1, arg2 as i32, context.x[3] as i32),
        SYS_OPENAT => sys_openat(arg0, arg1, arg2),
        SYS_CLOSE => sys_close(arg0),
        SYS_PIPE2 => sys_pipe2(arg0, arg1),
        SYS_GETDENTS64 => sys_getdents64(arg0, arg1, arg2),
        SYS_LSEEK => sys_lseek(arg0, arg1 as i64, arg2),
        SYS_READ => sys_read(arg0, arg1, arg2),
        SYS_WRITE => sys_write(arg0, arg1, arg2),
        SYS_WRITEV => sys_writev(arg0, arg1, arg2),
        SYS_PPOLL => sys_ppoll(arg0, arg1, arg2, context.x[3]),
        SYS_READLINKAT => sys_readlinkat(arg0 as i64, arg1, arg2, context.x[3]),
        SYS_NEWFSTATAT => sys_newfstatat(arg0 as i64, arg1, arg2, context.x[3]),
        SYS_FSTAT => sys_fstat(arg0, arg1),
        SYS_EXIT => sys_exit(arg0),
        SYS_EXIT_GROUP => sys_exit_group(arg0),
        SYS_SET_TID_ADDRESS => sys_set_tid_address(arg0),
        SYS_FUTEX => crate::sys::futex::sys_futex(arg0, arg1 as i32, arg2 as u32, context.x[3], context.x[4], context.x[5] as u32),
        SYS_SET_ROBUST_LIST => 0,
        SYS_NANOSLEEP => sys_nanosleep(arg0, arg1),
        SYS_CLOCK_GETTIME => sys_clock_gettime(arg0, arg1),
        SYS_SCHED_YIELD => {
            crate::sys::process::schedule();
            0
        },
        122 => 0, // SYS_SCHED_SETAFFINITY
        123 => sys_sched_getaffinity(arg0, arg1, arg2),
        SYS_KILL => sys_kill(arg0 as i64, arg1 as i32),
        SYS_TKILL | SYS_TGKILL => 0,
        SYS_SIGALTSTACK => 0,
        SYS_RT_SIGACTION => sys_rt_sigaction(arg0 as i32, arg1, arg2, context.x[3]),
        SYS_RT_SIGPROCMASK => sys_rt_sigprocmask(arg0 as i32, arg1, arg2, context.x[3]),
        SYS_SETGID => sys_setgid(arg0),
        SYS_SETUID => sys_setuid(arg0),
        SYS_UNAME => sys_uname(arg0),
        SYS_MOUNT => sys_mount(arg0, arg1, arg2, context.x[3], context.x[4]),
        SYS_UMOUNT2 => sys_umount2(arg0, arg1 as i32),
        SYS_UMASK => sys_umask(arg0),
        SYS_PRCTL => 0,
        SYS_GETTIMEOFDAY => sys_gettimeofday(arg0, arg1),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_GETUID | SYS_GETEUID => sys_getuid(),
        SYS_GETGID | SYS_GETEGID => sys_getgid(),
        SYS_GETTID => sys_gettid(),
        SYS_SOCKET => sys_socket(arg0, arg1, arg2),
        199 => sys_socketpair(arg0, arg1, arg2, context.x[3]),
        SYS_BIND => sys_bind(arg0, arg1, arg2),
        SYS_LISTEN => sys_listen(arg0, arg1),
        SYS_ACCEPT => sys_accept(arg0, arg1, arg2),
        SYS_CONNECT => sys_connect(arg0, arg1, arg2),
        SYS_BRK => sys_brk(arg0),
        SYS_MUNMAP => 0,
        SYS_MREMAP => sys_mremap(arg0, arg1, arg2, context.x[3], context.x[4]),
        SYS_CLONE => sys_clone_ctx(arg0, arg1, arg2, context.x[3], context.x[4], context),
        SYS_EXECVE => sys_execve_ctx(arg0, arg1, arg2, context),
        SYS_MMAP => sys_mmap(arg0, arg1, arg2, context.x[3], context.x[4], context.x[5]),
        SYS_MPROTECT => 0,
        SYS_MADVISE => 0,
        SYS_WAIT4 => sys_wait4(arg0 as i64, arg1, arg2),
        SYS_PRLIMIT64 => sys_prlimit64(arg0 as i64, arg1, arg2, context.x[3]),
        SYS_GETRANDOM => sys_getrandom(arg0, arg1, arg2),
        52 | 53 => 0, // sys_fchmod / sys_fchmodat (success)
        SYS_RSEQ => (-38i64) as u64, // -ENOSYS
        291 => sys_statx(arg0 as i64, arg1, arg2 as i32, context.x[3] as u32, context.x[4]),
        435 => (-38i64) as u64, // SYS_CLONE3 -> -ENOSYS
        SYS_GET_FB_INFO => sys_get_fb_info(arg0),
        81 | 162 => sys_sync(),
        _ => {
            serial_println!("[Syscall] Unimplemented syscall: {}", sys_no);
            (-38i64) as u64 // -ENOSYS
        }
    }
}

fn sys_sched_getaffinity(_pid: u64, len: u64, mask_ptr: u64) -> u64 {
    if mask_ptr == 0 { return (-14i64) as u64; } // -EFAULT
    if len < 8 { return (-22i64) as u64; } // -EINVAL
    let copy_bytes = (len as usize).min(128);
    unsafe {
        core::ptr::write_bytes(mask_ptr as *mut u8, 0, copy_bytes);
        *(mask_ptr as *mut u64) = 1; // Bit 0 = CPU 0
    }
    copy_bytes as u64
}

pub fn dispatch_syscall(sys_no: u64, arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let mut dummy_ctx = crate::hal::exceptions::ExceptionContext {
        x: [0; 31],
        elr: 0,
        spsr: 0,
        sp: 0,
    };
    dummy_ctx.x[8] = sys_no;
    dummy_ctx.x[0] = arg0;
    dummy_ctx.x[1] = arg1;
    dummy_ctx.x[2] = arg2;
    dispatch_syscall_ctx(&mut dummy_ctx)
}

#[derive(Clone)]
pub struct FileDescriptor {
    pub node: Arc<VfsNode>,
    pub offset: usize,
    pub pty_session: Option<Arc<crate::fs::pty::PtySession>>,
    pub path: alloc::string::String,
    pub nonblock: bool,
}

impl FileDescriptor {
    pub fn new(node: Arc<VfsNode>, offset: usize) -> Self {
        Self { node, offset, pty_session: None, path: alloc::string::String::new(), nonblock: false }
    }

    pub fn with_path(node: Arc<VfsNode>, offset: usize, path: alloc::string::String) -> Self {
        Self { node, offset, pty_session: None, path, nonblock: false }
    }
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

unsafe fn alloc_fd(desc: FileDescriptor) -> Option<usize> {
    for i in 3..64 {
        if FD_TABLE[i].is_none() {
            FD_TABLE[i] = Some(desc);
            return Some(i);
        }
    }
    None
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

fn sys_ppoll(fds_ptr: u64, nfds: u64, _timeout_ptr: u64, _sigmask_ptr: u64) -> u64 {
    if fds_ptr == 0 { return 0; }
    if nfds > 256 { return !0; }
    let mut ready = 0;
    unsafe {
        for i in 0..nfds as usize {
            let pfd_ptr = (fds_ptr as *mut PollFd).add(i);
            let mut pfd = core::ptr::read_unaligned(pfd_ptr);
            if pfd.fd >= 0 && pfd.fd < 3 {
                // Stdin, Stdout, Stderr are always valid console character devices!
                // Bit 5 (0x20 = POLLNVAL) MUST NOT BE SET.
                pfd.revents = pfd.events & 0x0005; // POLLIN (1) | POLLOUT (4)
                if pfd.revents != 0 {
                    ready += 1;
                }
            } else if pfd.fd >= 3 && (pfd.fd as usize) < 64 && FD_TABLE[pfd.fd as usize].is_some() {
                pfd.revents = pfd.events & 0x0005;
                if pfd.revents != 0 {
                    ready += 1;
                }
            } else {
                pfd.revents = 0x0020; // POLLNVAL
            }
            core::ptr::write_unaligned(pfd_ptr, pfd);
        }
    }
    ready
}

fn sys_openat(dirfd: u64, pathname: u64, flags: u64) -> u64 {
    unsafe {
        let mut len = 0;
        let mut ptr = pathname as *const u8;
        while *ptr != 0 {
            len += 1;
            ptr = ptr.add(1);
        }
        
        let path_bytes = core::slice::from_raw_parts(pathname as *const u8, len);
        if let Ok(raw_path_str) = core::str::from_utf8(path_bytes) {
            let (euid, egid) = {
                let pm = crate::sys::process::PROCESS_MANAGER.lock();
                let cur_pid = pm.current_pid;
                if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == cur_pid) {
                    (p.euid, p.egid)
                } else {
                    (0, 0)
                }
            };
            let path_resolved = resolve_at_path(dirfd as i64, raw_path_str);
            let path_str = &path_resolved;
            let clean = path_str.trim_start_matches('/');
            if clean == "dev/null" || clean == "dev/zero" || clean == "dev/urandom" {
                let dev_name = if clean == "dev/null" { "null" } else if clean == "dev/zero" { "zero" } else { "urandom" };
                let dev_node = alloc::sync::Arc::new(VfsNode {
                    name: alloc::string::String::from(dev_name),
                    kind: NodeKind::CharDevice,
                    size: 0,
                    children: spin::RwLock::new(alloc::vec::Vec::new()),
                    file_ops: None,
                    data: spin::RwLock::new(alloc::vec::Vec::new()),
                    static_data: None,
                    uid: spin::RwLock::new(0),
                    gid: spin::RwLock::new(0),
                    mode: spin::RwLock::new(0o666),
                });
                for i in 3..64 {
                    if FD_TABLE[i].is_none() {
                        FD_TABLE[i] = Some(FileDescriptor::with_path(dev_node, 0, path_resolved));
                        return i as u64;
                    }
                }
                return !0;
            }

            if clean == "dev/ptmx" {
                if let Some((session, master_node)) = crate::fs::pty::open_ptmx() {
                    for i in 3..64 {
                        if FD_TABLE[i].is_none() {
                            FD_TABLE[i] = Some(FileDescriptor {
                                node: master_node,
                                offset: 0,
                                pty_session: Some(session),
                                path: alloc::string::String::from("/dev/ptmx"),
                                nonblock: false,
                            });
                            return i as u64;
                        }
                    }
                }
                return !0;
            }

            if clean.starts_with("dev/pts/") {
                let sub = &clean["dev/pts/".len()..];
                if let Ok(id) = sub.parse::<usize>() {
                    if let Some(session) = crate::fs::pty::get_pty_session(id) {
                        let node = crate::fs::vfs::lookup(path_str)
                            .or_else(|| crate::fs::vfs::lookup(clean))
                            .or_else(|| session.slave_node.read().clone());
                        if let Some(node) = node {
                            for i in 3..64 {
                                if FD_TABLE[i].is_none() {
                                    FD_TABLE[i] = Some(FileDescriptor {
                                        node,
                                        offset: 0,
                                        pty_session: Some(session),
                                        path: path_resolved,
                                        nonblock: false,
                                    });
                                    return i as u64;
                                }
                            }
                        }
                    }
                }
                return !0;
            }

            let (euid, egid) = {
                let pm = crate::sys::process::PROCESS_MANAGER.lock();
                let cur_pid = pm.current_pid;
                if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == cur_pid) {
                    (p.euid, p.egid)
                } else {
                    (0, 0)
                }
            };

            let node = if let Some(n) = crate::fs::vfs::lookup(path_str).or_else(|| {
                if path_str.starts_with("/lib/") {
                    let mut p = alloc::string::String::from("/usr");
                    p.push_str(path_str);
                    crate::fs::vfs::lookup(&p)
                } else if path_str.starts_with("/usr/lib/") {
                    crate::fs::vfs::lookup(&path_str[4..])
                } else {
                    None
                }
            }) {
                let want_read = (flags & 3) == 0 || (flags & 3) == 2;
                let want_write = (flags & 3) == 1 || (flags & 3) == 2;
                if !n.check_permission(euid, egid, want_read, want_write, false) {
                    return (-13i64) as u64; // -EACCES
                }
                n
            } else if (flags & 64) != 0 {
                let clean = path_str.trim_start_matches('/');
                let mounts = crate::fs::vfs::MOUNT_POINTS.read();
                let mut ext4_node = None;
                for mp in mounts.iter() {
                    let target_clean = mp.target.trim_start_matches('/');
                    if clean.starts_with(target_clean) && clean.len() > target_clean.len() {
                        let sub = clean[target_clean.len()..].trim_start_matches('/');
                        let mut fs = mp.fs.lock();
                        if let Ok(ino) = fs.create_file(2, sub, &[]) {
                            let mut fnode = VfsNode::new(alloc::string::ToString::to_string(sub), NodeKind::File, euid, egid, 0o644);
                            fnode.file_ops = Some(alloc::sync::Arc::new(crate::fs::vfs::Ext4FileOps {
                                fs: mp.fs.clone(),
                                inode_nr: ino,
                            }));
                            ext4_node = Some(alloc::sync::Arc::new(fnode));
                            break;
                        }
                    }
                }
                drop(mounts);

                if let Some(new_node) = ext4_node {
                    new_node
                } else {
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
                        uid: spin::RwLock::new(euid),
                        gid: spin::RwLock::new(egid),
                        mode: spin::RwLock::new(0o644),
                    });
                    crate::fs::vfs::add_node(path_str, new_node.clone());
                    new_node
                }
            } else {
                return (-2i64) as u64; // -ENOENT
            };

            for i in 3..64 {
                if FD_TABLE[i].is_none() {
                    FD_TABLE[i] = Some(FileDescriptor::with_path(node, 0, path_resolved));
                    return i as u64;
                }
            }
        }
    }
    (-2i64) as u64 // -ENOENT
}

fn sys_read(fd: u64, buf: u64, count: u64) -> u64 {
    if fd >= 64 { return !0; }
    if fd == 0 {
        if count == 0 || buf == 0 { return 0; }
        unsafe {
            if let Some(ref mut desc) = FD_TABLE[0] {
                return read_from_desc(desc, buf, count);
            }
        }
        // 1. Check USB HID Keyboard via xHCI (Parallels Desktop Apple Silicon & QEMU)
        if let Some(ch) = crate::hal::xhci::poll_keyboard() {
            unsafe { *(buf as *mut u8) = ch; }
            return 1;
        }
        // 2. Check VirtIO Input (Parallels Desktop AArch64 / virtio-input path)
        if let Some(ch) = crate::hal::virtio_input::poll_keyboard() {
            unsafe { *(buf as *mut u8) = ch; }
            return 1;
        }
        // 3. Check PL050 KMI PS/2 Keyboard
        if let Some(ch) = crate::hal::kmi::poll_keyboard() {
            unsafe { *(buf as *mut u8) = ch; }
            return 1;
        }
        // 4. Serial Port fallback (QEMU PL011 / UART console input)
        if let Some(ch) = crate::hal::serial::read_byte() {
            unsafe { *(buf as *mut u8) = ch; }
            return 1;
        }
        return 0;
    }
    unsafe {
        if let Some(ref mut desc) = FD_TABLE[fd as usize] {
            return read_from_desc(desc, buf, count);
        }
    }
    !0
}

unsafe fn read_from_desc(desc: &mut FileDescriptor, buf: u64, count: u64) -> u64 {
    if let NodeKind::EventFd(ref lock) = desc.node.kind {
        if count < 8 || buf == 0 { return (-22i64) as u64; /* -EINVAL */ }
        let mut state = lock.lock();
        let val = if state.counter > 0 {
            let v = state.counter;
            state.counter = 0;
            v
        } else {
            0
        };
        *(buf as *mut u64) = val;
        return 8;
    }
    if let NodeKind::Pipe(ref ring_lock) = desc.node.kind {
        let buf_slice = core::slice::from_raw_parts_mut(buf as *mut u8, count as usize);
        let start = crate::net::socket::now();
        loop {
            let mut ring = ring_lock.lock();
            let bytes_read = ring.read(buf_slice);
            if bytes_read > 0 {
                return bytes_read as u64;
            }
            if ring.writers == 0 {
                return 0; // EOF
            }
            drop(ring);
            if (crate::net::socket::now() - start).total_millis() > 3000 {
                return 0;
            }
            crate::sys::process::schedule();
        }
    }
    if let NodeKind::Socket(target) = desc.node.kind {
        let nonblock = desc.nonblock;
        match target {
            crate::fs::vfs::SocketTarget::Loopback(handle) => {
                let start = crate::net::socket::now();
                let timeout_ms = if nonblock { 0 } else { 2000 };
                loop {
                    crate::net::socket::poll();
                    let mut sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
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
                    } else if !socket.is_active() || socket.state() == State::CloseWait || socket.state() == State::Closed || socket.state() == State::TimeWait {
                        return 0; // EOF
                    }
                    drop(sockets);
                    if (crate::net::socket::now() - start).total_millis() >= timeout_ms {
                        return 0; // -EAGAIN for nonblock, or timeout
                    }
                    for _ in 0..5_000 { core::hint::spin_loop(); }
                }
            }
            crate::fs::vfs::SocketTarget::Ethernet(handle) => {
                let start = crate::net::socket::now();
                let timeout_ms = if nonblock { 0 } else { 2000 };
                loop {
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
                    } else if !socket.is_active() || socket.state() == State::CloseWait || socket.state() == State::Closed || socket.state() == State::TimeWait {
                        return 0; // EOF
                    }
                    drop(sockets);
                    if (crate::net::socket::now() - start).total_millis() >= timeout_ms {
                        return 0; // -EAGAIN for nonblock, or timeout
                    }
                    for _ in 0..5_000 { core::hint::spin_loop(); }
                }
            }
            _ => return !0,
        }
    }

    if let NodeKind::UdpSocket(handle, _) = desc.node.kind {
        let start = crate::net::socket::now();
        while (crate::net::socket::now() - start).total_millis() < 2500 {
            crate::net::socket::poll();
            let mut sockets = crate::net::socket::NET_SOCKETS.lock();
            let socket = sockets.get_mut::<smoltcp::socket::udp::Socket>(handle);
            if socket.can_recv() {
                let buf_slice = core::slice::from_raw_parts_mut(buf as *mut u8, count as usize);
                if let Ok((len, _meta)) = socket.recv_slice(buf_slice) {
                    return len as u64;
                }
            }
            drop(sockets);
            for _ in 0..10_000 { core::hint::spin_loop(); }
        }
        return !0;
    }

    if matches!(desc.node.kind, NodeKind::CharDevice) {
        if desc.node.name == "null" {
            return 0;
        } else if desc.node.name == "zero" {
            let buf_slice = core::slice::from_raw_parts_mut(buf as *mut u8, count as usize);
            buf_slice.fill(0);
            return count;
        } else if desc.node.name == "urandom" {
            return sys_getrandom(buf, count, 0);
        }
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

fn sys_close(fd: u64) -> u64 {
    if fd >= 64 { return (-9i64) as u64; }
    unsafe {
        if let Some(desc) = FD_TABLE[fd as usize].take() {
            if let NodeKind::Pipe(ref ring_lock) = desc.node.kind {
                let mut ring = ring_lock.lock();
                if desc.node.name == "pipe_r" {
                    ring.readers = ring.readers.saturating_sub(1);
                } else if desc.node.name == "pipe_w" {
                    ring.writers = ring.writers.saturating_sub(1);
                }
                return 0;
            }
            if let NodeKind::Socket(target) = desc.node.kind {
                match target {
                    crate::fs::vfs::SocketTarget::Loopback(handle) => {
                        let mut sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
                        let socket = sockets.get_mut::<TcpSocket>(handle);
                        socket.close();
                        drop(sockets);
                        let start = crate::net::socket::now();
                        while (crate::net::socket::now() - start).total_millis() < 200 {
                            crate::net::socket::poll();
                            let sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
                            let sock = sockets.get::<TcpSocket>(handle);
                            if sock.state() == State::Closed || sock.state() == State::TimeWait {
                                break;
                            }
                            drop(sockets);
                            for _ in 0..1_000 { core::hint::spin_loop(); }
                        }
                        let mut sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
                        let sock = sockets.get_mut::<TcpSocket>(handle);
                        if sock.state() != State::Closed && sock.state() != State::TimeWait {
                            sock.abort();
                        }
                        drop(sockets);
                        for _ in 0..5 { crate::net::socket::poll(); }
                        let mut sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
                        sockets.remove(handle);
                    }
                    crate::fs::vfs::SocketTarget::Ethernet(handle) => {
                        let start = crate::net::socket::now();
                        while (crate::net::socket::now() - start).total_millis() < 500 {
                            crate::net::socket::poll();
                            let sockets = crate::net::socket::NET_SOCKETS.lock();
                            let socket = sockets.get::<TcpSocket>(handle);
                            if socket.send_queue() == 0 {
                                break;
                            }
                            drop(sockets);
                            for _ in 0..5_000 { core::hint::spin_loop(); }
                        }
                        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                        let socket = sockets.get_mut::<TcpSocket>(handle);
                        socket.close();
                        drop(sockets);
                        let close_start = crate::net::socket::now();
                        while (crate::net::socket::now() - close_start).total_millis() < 500 {
                            crate::net::socket::poll();
                            let sockets = crate::net::socket::NET_SOCKETS.lock();
                            let sock = sockets.get::<TcpSocket>(handle);
                            if sock.state() == State::Closed || sock.state() == State::TimeWait {
                                break;
                            }
                            drop(sockets);
                            for _ in 0..2_000 { core::hint::spin_loop(); }
                        }
                        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                        let sock = sockets.get_mut::<TcpSocket>(handle);
                        if sock.state() != State::Closed && sock.state() != State::TimeWait {
                            sock.abort();
                        }
                        drop(sockets);
                        for _ in 0..5 { crate::net::socket::poll(); }
                        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                        sockets.remove(handle);
                    }
                    crate::fs::vfs::SocketTarget::Dual { lo, eth } => {
                        let mut lo_socks = crate::net::socket::LOOPBACK_SOCKETS.lock();
                        lo_socks.get_mut::<TcpSocket>(lo).close();
                        lo_socks.remove(lo);
                        let mut net_socks = crate::net::socket::NET_SOCKETS.lock();
                        net_socks.get_mut::<TcpSocket>(eth).close();
                        net_socks.remove(eth);
                    }
                }
            }
            if let NodeKind::UdpSocket(handle, _) = desc.node.kind {
                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                sockets.remove(handle);
            }
            return 0;
        }
        if fd <= 2 {
            return 0;
        }
    }
    (-9i64) as u64
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad1: u64,
    pub st_size: i64,
    pub st_blksize: i32,
    pub __pad2: i32,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: u64,
    pub st_mtime: i64,
    pub st_mtime_nsec: u64,
    pub st_ctime: i64,
    pub st_ctime_nsec: u64,
    pub __unused4: u32,
    pub __unused5: u32,
}

pub fn sys_fstat(fd: u64, statbuf: u64) -> u64 {
    if statbuf == 0 { return !0; }
    if fd < 3 {
        unsafe {
            core::ptr::write_bytes(statbuf as *mut u8, 0, core::mem::size_of::<LinuxStat>());
            let stat = &mut *(statbuf as *mut LinuxStat);
            stat.st_mode = 0o020666; // S_IFCHR | 0666
            stat.st_rdev = 0x0501;   // /dev/tty
            stat.st_nlink = 1;
            stat.st_blksize = 1024;
            stat.st_atime = 1700000000;
            stat.st_mtime = 1700000000;
            stat.st_ctime = 1700000000;
        }
        return 0;
    }
    if fd >= 64 { return !0; }
    unsafe {
        if let Some(ref desc) = FD_TABLE[fd as usize] {
            core::ptr::write_bytes(statbuf as *mut u8, 0, core::mem::size_of::<LinuxStat>());
            let stat = &mut *(statbuf as *mut LinuxStat);
            stat.st_dev = 0x801;
            stat.st_ino = (alloc::sync::Arc::as_ptr(&desc.node) as u64) | 1;
            stat.st_nlink = 1;
            stat.st_size = desc.node.size as i64;
            stat.st_blksize = 4096;
            stat.st_blocks = (desc.node.size as i64 + 511) / 512;
            stat.st_atime = 1700000000;
            stat.st_mtime = 1700000000;
            stat.st_ctime = 1700000000;
            match desc.node.kind {
                NodeKind::File => stat.st_mode = 0o100777,
                NodeKind::Directory => stat.st_mode = 0o040777,
                NodeKind::CharDevice => stat.st_mode = 0o020777,
                NodeKind::BlockDevice(_) => stat.st_mode = 0o060660,
                NodeKind::Socket(_) | NodeKind::UdpSocket(_, _) => stat.st_mode = 0o140777,
                NodeKind::Pipe(_) => stat.st_mode = 0o010600,
                NodeKind::SymLink(_) => stat.st_mode = 0o120777,
                NodeKind::EventFd(_) | NodeKind::Epoll(_) => stat.st_mode = 0o100600,
            }
            return 0;
        }
    }
    !0
}

fn sys_write(fd: u64, buf: u64, count: u64) -> u64 {
    if fd >= 64 || buf == 0 { return !0; }
    if count == 0 { return 0; }
    if fd == 1 || fd == 2 {
        unsafe {
            if let Some(ref mut desc) = FD_TABLE[fd as usize] {
                return write_to_desc(desc, buf, count);
            }
            let slice = core::slice::from_raw_parts(buf as *const u8, count as usize);
            for &b in slice {
                crate::hal::serial::write_byte(b);
            }
            if let Ok(s) = core::str::from_utf8(slice) {
                if let Some(req) = crate::FRAMEBUFFER_REQUEST.response() {
                    if let Some(fb) = req.framebuffers().first() {
                        crate::graphics::console_print_str(&crate::FB_INFO, fb.address() as *mut u8, s);
                    }
                }
            }
            return count;
        }
    }
    unsafe {
        if let Some(ref mut desc) = FD_TABLE[fd as usize] {
            return write_to_desc(desc, buf, count);
        }
    }
    !0
}

unsafe fn write_to_desc(desc: &mut FileDescriptor, buf: u64, count: u64) -> u64 {
    if let NodeKind::EventFd(ref lock) = desc.node.kind {
        if count < 8 || buf == 0 { return (-22i64) as u64; /* -EINVAL */ }
        let val = unsafe { *(buf as *const u64) };
        let mut state = lock.lock();
        state.counter = state.counter.saturating_add(val);
        return 8;
    }
    let buf_slice = core::slice::from_raw_parts(buf as *const u8, count as usize);
    if matches!(desc.node.kind, NodeKind::CharDevice) {
        if desc.node.name == "null" || desc.node.name == "zero" {
            return count;
        }
    }
    if let NodeKind::Pipe(ref ring_lock) = desc.node.kind {
        let mut ring = ring_lock.lock();
        let written = ring.write(buf_slice);
        if written == usize::MAX {
            return !0; // EPIPE
        }
        return written as u64;
    }
    if let NodeKind::Socket(target) = desc.node.kind {
        match target {
            crate::fs::vfs::SocketTarget::Loopback(handle) => {
                let mut sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
                let socket = sockets.get_mut::<TcpSocket>(handle);
                socket.set_nagle_enabled(false);
                if socket.can_send() {
                    let send_result = socket.send_slice(buf_slice);
                    drop(sockets);
                    for _ in 0..5 { crate::net::socket::poll(); }
                    if let Ok(bytes) = send_result {
                        return bytes as u64;
                    }
                }
                return !0;
            }
            crate::fs::vfs::SocketTarget::Ethernet(handle) => {
                let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                let socket = sockets.get_mut::<TcpSocket>(handle);
                socket.set_nagle_enabled(false);
                if socket.can_send() {
                    let send_result = socket.send_slice(buf_slice);
                    drop(sockets);
                    for _ in 0..10 { crate::net::socket::poll(); }
                    if let Ok(bytes) = send_result {
                        return bytes as u64;
                    }
                }
                return !0;
            }
            _ => return !0,
        }
    }
    if let NodeKind::UdpSocket(handle, ref peer) = desc.node.kind {
        if let Some((remote_ip, remote_port)) = *peer.lock() {
            let endpoint = smoltcp::wire::IpEndpoint::new(remote_ip, remote_port);
            let mut sockets = crate::net::socket::NET_SOCKETS.lock();
            let socket = sockets.get_mut::<smoltcp::socket::udp::Socket>(handle);
            if socket.can_send() {
                let _ = socket.send_slice(buf_slice, endpoint);
                drop(sockets);
                for _ in 0..5 { crate::net::socket::poll(); }
                return count;
            }
        }
        return !0;
    }
    if let Some(ref ops) = desc.node.file_ops {
        let write_bytes = ops.write(desc.offset, buf_slice);
        desc.offset += write_bytes;
        return write_bytes as u64;
    }
    let mut data = desc.node.data.write();
    data.extend_from_slice(buf_slice);
    desc.offset += count as usize;
    count
}

pub fn sys_pipe2(pipefd: u64, _flags: u64) -> u64 {
    if pipefd == 0 { return !0; }
    unsafe {
        let mut rfd = None;
        let mut wfd = None;
        for i in 3..64 {
            if FD_TABLE[i].is_none() {
                if rfd.is_none() {
                    rfd = Some(i);
                } else if wfd.is_none() {
                    wfd = Some(i);
                    break;
                }
            }
        }
        let (r, w) = match (rfd, wfd) {
            (Some(r), Some(w)) => (r, w),
            _ => return !0,
        };

        let ring = alloc::sync::Arc::new(spin::Mutex::new(crate::fs::vfs::PipeRingBuffer::new()));
        let r_node = alloc::sync::Arc::new(VfsNode::new(
            alloc::string::String::from("pipe_r"),
            NodeKind::Pipe(ring.clone()),
            0, 0, 0o600
        ));
        let w_node = alloc::sync::Arc::new(VfsNode::new(
            alloc::string::String::from("pipe_w"),
            NodeKind::Pipe(ring.clone()),
            0, 0, 0o600
        ));

        FD_TABLE[r] = Some(FileDescriptor::new(r_node, 0));
        FD_TABLE[w] = Some(FileDescriptor::new(w_node, 0));

        let fds_ptr = pipefd as *mut i32;
        *fds_ptr.add(0) = r as i32;
        *fds_ptr.add(1) = w as i32;
        0
    }
}

pub fn sys_socketpair(_domain: u64, _type: u64, _protocol: u64, sv_ptr: u64) -> u64 {
    if sv_ptr == 0 { return (-14i64) as u64; } // -EFAULT
    unsafe {
        let mut fd1 = None;
        let mut fd2 = None;
        for i in 3..64 {
            if FD_TABLE[i].is_none() {
                if fd1.is_none() {
                    fd1 = Some(i);
                } else if fd2.is_none() {
                    fd2 = Some(i);
                    break;
                }
            }
        }
        let (s1, s2) = match (fd1, fd2) {
            (Some(a), Some(b)) => (a, b),
            _ => return (-24i64) as u64, // -EMFILE
        };

        let mut ring = crate::fs::vfs::PipeRingBuffer::new();
        ring.readers = 2;
        ring.writers = 2;
        let ring_arc = alloc::sync::Arc::new(spin::Mutex::new(ring));

        let node1 = alloc::sync::Arc::new(VfsNode::new(
            alloc::string::String::from("sock_pair0"),
            NodeKind::Pipe(ring_arc.clone()),
            0, 0, 0o600
        ));
        let node2 = alloc::sync::Arc::new(VfsNode::new(
            alloc::string::String::from("sock_pair1"),
            NodeKind::Pipe(ring_arc.clone()),
            0, 0, 0o600
        ));

        FD_TABLE[s1] = Some(FileDescriptor::new(node1, 0));
        FD_TABLE[s2] = Some(FileDescriptor::new(node2, 0));

        let fds_ptr = sv_ptr as *mut i32;
        *fds_ptr.add(0) = s1 as i32;
        *fds_ptr.add(1) = s2 as i32;
        0
    }
}

pub fn sys_dup(oldfd: u64) -> u64 {
    if oldfd >= 64 { return !0; }
    unsafe {
        for i in 3..64 {
            if FD_TABLE[i].is_none() {
                return sys_dup3(oldfd, i as u64, 0);
            }
        }
    }
    !0
}

pub fn sys_dup3(oldfd: u64, newfd: u64, _flags: u64) -> u64 {
    if oldfd >= 64 || newfd >= 64 || oldfd == newfd { return !0; }
    unsafe {
        if FD_TABLE[oldfd as usize].is_none() { return !0; }
        if FD_TABLE[newfd as usize].is_some() {
            sys_close(newfd);
        }
        let desc = FD_TABLE[oldfd as usize].as_ref().unwrap().clone();
        if let NodeKind::Pipe(ref ring_lock) = desc.node.kind {
            let mut ring = ring_lock.lock();
            if desc.node.name == "pipe_r" {
                ring.readers += 1;
            } else if desc.node.name == "pipe_w" {
                ring.writers += 1;
            }
        }
        FD_TABLE[newfd as usize] = Some(desc);
        newfd
    }
}

pub fn get_fd_table() -> [Option<FileDescriptor>; 64] {
    unsafe { FD_TABLE.clone() }
}

pub fn set_fd_table(table: &[Option<FileDescriptor>; 64]) {
    unsafe { FD_TABLE = table.clone(); }
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
                let page_virt = crate::mm::vmm::phys_to_virt(paddr) as *mut u8;
                crate::sysmem::page_zero(page_virt, 4096);
                crate::graphics::clean_dcache_range(page_virt as usize, 4096);
                crate::mm::vmm::map_user_page(p * 4096, paddr);
            }
            HEAP_BRK = target;
        }
        HEAP_BRK as u64
    }
}

fn sys_ioctl(fd: u64, req: u64, arg: u64) -> u64 {
    if fd < 64 {
        unsafe {
            if let Some(ref desc) = FD_TABLE[fd as usize] {
                // 1. TIOCGPTN (0x80045430 or 0x5430) - get pty number
                if (req == 0x80045430 || req == 0x5430) && arg != 0 {
                    if let Some(ref pty) = desc.pty_session {
                        *(arg as *mut i32) = pty.id as i32;
                        return 0;
                    }
                }
                // 2. TIOCSPTLCK (0x40045431 or 0x5431) - unlock pty
                if (req == 0x40045431 || req == 0x5431) && arg != 0 {
                    if let Some(ref pty) = desc.pty_session {
                        let lock_val = *(arg as *const i32);
                        *pty.unlocked.write() = (lock_val == 0);
                        return 0;
                    }
                }
                // 3. TCGETS (0x5401) - get termios
                if req == 0x5401 {
                    if arg != 0 {
                        let termios = if let Some(ref pty) = desc.pty_session {
                            *pty.termios.read()
                        } else {
                            crate::fs::pty::Termios::default()
                        };
                        *(arg as *mut crate::fs::pty::Termios) = termios;
                    }
                    return 0;
                }
                // 4. TCSETS / TCSETSW / TCSETSF (0x5402..=0x5404)
                if req == 0x5402 || req == 0x5403 || req == 0x5404 {
                    if arg != 0 {
                        if let Some(ref pty) = desc.pty_session {
                            *pty.termios.write() = *(arg as *const crate::fs::pty::Termios);
                        }
                    }
                    return 0;
                }
                // 5. TIOCGWINSZ (0x5413) - get window size
                if req == 0x5413 && arg != 0 {
                    let ws = if let Some(ref pty) = desc.pty_session {
                        *pty.winsize.read()
                    } else {
                        crate::fs::pty::WinSize::default()
                    };
                    *(arg as *mut crate::fs::pty::WinSize) = ws;
                    return 0;
                }
                // 6. TIOCSWINSZ (0x5414) - set window size
                if req == 0x5414 && arg != 0 {
                    if let Some(ref pty) = desc.pty_session {
                        *pty.winsize.write() = *(arg as *const crate::fs::pty::WinSize);
                    }
                    return 0;
                }
                // 7. TIOCSCTTY (0x540E)
                if req == 0x540E {
                    return 0;
                }
                // 8. TIOCGPGRP (0x540F)
                if req == 0x540F && arg != 0 {
                    *(arg as *mut i32) = 1;
                    return 0;
                }
                // 9. TIOCSPGRP (0x5410)
                if req == 0x5410 {
                    return 0;
                }

                // Block Device & Loop Device ioctls
                if let NodeKind::BlockDevice(ref blk) = desc.node.kind {
                    // 10. BLKGETSIZE64 (0x80081272)
                    if (req == 0x80081272 || req == 0x1272) && arg != 0 {
                        *(arg as *mut u64) = blk.lock().capacity();
                        return 0;
                    }
                    // 11. BLKSSZGET (0x1268)
                    if req == 0x1268 && arg != 0 {
                        *(arg as *mut i32) = 512;
                        return 0;
                    }
                    // 12. LOOP_SET_FD (0x4C00)
                    if req == 0x4C00 {
                        let file_fd = arg as usize;
                        if file_fd < 64 {
                            if let Some(ref file_desc) = FD_TABLE[file_fd] {
                                if desc.node.name.starts_with("loop") {
                                    if let Ok(id) = desc.node.name["loop".len()..].parse::<usize>() {
                                        if id < crate::fs::loop_dev::LOOP_DEVICES.len() {
                                            let mut loop_dev = crate::fs::loop_dev::LOOP_DEVICES[id].lock();
                                            loop_dev.backing_node = Some(file_desc.node.clone());
                                            crate::serial_println!("[ioctl] Bound /dev/loop{} to '{}' (size={} B)",
                                                id, file_desc.node.name, loop_dev.capacity());
                                            return 0;
                                        }
                                    }
                                }
                            }
                        }
                        return !0;
                    }
                    // 13. LOOP_CLR_FD (0x4C01)
                    if req == 0x4C01 {
                        if desc.node.name.starts_with("loop") {
                            if let Ok(id) = desc.node.name["loop".len()..].parse::<usize>() {
                                if id < crate::fs::loop_dev::LOOP_DEVICES.len() {
                                    let mut loop_dev = crate::fs::loop_dev::LOOP_DEVICES[id].lock();
                                    loop_dev.backing_node = None;
                                    crate::serial_println!("[ioctl] Unbound /dev/loop{}", id);
                                    return 0;
                                }
                            }
                        }
                        return 0;
                    }
                    // 14. LOOP_GET_STATUS64 (0x4C05)
                    if req == 0x4C05 && arg != 0 {
                        let info = arg as *mut crate::fs::loop_dev::LoopInfo64;
                        if desc.node.name.starts_with("loop") {
                            if let Ok(id) = desc.node.name["loop".len()..].parse::<usize>() {
                                if id < crate::fs::loop_dev::LOOP_DEVICES.len() {
                                    let loop_dev = crate::fs::loop_dev::LOOP_DEVICES[id].lock();
                                    (*info).lo_device = 0;
                                    (*info).lo_inode = 0;
                                    (*info).lo_rdevice = 0;
                                    (*info).lo_offset = loop_dev.offset;
                                    (*info).lo_sizelimit = loop_dev.capacity();
                                    (*info).lo_number = id as u32;
                                    (*info).lo_flags = loop_dev.flags;
                                    return 0;
                                }
                            }
                        }
                        return 0;
                    }
                }
            }
        }
    }

    // Default console fallback
    if req == 0x5413 && arg != 0 {
        unsafe {
            *(arg as *mut crate::fs::pty::WinSize) = crate::fs::pty::WinSize::default();
        }
        return 0;
    }
    if req == 0x5401 && arg != 0 {
        unsafe {
            *(arg as *mut crate::fs::pty::Termios) = crate::fs::pty::Termios::default();
        }
        return 0;
    }
    if req == 0x540F && arg != 0 {
        unsafe { *(arg as *mut i32) = 1; }
        return 0;
    }
    0
}

fn sys_set_tid_address(tidptr: u64) -> u64 {
    let cpu = crate::hal::smp::current_cpu_id();
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    let cur = pm.current_pids[cpu];
    let cur_pid = if cur != 0 { cur } else { pm.current_pid };
    if let Some(proc) = pm.get_process_mut(cur_pid) {
        proc.clear_child_tid = tidptr;
    }
    cur_pid as u64
}

fn sys_getcwd(buf_ptr: u64, size: u64) -> u64 {
    if buf_ptr == 0 || size == 0 { return !0; }
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    let cwd = if let Some(proc) = pm.get_current_mut() {
        proc.cwd.clone()
    } else {
        alloc::string::String::from("/")
    };
    drop(pm);
    let bytes = cwd.as_bytes();
    if bytes.len() + 1 > size as usize { return !0; }
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf_ptr as *mut u8, bytes.len());
        *(buf_ptr as *mut u8).add(bytes.len()) = 0;
    }
    buf_ptr
}

pub fn resolve_path_relative(cwd: &str, path: &str) -> alloc::string::String {
    let path = path.trim();
    if path.is_empty() {
        return alloc::string::String::from(cwd);
    }
    let mut parts: alloc::vec::Vec<&str> = alloc::vec::Vec::new();
    if !path.starts_with('/') {
        for seg in cwd.split('/').filter(|s| !s.is_empty()) {
            parts.push(seg);
        }
    }
    for seg in path.split('/').filter(|s| !s.is_empty()) {
        if seg == "." {
            continue;
        } else if seg == ".." {
            parts.pop();
        } else {
            parts.push(seg);
        }
    }
    if parts.is_empty() {
        alloc::string::String::from("/")
    } else {
        let mut out = alloc::string::String::new();
        for p in parts {
            out.push('/');
            out.push_str(p);
        }
        out
    }
}

pub fn resolve_at_path(dirfd: i64, path: &str) -> alloc::string::String {
    let path = path.trim();
    if path.starts_with('/') {
        return resolve_path_relative("/", path);
    }
    if dirfd >= 0 && (dirfd as usize) < 64 {
        unsafe {
            if let Some(ref desc) = FD_TABLE[dirfd as usize] {
                if !desc.path.is_empty() {
                    return resolve_path_relative(&desc.path, path);
                }
            }
        }
    }
    let proc_cwd = {
        let pm = crate::sys::process::PROCESS_MANAGER.lock();
        let cur_pid = pm.current_pid;
        if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == cur_pid) {
            p.cwd.clone()
        } else {
            alloc::string::String::from("/")
        }
    };
    resolve_path_relative(&proc_cwd, path)
}

fn sys_chdir(pathname: u64) -> u64 {
    if pathname == 0 { return (-14i64) as u64; } // -EFAULT
    unsafe {
        let mut len = 0;
        let mut ptr = pathname as *const u8;
        while *ptr != 0 && len < 256 {
            len += 1;
            ptr = ptr.add(1);
        }
        let slice = core::slice::from_raw_parts(pathname as *const u8, len);
        if let Ok(raw_path) = core::str::from_utf8(slice) {
            let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
            let current_cwd = if let Some(proc) = pm.get_current_mut() {
                proc.cwd.clone()
            } else {
                alloc::string::String::from("/")
            };

            let resolved = resolve_path_relative(&current_cwd, raw_path);

            let node_opt = if resolved == "/" {
                Some(crate::fs::vfs::ROOT.clone())
            } else {
                crate::fs::vfs::lookup(&resolved)
            };

            if let Some(node) = node_opt {
                if matches!(node.kind, NodeKind::Directory) {
                    if let Some(proc) = pm.get_current_mut() {
                        proc.cwd = resolved;
                        return 0;
                    }
                } else {
                    return (-20i64) as u64; // -ENOTDIR
                }
            } else {
                let mounts = crate::fs::vfs::MOUNT_POINTS.read();
                let mut found_mount = false;
                for mp in mounts.iter() {
                    let mp_target = if mp.target.starts_with('/') { mp.target.clone() } else { alloc::format!("/{}", mp.target) };
                    if resolved == mp_target {
                        found_mount = true;
                        break;
                    }
                }
                drop(mounts);
                if found_mount {
                    if let Some(proc) = pm.get_current_mut() {
                        proc.cwd = resolved;
                        return 0;
                    }
                }
                return (-2i64) as u64; // -ENOENT
            }
        }
    }
    (-2i64) as u64
}

fn sys_fcntl(fd: u64, cmd: u64, arg: u64) -> u64 {
    if fd >= 64 { return !0; }
    match cmd {
        0 /* F_DUPFD */ | 1030 /* F_DUPFD_CLOEXEC */ => sys_dup(fd),
        1 /* F_GETFD */ => 0,
        2 /* F_SETFD */ => 0,
        3 /* F_GETFL */ => {
            unsafe {
                if let Some(ref desc) = FD_TABLE[fd as usize] {
                    if desc.nonblock { 2 | 0x800 } else { 2 } // O_RDWR | O_NONBLOCK
                } else { 2 }
            }
        },
        4 /* F_SETFL */ => {
            const O_NONBLOCK: u64 = 0x800;
            unsafe {
                if let Some(ref mut desc) = FD_TABLE[fd as usize] {
                    desc.nonblock = (arg & O_NONBLOCK) != 0;
                }
            }
            0
        },
        _ => 0,
    }
}

fn sys_lseek(fd: u64, offset: i64, whence: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref mut desc) = FD_TABLE[fd as usize] {
            let file_size = desc.node.size as i64;
            let new_offset = match whence {
                0 /* SEEK_SET */ => offset,
                1 /* SEEK_CUR */ => desc.offset as i64 + offset,
                2 /* SEEK_END */ => file_size + offset,
                _ => return !0,
            };
            if new_offset < 0 { return !0; }
            desc.offset = new_offset as usize;
            return desc.offset as u64;
        }
    }
    !0
}

fn sys_readlinkat(_dirfd: i64, pathname_ptr: u64, buf_ptr: u64, bufsiz: u64) -> u64 {
    if pathname_ptr == 0 || buf_ptr == 0 || bufsiz == 0 { return !0; }
    let mut len = 0;
    let mut p = pathname_ptr as *const u8;
    while unsafe { *p } != 0 {
        len += 1;
        p = unsafe { p.add(1) };
    }
    let path_slice = unsafe { core::slice::from_raw_parts(pathname_ptr as *const u8, len) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return !0,
    };

    let target = if path_str.contains("/proc/self/exe") {
        let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
        let name = if let Some(proc) = pm.get_current_mut() {
            alloc::string::String::from(proc.get_name())
        } else {
            alloc::string::String::from("/bin/bash")
        };
        drop(pm);
        name
    } else {
        let full_path = resolve_at_path(_dirfd, path_str);
        if let Some(node) = crate::fs::vfs::lookup_no_follow(&full_path) {
            if let NodeKind::SymLink(ref target) = node.kind {
                target.clone()
            } else {
                return (-22i64) as u64; // -EINVAL: not a symbolic link
            }
        } else {
            return (-2i64) as u64; // -ENOENT
        }
    };

    let tb = target.as_bytes();
    let copy_len = tb.len().min(bufsiz as usize);
    unsafe {
        core::ptr::copy_nonoverlapping(tb.as_ptr(), buf_ptr as *mut u8, copy_len);
    }
    copy_len as u64
}

fn sys_newfstatat(dirfd: i64, pathname_ptr: u64, statbuf: u64, _flags: u64) -> u64 {
    if statbuf == 0 { return !0; }
    if pathname_ptr == 0 {
        return sys_fstat(dirfd as u64, statbuf);
    }
    let mut len = 0;
    let mut ptr = pathname_ptr as *const u8;
    while unsafe { *ptr } != 0 {
        len += 1;
        ptr = unsafe { ptr.add(1) };
    }
    let path_slice = unsafe { core::slice::from_raw_parts(pathname_ptr as *const u8, len) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return !0,
    };
    if path_str.is_empty() {
        return sys_fstat(dirfd as u64, statbuf);
    }
    
    let resolved = resolve_at_path(dirfd, path_str);

    let node_opt = crate::fs::vfs::lookup(&resolved).or_else(|| {
        if resolved.starts_with("/lib/") {
            let mut p = alloc::string::String::from("/usr");
            p.push_str(&resolved);
            crate::fs::vfs::lookup(&p)
        } else if resolved.starts_with("/usr/lib/") {
            crate::fs::vfs::lookup(&resolved[4..])
        } else {
            None
        }
    });

    if let Some(node) = node_opt {
        unsafe {
            core::ptr::write_bytes(statbuf as *mut u8, 0, core::mem::size_of::<LinuxStat>());
            let stat = &mut *(statbuf as *mut LinuxStat);
            stat.st_dev = 0x801;
            stat.st_ino = (alloc::sync::Arc::as_ptr(&node) as u64) | 1;
            stat.st_nlink = 1;
            stat.st_size = node.size as i64;
            stat.st_blksize = 4096;
            stat.st_blocks = (node.size as i64 + 511) / 512;
            stat.st_atime = 1700000000;
            stat.st_mtime = 1700000000;
            stat.st_ctime = 1700000000;
            match node.kind {
                NodeKind::File => stat.st_mode = 0o100777,
                NodeKind::Directory => stat.st_mode = 0o040777,
                NodeKind::CharDevice => stat.st_mode = 0o020777,
                NodeKind::BlockDevice(_) => stat.st_mode = 0o060660,
                NodeKind::Socket(_) | NodeKind::UdpSocket(_, _) => stat.st_mode = 0o140777,
                NodeKind::Pipe(_) => stat.st_mode = 0o010600,
                NodeKind::SymLink(_) => stat.st_mode = 0o120777,
                NodeKind::EventFd(_) | NodeKind::Epoll(_) => stat.st_mode = 0o100600,
            }
        }
        0
    } else {
        (-2i64) as u64 // -ENOENT
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __statx_pad1: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinuxStatx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    pub __spare0: [u16; 1],
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: StatxTimestamp,
    pub stx_btime: StatxTimestamp,
    pub stx_ctime: StatxTimestamp,
    pub stx_mtime: StatxTimestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub stx_mnt_id: u64,
    pub stx_dio_mem_align: u32,
    pub stx_dio_offset_align: u32,
    pub __spare2: [u64; 12],
}

pub fn sys_statx(dirfd: i64, pathname_ptr: u64, _flags: i32, _mask: u32, statxbuf: u64) -> u64 {
    if statxbuf == 0 { return (-14i64) as u64; } // -EFAULT
    if pathname_ptr == 0 {
        return (-14i64) as u64;
    }
    let mut len = 0;
    let mut ptr = pathname_ptr as *const u8;
    while unsafe { *ptr } != 0 {
        len += 1;
        ptr = unsafe { ptr.add(1) };
    }
    let path_slice = unsafe { core::slice::from_raw_parts(pathname_ptr as *const u8, len) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return (-14i64) as u64,
    };
    let resolved = resolve_at_path(dirfd, path_str);
    let node_opt = crate::fs::vfs::lookup(&resolved).or_else(|| {
        if resolved.starts_with("/lib/") {
            let mut p = alloc::string::String::from("/usr");
            p.push_str(&resolved);
            crate::fs::vfs::lookup(&p)
        } else if resolved.starts_with("/usr/lib/") {
            crate::fs::vfs::lookup(&resolved[4..])
        } else {
            None
        }
    });

    if let Some(node) = node_opt {
        unsafe {
            core::ptr::write_bytes(statxbuf as *mut u8, 0, core::mem::size_of::<LinuxStatx>());
            let sx = &mut *(statxbuf as *mut LinuxStatx);
            sx.stx_mask = 0x7FF; // STATX_BASIC_STATS
            sx.stx_blksize = 4096;
            sx.stx_nlink = 1;
            sx.stx_ino = (alloc::sync::Arc::as_ptr(&node) as u64) | 1;
            sx.stx_size = node.size as u64;
            sx.stx_blocks = (node.size as u64 + 511) / 512;
            sx.stx_atime.tv_sec = 1700000000;
            sx.stx_mtime.tv_sec = 1700000000;
            sx.stx_ctime.tv_sec = 1700000000;
            sx.stx_btime.tv_sec = 1700000000;
            match node.kind {
                NodeKind::File => sx.stx_mode = 0o100777,
                NodeKind::Directory => sx.stx_mode = 0o040777,
                NodeKind::CharDevice => sx.stx_mode = 0o020777,
                NodeKind::BlockDevice(_) => sx.stx_mode = 0o060660,
                NodeKind::Socket(_) | NodeKind::UdpSocket(_, _) => sx.stx_mode = 0o140777,
                NodeKind::Pipe(_) => sx.stx_mode = 0o010600,
                NodeKind::SymLink(_) => sx.stx_mode = 0o120777,
                NodeKind::EventFd(_) | NodeKind::Epoll(_) => sx.stx_mode = 0o100600,
            }
        }
        0
    } else {
        (-2i64) as u64 // -ENOENT
    }
}

fn sys_faccessat(dirfd: i64, pathname_ptr: u64, mode: i32, _flags: i32) -> u64 {
    if pathname_ptr == 0 {
        return (-14i64) as u64; // -EFAULT
    }
    let mut len: u16 = 0;
    let mut ptr = pathname_ptr as *const u8;
    while unsafe { *ptr } != 0 && len < 4096 {
        len += 1;
        ptr = unsafe { ptr.add(1) };
    }
    let path_slice = unsafe { core::slice::from_raw_parts(pathname_ptr as *const u8, len as usize) };
    let path_str = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return (-14i64) as u64, // -EFAULT
    };
    if path_str.is_empty() {
        return (-2i64) as u64; // -ENOENT
    }

    let resolved = resolve_at_path(dirfd, path_str);

    let node_opt = crate::fs::vfs::lookup(&resolved).or_else(|| {
        if resolved.starts_with("/lib/") {
            let mut p = alloc::string::String::from("/usr");
            p.push_str(&resolved);
            crate::fs::vfs::lookup(&p)
        } else if resolved.starts_with("/usr/lib/") {
            crate::fs::vfs::lookup(&resolved[4..])
        } else {
            None
        }
    });

    if let Some(node) = node_opt {
        let (euid, egid) = {
            let pm = crate::sys::process::PROCESS_MANAGER.lock();
            let cur_pid = pm.current_pid;
            if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == cur_pid) {
                (p.euid, p.egid)
            } else {
                (0, 0)
            }
        };

        let want_read = (mode & 4) != 0;
        let want_write = (mode & 2) != 0;
        let want_exec = (mode & 1) != 0;

        if node.check_permission(euid, egid, want_read, want_write, want_exec) {
            0
        } else {
            (-13i64) as u64 // -EACCES
        }
    } else {
        (-2i64) as u64 // -ENOENT
    }
}

fn sys_nanosleep(req_ptr: u64, _rem_ptr: u64) -> u64 {
    if req_ptr == 0 { return (-14i64) as u64; /* -EFAULT */ }
    let (sec, nsec) = unsafe {
        let p = req_ptr as *const u64;
        (*p.add(0), *p.add(1))
    };
    if nsec >= 1_000_000_000 || sec > 86400 {
        return (-22i64) as u64; /* -EINVAL */
    }
    let mut freq: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) freq);
    }
    if freq > 0 {
        let total_ticks = sec * freq + (nsec * freq) / 1_000_000_000;
        let mut start: u64 = 0;
        unsafe { core::arch::asm!("mrs {0}, cntvct_el0", out(reg) start); }
        let mut now = start;
        while now.saturating_sub(start) < total_ticks {
            crate::net::socket::poll();
            crate::sys::process::schedule();
            for _ in 0..1_000 {
                core::hint::spin_loop();
            }
            unsafe {
                core::arch::asm!("mrs {0}, cntvct_el0", out(reg) now);
            }
        }
        crate::net::socket::poll();
    }
    0
}

fn sys_gettimeofday(tv_ptr: u64, _tz_ptr: u64) -> u64 {
    if tv_ptr == 0 { return !0; }
    let mut count: u64 = 0;
    let mut freq: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {0}, cntvct_el0", out(reg) count);
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) freq);
        if freq > 0 {
            let sec = count / freq;
            let usec = ((count % freq) * 1_000_000) / freq;
            let ptr = tv_ptr as *mut u64;
            *ptr.add(0) = sec;
            *ptr.add(1) = usec;
        }
    }
    0
}

fn sys_prlimit64(_pid: i64, resource: u64, _new_limit: u64, old_limit: u64) -> u64 {
    if old_limit != 0 {
        #[repr(C)]
        struct Rlimit {
            rlim_cur: u64,
            rlim_max: u64,
        }
        let (cur, max) = match resource {
            3 /* RLIMIT_STACK */ => (8 * 1024 * 1024, 8 * 1024 * 1024),
            7 /* RLIMIT_NOFILE */ => (1024, 4096),
            _ => (0xffff_ffff, 0xffff_ffff),
        };
        unsafe {
            let r = old_limit as *mut Rlimit;
            (*r).rlim_cur = cur;
            (*r).rlim_max = max;
        }
    }
    0
}

fn sys_getrandom(buf: u64, buflen: u64, _flags: u64) -> u64 {
    if buf == 0 { return !0; }
    let mut count: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {0}, cntvct_el0", out(reg) count);
    }
    let mut state = count ^ 0x5deece66d;
    let ptr = buf as *mut u8;
    for i in 0..buflen {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let byte = (state >> 33) as u8;
        unsafe {
            *ptr.add(i as usize) = byte;
        }
    }
    buflen
}

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

pub fn sys_getpid() -> u64 {
    let cpu = crate::hal::smp::current_cpu_id();
    let pm = crate::sys::process::PROCESS_MANAGER.lock();
    let cur = pm.current_pids[cpu];
    let target_pid = if cur != 0 { cur } else { pm.current_pid };
    if let Some(proc) = pm.procs.iter().flatten().find(|p| p.pid == target_pid) {
        proc.tgid as u64
    } else {
        target_pid as u64
    }
}

pub fn sys_gettid() -> u64 {
    let cpu = crate::hal::smp::current_cpu_id();
    let pm = crate::sys::process::PROCESS_MANAGER.lock();
    let cur = pm.current_pids[cpu];
    if cur != 0 {
        cur as u64
    } else {
        pm.current_pid as u64
    }
}

pub fn sys_getppid() -> u64 {
    crate::sys::process::PROCESS_MANAGER.lock().get_current_mut().map_or(0, |p| p.ppid as u64)
}

pub fn sys_getuid() -> u64 {
    crate::sys::process::PROCESS_MANAGER.lock().get_current_mut().map_or(0, |p| p.uid as u64)
}

pub fn sys_getgid() -> u64 {
    crate::sys::process::PROCESS_MANAGER.lock().get_current_mut().map_or(0, |p| p.gid as u64)
}

pub fn sys_setuid(uid: u64) -> u64 {
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    if let Some(p) = pm.get_current_mut() {
        p.uid = uid as u32;
        p.euid = uid as u32;
        0
    } else {
        !0
    }
}

pub fn sys_setgid(gid: u64) -> u64 {
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    if let Some(p) = pm.get_current_mut() {
        p.gid = gid as u32;
        p.egid = gid as u32;
        0
    } else {
        !0
    }
}

pub fn sys_fchmodat(_dirfd: u64, pathname: u64, mode: u64) -> u64 {
    if pathname == 0 { return !0; }
    unsafe {
        let mut len = 0;
        let mut ptr = pathname as *const u8;
        while *ptr != 0 && len < 256 {
            len += 1;
            ptr = ptr.add(1);
        }
        let path_bytes = core::slice::from_raw_parts(pathname as *const u8, len);
        if let Ok(raw_path) = core::str::from_utf8(path_bytes) {
            let path_str = raw_path.trim_start_matches('.');
            if let Some(node) = crate::fs::vfs::lookup(path_str) {
                let pm = crate::sys::process::PROCESS_MANAGER.lock();
                let cur_pid = pm.current_pid;
                let euid = if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == cur_pid) {
                    p.euid
                } else {
                    0
                };
                drop(pm);
                if euid == 0 || euid == *node.uid.read() {
                    *node.mode.write() = (mode & 0o777) as u32;
                    return 0;
                } else {
                    return !0; // EPERM
                }
            }
        }
    }
    !0
}

pub fn sys_fchownat(_dirfd: u64, pathname: u64, owner: u64, group: u64, _flags: u64) -> u64 {
    if pathname == 0 { return !0; }
    unsafe {
        let mut len = 0;
        let mut ptr = pathname as *const u8;
        while *ptr != 0 && len < 256 {
            len += 1;
            ptr = ptr.add(1);
        }
        let path_bytes = core::slice::from_raw_parts(pathname as *const u8, len);
        if let Ok(raw_path) = core::str::from_utf8(path_bytes) {
            let path_str = raw_path.trim_start_matches('.');
            if let Some(node) = crate::fs::vfs::lookup(path_str) {
                let pm = crate::sys::process::PROCESS_MANAGER.lock();
                let cur_pid = pm.current_pid;
                let euid = if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == cur_pid) {
                    p.euid
                } else {
                    0
                };
                drop(pm);
                if euid == 0 {
                    if owner != !0 && owner != 0xFFFF_FFFF {
                        *node.uid.write() = owner as u32;
                    }
                    if group != !0 && group != 0xFFFF_FFFF {
                        *node.gid.write() = group as u32;
                    }
                    return 0;
                } else {
                    return !0; // EPERM
                }
            }
        }
    }
    !0
}

pub fn sys_exit_group(code: u64) -> u64 {
    let cpu = crate::hal::smp::current_cpu_id();
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    let cur = pm.current_pids[cpu];
    let cur_pid = if cur != 0 { cur } else { pm.current_pid };
    let cur_tgid = pm.get_process_mut(cur_pid).map(|p| p.tgid).unwrap_or(cur_pid);
    let cur_ttbr0 = pm.get_process_mut(cur_pid).map(|p| p.ttbr0).unwrap_or(0);

    for i in 0..crate::sys::process::MAX_PROCS {
        if let Some(ref mut proc) = pm.procs[i] {
            if proc.tgid == cur_tgid || (cur_ttbr0 != 0 && proc.ttbr0 == cur_ttbr0) {
                proc.state = crate::sys::process::ProcessState::Zombie;
                proc.exit_code = code as i32;
                proc.running_cpu.store(-1, core::sync::atomic::Ordering::Release);
                let clear_tid = proc.clear_child_tid;
                if clear_tid != 0 {
                    unsafe {
                        *(clear_tid as *mut i32) = 0;
                    }
                    crate::sys::futex::futex_wake(clear_tid, 1, 0xFFFF_FFFF);
                }
            }
        }
    }
    drop(pm);
    crate::serial_println!("[Process/Thread] TGID {} exited with code {}", cur_tgid, code);
    crate::sys::process::schedule();
    loop {
        unsafe { core::arch::asm!("wfe"); }
    }
}

pub fn sys_exit(code: u64) -> u64 {
    let cpu = crate::hal::smp::current_cpu_id();
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    let cur = pm.current_pids[cpu];
    let cur_pid = if cur != 0 { cur } else { pm.current_pid };

    let is_thread = pm.get_process_mut(cur_pid).map(|p| p.is_thread).unwrap_or(false);
    if !is_thread {
        drop(pm);
        return sys_exit_group(code);
    }

    let mut clear_tid = 0u64;
    if let Some(proc) = pm.get_process_mut(cur_pid) {
        proc.state = crate::sys::process::ProcessState::Zombie;
        proc.exit_code = code as i32;
        proc.running_cpu.store(-1, core::sync::atomic::Ordering::Release);
        clear_tid = proc.clear_child_tid;
        crate::serial_println!("[Process/Thread] TID {} exited with code {}", cur_pid, code);
    }
    drop(pm);

    // If CLONE_CHILD_CLEARTID was set (e.g. pthread_join), clear it and wake waiters
    if clear_tid != 0 {
        unsafe {
            *(clear_tid as *mut i32) = 0;
        }
        crate::sys::futex::futex_wake(clear_tid, 1, 0xFFFF_FFFF);
    }

    crate::sys::process::schedule();
    loop {
        unsafe { core::arch::asm!("wfe"); }
    }
}


pub fn sys_wait4(pid: i64, status_ptr: u64, _options: u64) -> u64 {
    crate::serial_println!("[Wait4] PID {} waiting for child {}", crate::sys::process::PROCESS_MANAGER.lock().current_pid, pid);
    loop {
        let cpu = crate::hal::smp::current_cpu_id();
        let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
        let cur = pm.current_pids[cpu];
        let cur_pid = if cur != 0 { cur } else { pm.current_pid };

        let mut found_child = false;
        let mut zombie_idx = None;

        for i in 0..crate::sys::process::MAX_PROCS {
            if let Some(ref p) = pm.procs[i] {
                if p.ppid == cur_pid && (pid == -1 || p.pid == pid as usize) {
                    found_child = true;
                    if p.state == crate::sys::process::ProcessState::Zombie {
                        zombie_idx = Some(i);
                        break;
                    }
                }
            }
        }

        if let Some(idx) = zombie_idx {
            let zombie = pm.procs[idx].take().unwrap();
            let zpid = zombie.pid;
            let z_tgid = zombie.tgid;
            let status = (zombie.exit_code & 0xFF) << 8;
            let z_ttbr0 = zombie.ttbr0;
            let z_kstack = zombie.kstack;
            let is_thread = zombie.is_thread;

            if !is_thread {
                for i in 0..crate::sys::process::MAX_PROCS {
                    let should_clean = if let Some(ref p) = pm.procs[i] {
                        p.pid != zpid && (p.tgid == z_tgid || (z_ttbr0 != 0 && p.ttbr0 == z_ttbr0))
                    } else {
                        false
                    };
                    if should_clean {
                        let thread_proc = pm.procs[i].take().unwrap();
                        let th_kstack = thread_proc.kstack;
                        if th_kstack != 0 {
                            let kstack_phys = unsafe { crate::mm::vmm::virt_to_phys(th_kstack) };
                            if kstack_phys != 0 {
                                let kstack_pages = crate::sys::process::KSTACK_SIZE / 4096;
                                crate::mm::pmm::free_frames(kstack_phys, kstack_pages);
                            }
                        }
                    }
                }
            }

            drop(pm);
            crate::serial_println!("[Wait4] Reaped child PID {}", zpid);

            if !is_thread && z_ttbr0 != 0 {
                unsafe {
                    crate::mm::vmm::destroy_user_address_space(z_ttbr0);
                }
            }
            if z_kstack != 0 {
                let kstack_phys = unsafe { crate::mm::vmm::virt_to_phys(z_kstack) };
                if kstack_phys != 0 {
                    let kstack_pages = crate::sys::process::KSTACK_SIZE / 4096;
                    crate::mm::pmm::free_frames(kstack_phys, kstack_pages);
                }
            }

            if status_ptr != 0 {
                unsafe {
                    *(status_ptr as *mut i32) = status;
                }
            }
            return zpid as u64;
        }

        if !found_child {
            crate::serial_println!("[Wait4] No child found for PID {} matching {}", cur_pid, pid);
            return !0; // ECHILD
        }

        drop(pm);
        crate::sys::process::schedule();
    }
}

pub fn sys_kill(pid: i64, sig: i32) -> u64 {
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    let cur_pid = pm.current_pid;
    if sig == 0 {
        for p in pm.procs.iter().flatten() {
            if p.pid == pid as usize { return 0; }
        }
        return !0;
    }
    for p in pm.procs.iter_mut().flatten() {
        if p.pid == pid as usize {
            match sig {
                9 | 15 | 2 => {
                    p.state = crate::sys::process::ProcessState::Zombie;
                    p.exit_code = 128 + sig;
                    crate::serial_println!("[Signal] PID {} terminated by signal {}", pid, sig);
                    let should_schedule = (pid as usize == cur_pid);
                    drop(pm);
                    if should_schedule {
                        crate::sys::process::schedule();
                    }
                    return 0;
                }
                19 => {
                    p.state = crate::sys::process::ProcessState::Blocked;
                    crate::serial_println!("[Signal] PID {} stopped by SIGSTOP", pid);
                    return 0;
                }
                18 => {
                    p.state = crate::sys::process::ProcessState::Ready;
                    crate::serial_println!("[Signal] PID {} resumed by SIGCONT", pid);
                    return 0;
                }
                _ => {
                    crate::serial_println!("[Signal] PID {} received signal {}", pid, sig);
                    return 0;
                }
            }
        }
    }
    !0
}

pub fn sys_rt_sigaction(_sig: i32, _act: u64, _oldact: u64, _sigsetsize: u64) -> u64 {
    0
}

pub fn sys_rt_sigprocmask(_how: i32, _set: u64, _oldset: u64, _sigsetsize: u64) -> u64 {
    0
}

pub fn sys_clone_ctx(
    flags: u64,
    newsp: u64,
    parent_tidptr: u64,
    tls: u64,
    child_tidptr: u64,
    context: &crate::hal::exceptions::ExceptionContext,
) -> u64 {
    const CLONE_VM: u64            = 0x0000_0100;
    const CLONE_FS: u64            = 0x0000_0200;
    const CLONE_FILES: u64         = 0x0000_0400;
    const CLONE_SIGHAND: u64       = 0x0000_0800;
    const CLONE_THREAD: u64        = 0x0001_0000;
    const CLONE_SETTLS: u64        = 0x0008_0000;
    const CLONE_PARENT_SETTID: u64 = 0x0010_0000;
    const CLONE_CHILD_CLEARTID: u64= 0x0020_0000;
    const CLONE_CHILD_SETTID: u64  = 0x0100_0000;

    let is_thread = (flags & CLONE_VM) != 0;

    let cpu = crate::hal::smp::current_cpu_id();
    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    let parent_pid = {
        let cur = pm.current_pids[cpu];
        if cur != 0 { cur } else { pm.current_pid }
    };
    let parent_idx = match pm.procs.iter().position(|p| p.as_ref().map_or(false, |proc| proc.pid == parent_pid)) {
        Some(idx) => idx,
        None => return !0,
    };

    let free_slot = match pm.find_free_slot() {
        Some(slot) => slot,
        None => return !0,
    };

    let child_pid = match pm.allocate_pid() {
        Some(pid) => pid,
        None => return !0,
    };

    let parent_proc = pm.procs[parent_idx].as_ref().unwrap();
    let parent_tgid = parent_proc.tgid;
    let parent_ttbr0 = parent_proc.ttbr0;
    let parent_cwd = parent_proc.cwd.clone();
    let parent_name = parent_proc.name;
    let parent_uid = parent_proc.uid;
    let parent_gid = parent_proc.gid;
    let parent_euid = parent_proc.euid;
    let parent_egid = parent_proc.egid;
    let parent_brk = parent_proc.brk;
    let parent_fds = get_fd_table();

    let child_tgid = if (flags & CLONE_THREAD) != 0 {
        parent_tgid
    } else {
        child_pid
    };

    if !is_thread {
        // Increment pipe reader/writer reference counts for inherited descriptors on fork
        for fd_opt in &parent_fds {
            if let Some(ref desc) = fd_opt {
                if let NodeKind::Pipe(ref ring_lock) = desc.node.kind {
                    let mut ring = ring_lock.lock();
                    if desc.node.name == "pipe_r" {
                        ring.readers += 1;
                    } else if desc.node.name == "pipe_w" {
                        ring.writers += 1;
                    }
                }
            }
        }
    }

    drop(pm);

    // If thread (CLONE_VM): share address space (TTBR0)
    // If process (fork): duplicate address space via SIMD page copy
    let child_ttbr0 = if is_thread {
        parent_ttbr0
    } else {
        unsafe { crate::mm::vmm::clone_user_address_space(parent_ttbr0) }
    };

    let mut child = crate::sys::process::Process::new(
        child_pid,
        parent_pid,
        child_ttbr0,
        if is_thread { "thread" } else { "forked" },
    );
    child.tgid = child_tgid;
    child.is_thread = is_thread;
    child.cwd = parent_cwd;
    child.name = parent_name;
    child.uid = parent_uid;
    child.gid = parent_gid;
    child.euid = parent_euid;
    child.egid = parent_egid;
    child.brk = parent_brk;
    child.fd_table = parent_fds.clone();

    if (flags & CLONE_SETTLS) != 0 {
        child.tls = tls;
    }
    if (flags & CLONE_CHILD_CLEARTID) != 0 {
        child.clear_child_tid = child_tidptr;
    }

    // Set up child kernel stack with parent ExceptionContext
    let ctx_size = core::mem::size_of::<crate::hal::exceptions::ExceptionContext>();
    let child_sp = child.kstack_top - ctx_size;
    let child_ctx_ptr = child_sp as *mut crate::hal::exceptions::ExceptionContext;

    unsafe {
        core::ptr::copy_nonoverlapping(context as *const _, child_ctx_ptr, 1);
        (*child_ctx_ptr).x[0] = 0; // Child returns 0
        if is_thread && newsp != 0 {
            (*child_ctx_ptr).sp = newsp; // Child userspace stack pointer
        }
    }

    child.cpu_context.sp = child_sp as u64;
    child.cpu_context.x30 = crate::sys::process::return_from_fork_trampoline as *const () as usize as u64;

    // Write TID to parent_tidptr if requested
    if (flags & CLONE_PARENT_SETTID) != 0 && parent_tidptr != 0 {
        unsafe {
            *(parent_tidptr as *mut i32) = child_pid as i32;
        }
    }

    // Write TID to child_tidptr if requested
    if (flags & CLONE_CHILD_SETTID) != 0 && child_tidptr != 0 {
        unsafe {
            *(child_tidptr as *mut i32) = child_pid as i32;
        }
    }

    let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
    if let Some(parent) = pm.procs[parent_idx].as_mut() {
        parent.fd_table = parent_fds;
    }
    pm.procs[free_slot] = Some(child);
    unsafe {
        core::arch::asm!("sev");
    }

    if is_thread {
        crate::serial_println!(
            "[Clone] Process PID {} spawned POSIX thread TID {} (newsp={:#x}, tls={:#x})",
            parent_pid,
            child_pid,
            newsp,
            tls
        );
    } else {
        crate::serial_println!(
            "[Fork] Parent PID {} forked child PID {}",
            parent_pid,
            child_pid
        );
    }

    child_pid as u64
}

pub fn sys_fork(context: &crate::hal::exceptions::ExceptionContext) -> u64 {
    sys_clone_ctx(0, 0, 0, 0, 0, context)
}


pub fn sys_execve_ctx(filename_ptr: u64, argv_ptr: u64, envp_ptr: u64, context: &mut crate::hal::exceptions::ExceptionContext) -> u64 {
    if filename_ptr == 0 { return !0; }
    let mut len = 0;
    let mut ptr = filename_ptr as *const u8;
    while unsafe { *ptr } != 0 {
        len += 1;
        ptr = unsafe { ptr.add(1) };
    }
    let path_slice = unsafe { core::slice::from_raw_parts(filename_ptr as *const u8, len) };
    let path_str_raw = match core::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return !0,
    };
    let path_string = alloc::string::String::from(path_str_raw);
    let path_str = path_string.as_str();

    let node = match crate::fs::vfs::lookup(path_str) {
        Some(n) => n,
        None => return !0,
    };

    // Safely parse argv from caller user memory
    let mut args = alloc::vec::Vec::new();
    if argv_ptr != 0 {
        let mut i = 0;
        while i < 64 {
            let arg_p = unsafe { *((argv_ptr as *const u64).add(i)) };
            if arg_p == 0 { break; }
            let mut arg_len = 0;
            let mut p = arg_p as *const u8;
            while unsafe { *p } != 0 && arg_len < 4096 {
                arg_len += 1;
                p = unsafe { p.add(1) };
            }
            let s_slice = unsafe { core::slice::from_raw_parts(arg_p as *const u8, arg_len) };
            if let Ok(s) = core::str::from_utf8(s_slice) {
                args.push(alloc::string::String::from(s));
            }
            i += 1;
        }
    }
    if args.is_empty() {
        args.push(alloc::string::String::from(path_str));
    }

    // Safely parse envp from caller user memory
    let mut envs = alloc::vec::Vec::new();
    if envp_ptr != 0 {
        let mut i = 0;
        while i < 64 {
            let env_p = unsafe { *((envp_ptr as *const u64).add(i)) };
            if env_p == 0 { break; }
            let mut env_len = 0;
            let mut p = env_p as *const u8;
            while unsafe { *p } != 0 && env_len < 4096 {
                env_len += 1;
                p = unsafe { p.add(1) };
            }
            let s_slice = unsafe { core::slice::from_raw_parts(env_p as *const u8, env_len) };
            if let Ok(s) = core::str::from_utf8(s_slice) {
                envs.push(alloc::string::String::from(s));
            }
            i += 1;
        }
    }
    if envs.is_empty() {
        envs.push(alloc::string::String::from("PATH=/bin:/usr/bin"));
        envs.push(alloc::string::String::from("LD_LIBRARY_PATH=/lib:/usr/lib"));
        envs.push(alloc::string::String::from("USER=root"));
        envs.push(alloc::string::String::from("HOME=/root"));
        envs.push(alloc::string::String::from("TERM=xterm-256color"));
    } else if !envs.iter().any(|e| e.starts_with("LD_LIBRARY_PATH=")) {
        envs.push(alloc::string::String::from("LD_LIBRARY_PATH=/lib:/usr/lib"));
    }

    let new_ttbr0 = crate::mm::vmm::create_user_address_space();
    unsafe {
        MMAP_NEXT = 0x4000_0000_0000;
    }
    let (entry, sp) = if let Some(data) = node.static_data {
        if data.len() < 4 || &data[0..4] != b"\x7fELF" {
            crate::serial_println!("[Execve FAIL] Not a valid ELF binary");
            return !0;
        }
        match crate::sys::elf::load_elf_image(new_ttbr0, data, path_str, &args, &envs) {
            Ok(res) => res,
            Err(e) => {
                crate::serial_println!("[Execve FAIL] load_elf_image failed: {}", e);
                return !0;
            }
        }
    } else {
        let file_bytes = node.data.read();
        if file_bytes.len() < 4 || &file_bytes[0..4] != b"\x7fELF" {
            crate::serial_println!("[Execve FAIL] Not a valid ELF binary");
            return !0;
        }
        match crate::sys::elf::load_elf_image(new_ttbr0, &file_bytes, path_str, &args, &envs) {
            Ok(res) => res,
            Err(e) => {
                crate::serial_println!("[Execve FAIL] load_elf_image failed: {}", e);
                return !0;
            }
        }
    };

    let old_ttbr0 = {
        let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
        if let Some(proc) = pm.get_current_mut() {
            let old = proc.ttbr0;
            proc.ttbr0 = new_ttbr0;
            proc.entry_point = entry;
            proc.ustack_top = sp;
            
            let mut name = [0u8; 32];
            let pb = path_str.as_bytes();
            let cp = pb.len().min(31);
            name[..cp].copy_from_slice(&pb[..cp]);
            proc.name = name;
            old
        } else {
            0
        }
    };

    unsafe {
        core::arch::asm!(
            "msr ttbr0_el1, {0}",
            "dsb ish",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            in(reg) new_ttbr0
        );

        if old_ttbr0 != 0 && old_ttbr0 != new_ttbr0 {
            crate::mm::vmm::destroy_user_address_space(old_ttbr0);
        }
    }

    context.elr = entry as u64;
    context.sp = sp as u64;
    context.x[0] = 0;

    crate::serial_println!("[Execve] Process PID replaced with {}", path_str);
    0
}

static mut MMAP_NEXT: usize = 0x4000_0000_0000;

fn sys_mmap(addr: u64, len: u64, prot: u64, flags: u64, fd: u64, offset: u64) -> u64 {
    let pages = (len + 4095) / 4096;
    if pages == 0 { return !0; }
    let phys = match crate::mm::pmm::alloc_frames(pages as usize) {
        Some(p) => p,
        None => return !0,
    };
    let page_virt = unsafe { crate::mm::vmm::phys_to_virt(phys) as *mut u8 };
    unsafe {
        crate::sysmem::page_zero(page_virt, (pages * 4096) as usize);
    }

    // Check if file-backed mapping (MAP_ANONYMOUS == 0x20 is not set)
    if (flags & 0x20) == 0 && (fd as usize) < 64 {
        unsafe {
            if let Some(ref desc) = FD_TABLE[fd as usize] {
                let dest_slice = core::slice::from_raw_parts_mut(page_virt, (pages * 4096) as usize);
                let copy_limit = core::cmp::min(len as usize, dest_slice.len());
                if let Some(ref ops) = desc.node.file_ops {
                    let _ = ops.read(offset as usize, &mut dest_slice[..copy_limit]);
                } else if let Some(data) = desc.node.static_data {
                    if (offset as usize) < data.len() {
                        let avail = data.len() - (offset as usize);
                        let to_copy = core::cmp::min(avail, copy_limit);
                        dest_slice[..to_copy].copy_from_slice(&data[offset as usize..offset as usize + to_copy]);
                    }
                } else {
                    let data = desc.node.data.read();
                    if (offset as usize) < data.len() {
                        let avail = data.len() - (offset as usize);
                        let to_copy = core::cmp::min(avail, copy_limit);
                        dest_slice[..to_copy].copy_from_slice(&data[offset as usize..offset as usize + to_copy]);
                    }
                }
                crate::graphics::clean_dcache_range(page_virt as usize, (pages * 4096) as usize);
            }
        }
    }

    let virt = if addr == 0 {
        unsafe {
            let v = MMAP_NEXT;
            MMAP_NEXT += (pages * 4096) as usize;
            v as u64
        }
    } else {
        unsafe {
            let end = (addr as usize) + (pages * 4096) as usize;
            if end > MMAP_NEXT {
                MMAP_NEXT = end;
            }
        }
        addr
    };

    let writable = (prot & 2) != 0 || prot == 0;
    let executable = (prot & 4) != 0;

    for i in 0..pages {
        let page_off = (i * 4096) as usize;
        unsafe {
            crate::mm::vmm::map_user_page_flags((virt + page_off as u64) as usize, phys + page_off, writable, executable);
        }
    }
    virt
}

fn sys_uname(buf: u64) -> u64 {
    if buf == 0 { return !0; }
    #[repr(C)]
    struct UtsName {
        sysname: [u8; 65],
        nodename: [u8; 65],
        release: [u8; 65],
        version: [u8; 65],
        machine: [u8; 65],
        domainname: [u8; 65],
    }
    unsafe {
        core::ptr::write_bytes(buf as *mut u8, 0, core::mem::size_of::<UtsName>());
        let u = &mut *(buf as *mut UtsName);
        
        let sysname = b"Linux\0";
        u.sysname[..sysname.len()].copy_from_slice(sysname);
        
        let nodename = b"himada-server\0";
        u.nodename[..nodename.len()].copy_from_slice(nodename);
        
        let release = b"6.8.0-himada-server\0";
        u.release[..release.len()].copy_from_slice(release);
        
        let version = b"#1 SMP PREEMPT_DYNAMIC Sat Sep 12 23:55:00 UTC 2026\0";
        u.version[..version.len()].copy_from_slice(version);
        
        let machine = b"aarch64\0";
        u.machine[..machine.len()].copy_from_slice(machine);
        
        let domainname = b"(none)\0";
        u.domainname[..domainname.len()].copy_from_slice(domainname);
    }
    0
}

fn sys_get_fb_info(_ptr: u64) -> u64 { 0 }

fn sys_socket(domain: u64, ty: u64, _protocol: u64) -> u64 {
    if domain == 2 && ty == 1 {
        let rx_lo = SocketBuffer::new(alloc::vec![0; 4096]);
        let tx_lo = SocketBuffer::new(alloc::vec![0; 4096]);
        let mut sock_lo = TcpSocket::new(rx_lo, tx_lo);
        sock_lo.set_nagle_enabled(false);
        let handle_lo = crate::net::socket::LOOPBACK_SOCKETS.lock().add(sock_lo);

        let rx_eth = SocketBuffer::new(alloc::vec![0; 4096]);
        let tx_eth = SocketBuffer::new(alloc::vec![0; 4096]);
        let mut sock_eth = TcpSocket::new(rx_eth, tx_eth);
        sock_eth.set_nagle_enabled(false);
        let handle_eth = crate::net::socket::NET_SOCKETS.lock().add(sock_eth);

        let target = crate::fs::vfs::SocketTarget::Dual {
            lo: handle_lo,
            eth: handle_eth,
        };
        
        unsafe {
            for i in 3..64 {
                if FD_TABLE[i].is_none() {
                    let node = alloc::sync::Arc::new(VfsNode {
                        name: alloc::string::String::from("socket"),
                        kind: NodeKind::Socket(target),
                        size: 0,
                        children: spin::RwLock::new(alloc::vec::Vec::new()),
                        file_ops: None,
                        data: spin::RwLock::new(alloc::vec::Vec::new()),
                        static_data: None,
                        uid: spin::RwLock::new(0),
                        gid: spin::RwLock::new(0),
                        mode: spin::RwLock::new(0o666),
                    });
                    FD_TABLE[i] = Some(FileDescriptor::new(node, 0));
                    return i as u64;
                }
            }
        }
    }
    if domain == 2 && ty == 2 {
        use smoltcp::socket::udp::{Socket as UdpSocket, PacketBuffer as UdpPacketBuffer, PacketMetadata as UdpPacketMetadata};
        let rx_buffer = UdpPacketBuffer::new(alloc::vec![UdpPacketMetadata::EMPTY; 8], alloc::vec![0; 4096]);
        let tx_buffer = UdpPacketBuffer::new(alloc::vec![UdpPacketMetadata::EMPTY; 8], alloc::vec![0; 4096]);
        let mut socket = UdpSocket::new(rx_buffer, tx_buffer);
        static mut NEXT_UDP_PORT: u16 = 40000;
        let port = unsafe {
            let p = NEXT_UDP_PORT;
            NEXT_UDP_PORT = if NEXT_UDP_PORT >= 50000 { 40000 } else { NEXT_UDP_PORT + 1 };
            p
        };
        let _ = socket.bind(port);
        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
        let handle = sockets.add(socket);
        unsafe {
            for i in 3..64 {
                if FD_TABLE[i].is_none() {
                    let node = alloc::sync::Arc::new(VfsNode {
                        name: alloc::string::String::from("udp_socket"),
                        kind: NodeKind::UdpSocket(handle, alloc::sync::Arc::new(spin::Mutex::new(None))),
                        size: 0,
                        children: spin::RwLock::new(alloc::vec::Vec::new()),
                        file_ops: None,
                        data: spin::RwLock::new(alloc::vec::Vec::new()),
                        static_data: None,
                        uid: spin::RwLock::new(0),
                        gid: spin::RwLock::new(0),
                        mode: spin::RwLock::new(0o666),
                    });
                    FD_TABLE[i] = Some(FileDescriptor::new(node, 0));
                    return i as u64;
                }
            }
        }
    }
    !0
}

static mut BOUND_PORTS: [u16; 64] = [0; 64];

fn sys_bind(fd: u64, sockaddr: u64, _addrlen: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    if sockaddr != 0 {
        unsafe {
            let port_be = *((sockaddr + 2) as *const u16);
            let port = u16::from_be(port_be);
            BOUND_PORTS[fd as usize] = port;
        }
    }
    0
}

fn sys_listen(fd: u64, _backlog: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        let port = if BOUND_PORTS[fd as usize] != 0 { BOUND_PORTS[fd as usize] } else { 80 };
        if let Some(ref desc) = FD_TABLE[fd as usize] {
            if let NodeKind::Socket(target) = desc.node.kind {
                match target {
                    crate::fs::vfs::SocketTarget::Dual { lo, eth } => {
                        let mut lo_socks = crate::net::socket::LOOPBACK_SOCKETS.lock();
                        let r1 = lo_socks.get_mut::<TcpSocket>(lo).listen(port);
                        let mut net_socks = crate::net::socket::NET_SOCKETS.lock();
                        let r2 = net_socks.get_mut::<TcpSocket>(eth).listen(port);
                        crate::serial_println!("[sys_listen] dual listen fd {} port {} (lo: {:?}, eth: {:?})", fd, port, r1, r2);
                        if r1.is_ok() || r2.is_ok() {
                            return 0;
                        }
                    }
                    crate::fs::vfs::SocketTarget::Loopback(handle) => {
                        let mut lo_socks = crate::net::socket::LOOPBACK_SOCKETS.lock();
                        let res = lo_socks.get_mut::<TcpSocket>(handle).listen(port);
                        if res.is_ok() { return 0; }
                    }
                    crate::fs::vfs::SocketTarget::Ethernet(handle) => {
                        let mut net_socks = crate::net::socket::NET_SOCKETS.lock();
                        let res = net_socks.get_mut::<TcpSocket>(handle).listen(port);
                        if res.is_ok() { return 0; }
                    }
                }
            }
        }
    }
    !0
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: [u8; 4],
    pub sin_zero: [u8; 8],
}

fn sys_accept(fd: u64, sockaddr: u64, flags: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    let non_blocking = flags != 0;
    let start = crate::net::socket::now();
    let port = unsafe { if BOUND_PORTS[fd as usize] != 0 { BOUND_PORTS[fd as usize] } else { 80 } };
    loop {
        crate::net::socket::poll();
        unsafe {
            if let Some(ref mut desc) = FD_TABLE[fd as usize] {
                if let NodeKind::Socket(target) = desc.node.kind {
                    // Check Loopback first
                    let lo_h = match target {
                        crate::fs::vfs::SocketTarget::Dual { lo, .. } => Some(lo),
                        crate::fs::vfs::SocketTarget::Loopback(lo) => Some(lo),
                        _ => None,
                    };
                    if let Some(lo) = lo_h {
                        let mut sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
                        let socket = sockets.get_mut::<TcpSocket>(lo);
                        if socket.state() == State::SynReceived {
                            for _ in 0..50 {
                                drop(sockets);
                                crate::net::socket::poll();
                                for _ in 0..1_000 { core::hint::spin_loop(); }
                                sockets = crate::net::socket::LOOPBACK_SOCKETS.lock();
                                if sockets.get_mut::<TcpSocket>(lo).state() == State::Established {
                                    break;
                                }
                            }
                        }
                        let socket = sockets.get_mut::<TcpSocket>(lo);
                        if socket.state() == State::Established || socket.state() == State::CloseWait {
                            let remote_endpoint = socket.remote_endpoint();
                            for i in 3..64 {
                                if FD_TABLE[i].is_none() {
                                    let client_node = alloc::sync::Arc::new(VfsNode {
                                        name: alloc::string::String::from("socket_client"),
                                        kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Loopback(lo)),
                                        size: 0,
                                        children: spin::RwLock::new(alloc::vec::Vec::new()),
                                        file_ops: None,
                                        data: spin::RwLock::new(alloc::vec::Vec::new()),
                                        static_data: None,
                                        uid: spin::RwLock::new(0),
                                        gid: spin::RwLock::new(0),
                                        mode: spin::RwLock::new(0o666),
                                    });
                                    FD_TABLE[i] = Some(FileDescriptor::new(client_node, 0));
                                    BOUND_PORTS[i] = 0;

                                    // Arm a fresh listening socket for loopback
                                    let rx_buffer = SocketBuffer::new(alloc::vec![0; 4096]);
                                    let tx_buffer = SocketBuffer::new(alloc::vec![0; 4096]);
                                    let mut new_sock = TcpSocket::new(rx_buffer, tx_buffer);
                                    new_sock.set_nagle_enabled(false);
                                    let _ = new_sock.listen(port);
                                    let new_handle = sockets.add(new_sock);

                                    match target {
                                        crate::fs::vfs::SocketTarget::Dual { eth, .. } => {
                                            desc.node = alloc::sync::Arc::new(VfsNode {
                                                 name: alloc::string::String::from("socket_listen"),
                                                 kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Dual { lo: new_handle, eth }),
                                                 size: 0,
                                                 children: spin::RwLock::new(alloc::vec::Vec::new()),
                                                 file_ops: None,
                                                 data: spin::RwLock::new(alloc::vec::Vec::new()),
                                                 static_data: None,
                                                 uid: spin::RwLock::new(0),
                                                 gid: spin::RwLock::new(0),
                                                 mode: spin::RwLock::new(0o666),
                                            });
                                        }
                                        crate::fs::vfs::SocketTarget::Loopback(_) => {
                                            desc.node = alloc::sync::Arc::new(VfsNode {
                                                 name: alloc::string::String::from("socket_listen"),
                                                 kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Loopback(new_handle)),
                                                 size: 0,
                                                 children: spin::RwLock::new(alloc::vec::Vec::new()),
                                                 file_ops: None,
                                                 data: spin::RwLock::new(alloc::vec::Vec::new()),
                                                 static_data: None,
                                                 uid: spin::RwLock::new(0),
                                                 gid: spin::RwLock::new(0),
                                                 mode: spin::RwLock::new(0o666),
                                            });
                                        }
                                        _ => {}
                                    }

                                    if sockaddr != 0 {
                                        if let Some(endpoint) = remote_endpoint {
                                            let smoltcp::wire::IpAddress::Ipv4(ipv4) = endpoint.addr;
                                            let p = sockaddr as *mut SockAddrIn;
                                            *p = SockAddrIn {
                                                sin_family: 2,
                                                sin_port: endpoint.port.to_be(),
                                                sin_addr: ipv4.0,
                                                sin_zero: [0; 8],
                                            };
                                        }
                                    }

                                    crate::serial_println!("[sys_accept] accepted lo client on fd {}", i);
                                    return i as u64;
                                }
                            }
                        } else if socket.state() != State::Listen && socket.state() != State::SynReceived {
                            socket.abort();
                            let _ = socket.listen(port);
                        }
                    }

                    // Check Ethernet next
                    let eth_h = match target {
                        crate::fs::vfs::SocketTarget::Dual { eth, .. } => Some(eth),
                        crate::fs::vfs::SocketTarget::Ethernet(eth) => Some(eth),
                        _ => None,
                    };
                    if let Some(eth) = eth_h {
                        let mut sockets = crate::net::socket::NET_SOCKETS.lock();
                        let socket = sockets.get_mut::<TcpSocket>(eth);
                        if socket.state() == State::SynReceived {
                            for _ in 0..50 {
                                drop(sockets);
                                crate::net::socket::poll();
                                for _ in 0..5_000 { core::hint::spin_loop(); }
                                sockets = crate::net::socket::NET_SOCKETS.lock();
                                if sockets.get_mut::<TcpSocket>(eth).state() == State::Established || sockets.get_mut::<TcpSocket>(eth).state() == State::CloseWait {
                                    break;
                                }
                            }
                        }
                        let socket = sockets.get_mut::<TcpSocket>(eth);
                        if socket.state() == State::Established || socket.state() == State::CloseWait {
                            let remote_endpoint = socket.remote_endpoint();
                            for i in 3..64 {
                                if FD_TABLE[i].is_none() {
                                    let client_node = alloc::sync::Arc::new(VfsNode {
                                        name: alloc::string::String::from("socket_client"),
                                        kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Ethernet(eth)),
                                        size: 0,
                                        children: spin::RwLock::new(alloc::vec::Vec::new()),
                                        file_ops: None,
                                        data: spin::RwLock::new(alloc::vec::Vec::new()),
                                        static_data: None,
                                        uid: spin::RwLock::new(0),
                                        gid: spin::RwLock::new(0),
                                        mode: spin::RwLock::new(0o666),
                                    });
                                    FD_TABLE[i] = Some(FileDescriptor::new(client_node, 0));
                                    BOUND_PORTS[i] = 0;

                                    // Arm a fresh listening socket for ethernet
                                    let rx_buffer = SocketBuffer::new(alloc::vec![0; 4096]);
                                    let tx_buffer = SocketBuffer::new(alloc::vec![0; 4096]);
                                    let mut new_sock = TcpSocket::new(rx_buffer, tx_buffer);
                                    new_sock.set_nagle_enabled(false);
                                    let _ = new_sock.listen(port);
                                    let new_handle = sockets.add(new_sock);

                                    match target {
                                        crate::fs::vfs::SocketTarget::Dual { lo, .. } => {
                                            desc.node = alloc::sync::Arc::new(VfsNode {
                                                name: alloc::string::String::from("socket_listen"),
                                                kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Dual { lo, eth: new_handle }),
                                                size: 0,
                                                children: spin::RwLock::new(alloc::vec::Vec::new()),
                                                file_ops: None,
                                                data: spin::RwLock::new(alloc::vec::Vec::new()),
                                                static_data: None,
                                                uid: spin::RwLock::new(0),
                                                gid: spin::RwLock::new(0),
                                                mode: spin::RwLock::new(0o666),
                                            });
                                        }
                                        crate::fs::vfs::SocketTarget::Ethernet(_) => {
                                            desc.node = alloc::sync::Arc::new(VfsNode {
                                                name: alloc::string::String::from("socket_listen"),
                                                kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Ethernet(new_handle)),
                                                size: 0,
                                                children: spin::RwLock::new(alloc::vec::Vec::new()),
                                                file_ops: None,
                                                data: spin::RwLock::new(alloc::vec::Vec::new()),
                                                static_data: None,
                                                uid: spin::RwLock::new(0),
                                                gid: spin::RwLock::new(0),
                                                mode: spin::RwLock::new(0o666),
                                            });
                                        }
                                        _ => {}
                                    }

                                    if sockaddr != 0 {
                                        if let Some(endpoint) = remote_endpoint {
                                            let smoltcp::wire::IpAddress::Ipv4(ipv4) = endpoint.addr;
                                            let p = sockaddr as *mut SockAddrIn;
                                            *p = SockAddrIn {
                                                sin_family: 2,
                                                sin_port: endpoint.port.to_be(),
                                                sin_addr: ipv4.0,
                                                sin_zero: [0; 8],
                                            };
                                        }
                                    }

                                    crate::serial_println!("[sys_accept] accepted eth client on fd {}", i);
                                    return i as u64;
                                }
                            }
                        } else if socket.state() != State::Listen && socket.state() != State::SynReceived {
                            socket.abort();
                            let _ = socket.listen(port);
                        }
                    }
                }
            }
        }
        if non_blocking || (crate::net::socket::now() - start).total_millis() > 50 {
            return !0;
        }
        for _ in 0..10_000 { core::hint::spin_loop(); }
    }
}

fn sys_connect(fd: u64, sockaddr: u64, addrlen: u64) -> u64 {
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref mut desc) = FD_TABLE[fd as usize] {
            if let NodeKind::Socket(target) = desc.node.kind {
                if addrlen >= 8 && sockaddr != 0 {
                    let port_be = core::ptr::read_unaligned((sockaddr + 2) as *const u16);
                    let port = u16::from_be(port_be);
                    let ip = core::ptr::read_unaligned((sockaddr + 4) as *const [u8; 4]);
                    
                    let is_loopback = ip[0] == 127;
                    if is_loopback {
                        let lo_handle = match target {
                            crate::fs::vfs::SocketTarget::Dual { lo, eth } => {
                                crate::net::socket::NET_SOCKETS.lock().remove(eth);
                                lo
                            }
                            crate::fs::vfs::SocketTarget::Loopback(h) => h,
                            crate::fs::vfs::SocketTarget::Ethernet(_) => return !0,
                        };
                        desc.node = alloc::sync::Arc::new(VfsNode {
                            name: alloc::string::String::from("socket_lo"),
                            kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Loopback(lo_handle)),
                            size: 0,
                            children: spin::RwLock::new(alloc::vec::Vec::new()),
                            file_ops: None,
                            data: spin::RwLock::new(alloc::vec::Vec::new()),
                            static_data: None,
                            uid: spin::RwLock::new(0),
                            gid: spin::RwLock::new(0),
                            mode: spin::RwLock::new(0o666),
                        });
                        if crate::net::socket::connect_lo(lo_handle, ip, port).is_ok() {
                            return 0;
                        }
                    } else {
                        let eth_handle = match target {
                            crate::fs::vfs::SocketTarget::Dual { lo, eth } => {
                                crate::net::socket::LOOPBACK_SOCKETS.lock().remove(lo);
                                eth
                            }
                            crate::fs::vfs::SocketTarget::Ethernet(h) => h,
                            crate::fs::vfs::SocketTarget::Loopback(_) => return !0,
                        };
                        desc.node = alloc::sync::Arc::new(VfsNode {
                            name: alloc::string::String::from("socket_eth"),
                            kind: NodeKind::Socket(crate::fs::vfs::SocketTarget::Ethernet(eth_handle)),
                            size: 0,
                            children: spin::RwLock::new(alloc::vec::Vec::new()),
                            file_ops: None,
                            data: spin::RwLock::new(alloc::vec::Vec::new()),
                            static_data: None,
                            uid: spin::RwLock::new(0),
                            gid: spin::RwLock::new(0),
                            mode: spin::RwLock::new(0o666),
                        });
                        if crate::net::socket::connect_eth(eth_handle, ip, port).is_ok() {
                            return 0;
                        }
                    }
                }
            }
            if let NodeKind::UdpSocket(_handle, ref peer) = desc.node.kind {
                if addrlen >= 8 && sockaddr != 0 {
                    let port_be = core::ptr::read_unaligned((sockaddr + 2) as *const u16);
                    let port = u16::from_be(port_be);
                    let ip = core::ptr::read_unaligned((sockaddr + 4) as *const [u8; 4]);
                    *peer.lock() = Some((smoltcp::wire::IpAddress::Ipv4(smoltcp::wire::Ipv4Address::new(ip[0], ip[1], ip[2], ip[3])), port));
                    return 0;
                }
            }
        }
    }
    !0
}

fn sys_getdents64(fd: u64, dirp: u64, count: u64) -> u64 {
    if fd < 3 || fd >= 64 || dirp == 0 || count == 0 { return !0; }
    unsafe {
        if let Some(ref mut desc) = FD_TABLE[fd as usize] {
            if !matches!(desc.node.kind, NodeKind::Directory) {
                return !0; // ENOTDIR
            }
            let children = desc.node.children.read();
            let total_entries = children.len() + 2; // +2 for . and ..
            if desc.offset >= total_entries {
                return 0; // EOF
            }
            let mut out_ptr = dirp as *mut u8;
            let mut bytes_written: u64 = 0;

            for i in desc.offset..total_entries {
                let (name_bytes, d_type, ino): (&[u8], u8, u64) = if i == 0 {
                    (b".", 4u8, (alloc::sync::Arc::as_ptr(&desc.node) as u64) | 1)
                } else if i == 1 {
                    (b"..", 4u8, ((alloc::sync::Arc::as_ptr(&desc.node) as u64) | 1).wrapping_sub(1))
                } else {
                    let child = &children[i - 2];
                    let dt = match child.kind {
                        NodeKind::Directory => 4u8,
                        NodeKind::File => 8u8,
                        NodeKind::CharDevice => 2u8,
                        NodeKind::BlockDevice(_) => 6u8,
                        NodeKind::Socket(_) | NodeKind::UdpSocket(_, _) => 12u8,
                        NodeKind::Pipe(_) => 1u8,
                        NodeKind::SymLink(_) => 10u8, // DT_LNK
                        NodeKind::EventFd(_) | NodeKind::Epoll(_) => 8u8, // DT_REG
                    };
                    (child.name.as_bytes(), dt, (alloc::sync::Arc::as_ptr(child) as u64) | 1)
                };

                let raw_len = 19 + name_bytes.len() + 1;
                let reclen = ((raw_len + 7) / 8) * 8; // 8-byte aligned

                if bytes_written + (reclen as u64) > count {
                    break;
                }

                // d_ino
                core::ptr::write_unaligned(out_ptr as *mut u64, ino);
                // d_off
                core::ptr::write_unaligned(out_ptr.add(8) as *mut i64, (i + 1) as i64);
                // d_reclen
                core::ptr::write_unaligned(out_ptr.add(16) as *mut u16, reclen as u16);
                // d_type
                *out_ptr.add(18) = d_type;
                // d_name
                core::ptr::copy_nonoverlapping(name_bytes.as_ptr(), out_ptr.add(19), name_bytes.len());
                *out_ptr.add(19 + name_bytes.len()) = 0; // null terminator

                out_ptr = out_ptr.add(reclen);
                bytes_written += reclen as u64;
                desc.offset = i + 1;
            }
            return bytes_written;
        }
    }
    !0
}

fn sys_mkdirat(dirfd: u64, pathname: u64, _mode: u64) -> u64 {
    if pathname == 0 { return !0; }
    unsafe {
        let mut len = 0;
        let mut ptr = pathname as *const u8;
        while *ptr != 0 && len < 256 { len += 1; ptr = ptr.add(1); }
        let slice = core::slice::from_raw_parts(pathname as *const u8, len);
        if let Ok(raw_path) = core::str::from_utf8(slice) {
            let (euid, egid) = {
                let pm = crate::sys::process::PROCESS_MANAGER.lock();
                let cur_pid = pm.current_pid;
                if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == cur_pid) {
                    (p.euid, p.egid)
                } else {
                    (0, 0)
                }
            };
            let resolved = resolve_at_path(dirfd as i64, raw_path);
            let parts: alloc::vec::Vec<&str> = resolved.split('/').filter(|s| !s.is_empty()).collect();
            let dname = parts.last().unwrap_or(&"dir");
            let dir_mode = if _mode != 0 { (_mode & 0o777) as u32 } else { 0o755 };
            let new_dir = alloc::sync::Arc::new(VfsNode {
                name: alloc::string::ToString::to_string(*dname),
                kind: NodeKind::Directory,
                size: 0,
                children: spin::RwLock::new(alloc::vec::Vec::new()),
                file_ops: None,
                data: spin::RwLock::new(alloc::vec::Vec::new()),
                static_data: None,
                uid: spin::RwLock::new(euid),
                gid: spin::RwLock::new(egid),
                mode: spin::RwLock::new(dir_mode),
            });
            crate::fs::vfs::add_node(&resolved, new_dir);
            return 0;
        }
    }
    !0
}

fn sys_unlinkat(dirfd: u64, pathname: u64, _flags: u64) -> u64 {
    if pathname == 0 { return !0; }
    unsafe {
        let mut len = 0;
        let mut ptr = pathname as *const u8;
        while *ptr != 0 && len < 256 { len += 1; ptr = ptr.add(1); }
        let slice = core::slice::from_raw_parts(pathname as *const u8, len);
        if let Ok(raw_path) = core::str::from_utf8(slice) {
            let resolved = resolve_at_path(dirfd as i64, raw_path);
            let parts: alloc::vec::Vec<&str> = resolved.split('/').filter(|s| !s.is_empty()).collect();
            if parts.is_empty() { return !0; }
            let target_name = parts.last().unwrap();
            let parent_path = if parts.len() == 1 { "/" } else {
                let slash_idx = resolved.rfind('/').unwrap_or(0);
                if slash_idx == 0 { "/" } else { &resolved[..slash_idx] }
            };
            if let Some(parent_node) = crate::fs::vfs::lookup(parent_path) {
                let mut children = parent_node.children.write();
                if let Some(pos) = children.iter().position(|c| c.name == *target_name) {
                    children.remove(pos);
                    return 0;
                }
            }
        }
    }
    0
}


pub fn sys_sync() -> u64 {
    if let Some(blk_arc) = crate::hal::device::DEVICE_MANAGER.lock().block_devices.first() {
        crate::fs::pfs::save_to_disk(&*blk_arc.lock());
    }
    0
}

static UMASK_VAL: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0o022);

fn sys_umask(new_mask: u64) -> u64 {
    let old = UMASK_VAL.swap(new_mask & 0o777, core::sync::atomic::Ordering::Relaxed);
    old
}

fn read_cstr(ptr_val: u64) -> Option<alloc::string::String> {
    if ptr_val < 0x1000 || ptr_val >= 0x0000_8000_0000_0000 {
        return None;
    }
    unsafe {
        let mut len = 0;
        let mut ptr = ptr_val as *const u8;
        while *ptr != 0 && len < 1024 {
            len += 1;
            ptr = ptr.add(1);
        }
        let slice = core::slice::from_raw_parts(ptr_val as *const u8, len);
        core::str::from_utf8(slice).ok().map(|s| alloc::string::ToString::to_string(s))
    }
}

fn sys_mount(special_ptr: u64, dir_ptr: u64, fstype_ptr: u64, _flags: u64, _data: u64) -> u64 {
    let special = match read_cstr(special_ptr) {
        Some(s) => s,
        None => return !0,
    };
    let dir = match read_cstr(dir_ptr) {
        Some(s) => s,
        None => return !0,
    };
    let fstype = match read_cstr(fstype_ptr) {
        Some(s) => s,
        None => alloc::string::String::from("ext4"),
    };

    crate::serial_println!("[sys_mount] special='{}', dir='{}', fstype='{}'", special, dir, fstype);

    if fstype != "ext4" && fstype != "ext2" && fstype != "ext3" {
        crate::serial_println!("[sys_mount] Unsupported fstype '{}'", fstype);
        return !0;
    }

    let block_device: Option<Arc<spin::Mutex<dyn crate::hal::device::BlockDevice>>> = {
        let clean_sp = special.trim_start_matches('/');
        if clean_sp == "dev/vda" || clean_sp == "dev/vda1" || clean_sp == "vda" || clean_sp == "vda1" {
            crate::hal::device::DEVICE_MANAGER.lock().block_devices.first().cloned()
        } else if clean_sp.starts_with("dev/loop") {
            if let Ok(id) = clean_sp["dev/loop".len()..].parse::<usize>() {
                if id < crate::fs::loop_dev::LOOP_DEVICES.len() {
                    Some(crate::fs::loop_dev::LOOP_DEVICES[id].clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else if clean_sp.starts_with("loop") {
            if let Ok(id) = clean_sp["loop".len()..].parse::<usize>() {
                if id < crate::fs::loop_dev::LOOP_DEVICES.len() {
                    Some(crate::fs::loop_dev::LOOP_DEVICES[id].clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Some(node) = crate::fs::vfs::lookup(&special) {
            if let NodeKind::BlockDevice(ref blk) = node.kind {
                Some(blk.clone())
            } else {
                None
            }
        } else {
            None
        }
    };

    let dev = match block_device {
        Some(d) => d,
        None => {
            crate::serial_println!("[sys_mount] Block device '{}' not found", special);
            return !0;
        }
    };

    match crate::fs::ext4::Ext4Filesystem::new(dev) {
        Ok(ext4) => {
            let mut mounts = crate::fs::vfs::MOUNT_POINTS.write();
            mounts.retain(|m| m.target != dir);
            mounts.push(crate::fs::vfs::MountPoint {
                target: dir.clone(),
                source: special.clone(),
                fstype: fstype.clone(),
                fs: Arc::new(spin::Mutex::new(ext4)),
            });
            crate::serial_println!("[sys_mount] Successfully mounted {} on {}", special, dir);
            0
        }
        Err(e) => {
            crate::serial_println!("[sys_mount] Ext4 mount failed: {}", e);
            !0
        }
    }
}

fn sys_umount2(target_ptr: u64, _flags: i32) -> u64 {
    let target = match read_cstr(target_ptr) {
        Some(s) => s,
        None => return !0,
    };
    let mut mounts = crate::fs::vfs::MOUNT_POINTS.write();
    let before = mounts.len();
    for m in mounts.iter() {
        if m.target == target {
            let mut fs = m.fs.lock();
            let _ = fs.sync_superblock();
        }
    }
    mounts.retain(|m| m.target != target);
    if mounts.len() < before {
        crate::serial_println!("[sys_umount2] Unmounted {}", target);
        0
    } else {
        crate::serial_println!("[sys_umount2] Target '{}' not mounted", target);
        !0
    }
}

fn sys_eventfd2(initval: u64, flags: u64) -> u64 {
    let node = Arc::new(VfsNode::new(
        alloc::string::String::from("eventfd"),
        NodeKind::EventFd(Arc::new(spin::Mutex::new(crate::fs::vfs::EventFdState {
            counter: initval,
            flags: flags as u32,
        }))),
        0, 0, 0o600
    ));
    unsafe {
        if let Some(fd) = alloc_fd(FileDescriptor::new(node, 0)) {
            fd as u64
        } else {
            (-24i64) as u64 // -EMFILE
        }
    }
}

fn sys_epoll_create1(flags: u64) -> u64 {
    let node = Arc::new(VfsNode::new(
        alloc::string::String::from("epoll"),
        NodeKind::Epoll(Arc::new(spin::Mutex::new(crate::fs::vfs::EpollState {
            registrations: alloc::vec::Vec::new(),
            flags: flags as u32,
        }))),
        0, 0, 0o600
    ));
    unsafe {
        if let Some(fd) = alloc_fd(FileDescriptor::new(node, 0)) {
            fd as u64
        } else {
            (-24i64) as u64 // -EMFILE
        }
    }
}

fn sys_epoll_ctl(epfd: u64, op: u64, fd: u64, event_ptr: u64) -> u64 {
    if epfd >= 64 || fd >= 64 { return (-9i64) as u64; /* -EBADF */ }
    unsafe {
        if let Some(ref desc) = FD_TABLE[epfd as usize] {
            if let NodeKind::Epoll(ref lock) = desc.node.kind {
                let mut state = lock.lock();
                match op {
                    1 => { // EPOLL_CTL_ADD
                        if event_ptr == 0 { return (-14i64) as u64; /* -EFAULT */ }
                        let events = *(event_ptr as *const u32);
                        let data = *((event_ptr + 8) as *const u64);
                        if state.registrations.iter().any(|r| r.fd == fd as i32) {
                            return (-17i64) as u64; // -EEXIST
                        }
                        state.registrations.push(crate::fs::vfs::EpollRegistration {
                            fd: fd as i32,
                            events,
                            data,
                        });
                        0
                    }
                    2 => { // EPOLL_CTL_DEL
                        if let Some(pos) = state.registrations.iter().position(|r| r.fd == fd as i32) {
                            state.registrations.remove(pos);
                            0
                        } else {
                            (-2i64) as u64 // -ENOENT
                        }
                    }
                    3 => { // EPOLL_CTL_MOD
                        if event_ptr == 0 { return (-14i64) as u64; /* -EFAULT */ }
                        let events = *(event_ptr as *const u32);
                        let data = *((event_ptr + 8) as *const u64);
                        if let Some(reg) = state.registrations.iter_mut().find(|r| r.fd == fd as i32) {
                            reg.events = events;
                            reg.data = data;
                            0
                        } else {
                            (-2i64) as u64 // -ENOENT
                        }
                    }
                    _ => (-22i64) as u64, // -EINVAL
                }
            } else {
                (-22i64) as u64 // -EINVAL: not an epoll fd
            }
        } else {
            (-9i64) as u64 // -EBADF
        }
    }
}

fn sys_epoll_pwait(epfd: u64, events_ptr: u64, maxevents: u64, timeout: i64, _sigmask_ptr: u64) -> u64 {
    if epfd >= 64 || maxevents == 0 || events_ptr == 0 { return (-22i64) as u64; }
    let epoll_lock = unsafe {
        if let Some(ref desc) = FD_TABLE[epfd as usize] {
            if let NodeKind::Epoll(ref lock) = desc.node.kind {
                lock.clone()
            } else {
                return (-22i64) as u64;
            }
        } else {
            return (-9i64) as u64;
        }
    };

    let start_ms = get_current_time_ms();
    loop {
        crate::net::socket::poll();
        let ready = check_epoll_events(&epoll_lock, events_ptr, maxevents as usize);
        if ready > 0 || timeout == 0 {
            return ready as u64;
        }
        if timeout > 0 {
            let elapsed = get_current_time_ms().saturating_sub(start_ms);
            if elapsed >= timeout as u64 {
                return 0;
            }
        }
        let req = [0u64, 1_000_000u64]; // 1ms
        sys_nanosleep(req.as_ptr() as u64, 0);
        crate::sys::process::schedule();
    }
}

fn sys_epoll_pwait2(epfd: u64, events_ptr: u64, maxevents: u64, timeout_ts: u64, sigmask_ptr: u64) -> u64 {
    let timeout_ms: i64 = if timeout_ts != 0 {
        unsafe {
            let sec = *(timeout_ts as *const u64);
            let nsec = *((timeout_ts + 8) as *const u64);
            (sec * 1000 + nsec / 1_000_000) as i64
        }
    } else {
        -1
    };
    sys_epoll_pwait(epfd, events_ptr, maxevents, timeout_ms, sigmask_ptr)
}

fn check_epoll_events(epoll_lock: &Arc<spin::Mutex<crate::fs::vfs::EpollState>>, events_ptr: u64, maxevents: usize) -> usize {
    let state = epoll_lock.lock();
    let mut ready_count = 0;
    for reg in state.registrations.iter() {
        let mut revents: u32 = 0;
        let fd = reg.fd;
        if fd == 0 {
            if crate::hal::serial::has_byte() {
                revents |= 0x0001; // EPOLLIN
            }
        } else if fd == 1 || fd == 2 {
            revents |= 0x0004; // EPOLLOUT
        } else if fd > 0 && (fd as usize) < 64 {
            unsafe {
                if let Some(ref desc) = FD_TABLE[fd as usize] {
                    match desc.node.kind {
                        NodeKind::EventFd(ref lock) => {
                            let ef = lock.lock();
                            if ef.counter > 0 { revents |= 0x0001; }
                            revents |= 0x0004;
                        }
                        NodeKind::Pipe(ref ring_lock) => {
                            let ring = ring_lock.lock();
                            if ring.count > 0 || ring.writers == 0 { revents |= 0x0001; }
                            if crate::fs::vfs::PIPE_BUFFER_SIZE - ring.count > 0 { revents |= 0x0004; }
                            if ring.writers == 0 { revents |= 0x0010; }
                        }
                        NodeKind::Socket(..) | NodeKind::UdpSocket(..) => {
                            revents |= 0x0005;
                        }
                        _ => {
                            revents |= 0x0005;
                        }
                    }
                } else {
                    revents |= 0x0020; // EPOLLERR
                }
            }
        }
        let matched = revents & reg.events;
        if matched != 0 {
            let out_ptr = unsafe { (events_ptr as *mut u8).add(ready_count * 16) };
            unsafe {
                core::ptr::write(out_ptr as *mut u32, matched);
                core::ptr::write((out_ptr.add(4)) as *mut u32, 0);
                core::ptr::write((out_ptr.add(8)) as *mut u64, reg.data);
            }
            ready_count += 1;
            if ready_count >= maxevents {
                break;
            }
        }
    }
    ready_count
}

fn get_current_time_ms() -> u64 {
    let mut count: u64 = 0;
    let mut freq: u64 = 0;
    unsafe {
        core::arch::asm!("mrs {0}, cntvct_el0", out(reg) count);
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) freq);
    }
    if freq > 0 {
        (count * 1000) / freq
    } else {
        0
    }
}

fn sys_symlinkat(target_ptr: u64, newdirfd: i64, linkpath_ptr: u64) -> u64 {
    let target_str = match read_cstr(target_ptr) {
        Some(s) => s,
        None => return (-14i64) as u64,
    };
    let linkpath_str = match read_cstr(linkpath_ptr) {
        Some(s) => s,
        None => return (-14i64) as u64,
    };
    let full_linkpath = resolve_at_path(newdirfd, &linkpath_str);
    let parts: alloc::vec::Vec<&str> = full_linkpath.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() { return (-22i64) as u64; }
    let leaf_name = parts.last().unwrap();
    let parent_path = if parts.len() > 1 {
        let mut p = alloc::string::String::new();
        for &comp in &parts[..parts.len() - 1] {
            p.push('/');
            p.push_str(comp);
        }
        p
    } else {
        alloc::string::String::from("/")
    };

    if let Some(parent_node) = crate::fs::vfs::lookup(&parent_path) {
        let sym_node = Arc::new(VfsNode::new(
            alloc::string::ToString::to_string(leaf_name),
            NodeKind::SymLink(target_str),
            0, 0, 0o777,
        ));
        parent_node.children.write().push(sym_node);
        0
    } else {
        (-2i64) as u64 // -ENOENT
    }
}

fn sys_mremap(old_addr: u64, old_size: u64, new_size: u64, _flags: u64, _new_addr: u64) -> u64 {
    if old_size == 0 || new_size == 0 { return (-22i64) as u64; }
    if new_size <= old_size {
        return old_addr;
    }
    let new_mapped = sys_mmap(0, new_size, 3, 0x22, !0, 0);
    if new_mapped == !0 {
        return (-12i64) as u64; // -ENOMEM
    }
    if old_addr != 0 {
        unsafe {
            core::ptr::copy_nonoverlapping(old_addr as *const u8, new_mapped as *mut u8, old_size as usize);
        }
    }
    new_mapped
}
