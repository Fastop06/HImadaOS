with open('src/sys/linux_abi.rs', 'r') as f:
    content = f.read()

dispatch_old = """        198 => sys_socket(arg0, arg1, arg2),
        200 => sys_bind(arg0, arg1, arg2),
        201 => sys_listen(arg0, arg1),
        202 => sys_accept(arg0, arg1, arg2),
        _ => {
            !0 // -ENOSYS
        }"""

dispatch_new = """        29 => sys_ioctl(arg0, arg1, arg2),
        66 => sys_writev(arg0, arg1, arg2),
        94 => sys_exit(arg0), // exit_group
        96 => sys_set_tid_address(arg0),
        113 => sys_clock_gettime(arg0, arg1),
        198 => sys_socket(arg0, arg1, arg2),
        200 => sys_bind(arg0, arg1, arg2),
        201 => sys_listen(arg0, arg1),
        202 => sys_accept(arg0, arg1, arg2),
        203 => sys_connect(arg0, arg1, arg2),
        214 => sys_brk(arg0),
        _ => {
            serial_println!("[Syscall] UNHANDLED sys_no: {}", sys_no);
            !0 // -ENOSYS
        }"""

content = content.replace(dispatch_old, dispatch_new)

helpers = """
fn sys_connect(fd: u64, sockaddr: u64, addrlen: u64) -> u64 {
    serial_println!("[Syscall] sys_connect({}, {:#x}, {})", fd, sockaddr, addrlen);
    if fd < 3 || fd >= 64 { return !0; }
    unsafe {
        if let Some(ref desc) = FD_TABLE[fd as usize] {
            if let NodeKind::Socket(handle) = desc.node.kind {
                if addrlen >= 8 && sockaddr != 0 {
                    let port_be = core::ptr::read_unaligned((sockaddr + 2) as *const u16);
                    let port = u16::from_be(port_be);
                    let ip = core::ptr::read_unaligned((sockaddr + 4) as *const [u8; 4]);
                    serial_println!("[Syscall] Connecting to {}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port);
                    match crate::net::socket::connect(handle, ip, port) {
                        Ok(()) => {
                            serial_println!("[Syscall] Connect successful!");
                            return 0;
                        }
                        Err(e) => {
                            serial_println!("[Syscall] Connect error: {}", e);
                            return !0;
                        }
                    }
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

fn sys_ioctl(_fd: u64, _req: u64, _arg: u64) -> u64 {
    0
}

fn sys_set_tid_address(_tidptr: u64) -> u64 {
    1
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
"""

content += helpers

with open('src/sys/linux_abi.rs', 'w') as f:
    f.write(content)
