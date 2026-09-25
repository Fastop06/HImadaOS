use core::cmp::Ord;
use xmas_elf::{ElfFile, program::{ProgramHeader, Type}};
use crate::serial_println;
use crate::mm::pmm::{alloc_frame, PAGE_SIZE};
use core::ptr::copy_nonoverlapping;
use crate::sys::process::{Process, ProcessState, PROCESS_MANAGER};

pub fn load_elf_image(
    root_paddr: usize,
    raw_file_data: &[u8],
    prog_name: &str,
    args: &[alloc::string::String],
    envs: &[alloc::string::String],
) -> Result<(usize, usize), &'static str> {
    let aligned_vec: alloc::vec::Vec<u8>;
    let file_data: &[u8] = if (raw_file_data.as_ptr() as usize) % 8 == 0 {
        raw_file_data
    } else {
        aligned_vec = alloc::vec::Vec::from(raw_file_data);
        // Leak the vector slice for the duration of load_elf_image using a local reference
        let ptr = aligned_vec.as_ptr();
        let len = aligned_vec.len();
        core::mem::forget(aligned_vec);
        unsafe { core::slice::from_raw_parts(ptr, len) }
    };
    let elf = ElfFile::new(file_data).map_err(|_| "Failed to parse ELF")?;

    serial_println!("[ELF] Loading ELF binary: {}", prog_name);

    let mut interp_path: Option<alloc::string::String> = None;
    for ph in elf.program_iter() {
        if ph.get_type() == Ok(Type::Interp) {
            let offset = ph.offset() as usize;
            let file_size = ph.file_size() as usize;
            if offset + file_size <= file_data.len() {
                let interp_bytes = &file_data[offset..offset + file_size];
                let len = interp_bytes.iter().position(|&b| b == 0).unwrap_or(interp_bytes.len());
                if let Ok(s) = core::str::from_utf8(&interp_bytes[..len]) {
                    interp_path = Some(alloc::string::String::from(s));
                }
            }
            break;
        }
    }

    let is_dyn = file_data.len() > 18 && u16::from_le_bytes([file_data[16], file_data[17]]) == 3;
    let load_bias: usize = if is_dyn {
        0x0000_0000_0040_0000
    } else {
        0
    };

    for ph in elf.program_iter() {
        if ph.get_type() == Ok(Type::Load) {
            let vaddr = load_bias + ph.virtual_addr() as usize;
            let mem_size = ph.mem_size() as usize;
            let file_size = ph.file_size() as usize;
            let offset = ph.offset() as usize;

            if mem_size == 0 {
                continue;
            }

            let writable = ph.flags().is_write();
            let executable = ph.flags().is_execute();

            // W^X Security enforcement: if both write and execute, prioritize executable for code segments
            let (w, x) = if writable && executable {
                (false, true)
            } else {
                (writable, executable)
            };

            let start_page = vaddr / PAGE_SIZE;
            let end_page = (vaddr + mem_size + PAGE_SIZE - 1) / PAGE_SIZE;

            for page_idx in start_page..end_page {
                let paddr = alloc_frame().ok_or("OOM alloc segment page")?;
                let page_vaddr = page_idx * PAGE_SIZE;
                let page_virt = unsafe { crate::mm::vmm::phys_to_virt(paddr) as *mut u8 };
                
                unsafe {
                    crate::mm::himada_page_zero(page_virt, PAGE_SIZE);
                }

                // If this page overlaps with segment file data
                if page_vaddr < vaddr + file_size && page_vaddr + PAGE_SIZE > vaddr {
                    let page_offset = if page_vaddr >= vaddr { page_vaddr - vaddr } else { 0 };
                    let dst_offset = if vaddr > page_vaddr { vaddr - page_vaddr } else { 0 };
                    let copy_len = (file_size - page_offset).min(PAGE_SIZE - dst_offset);
                    
                    let src = &file_data[offset + page_offset..offset + page_offset + copy_len];
                    let dst_virt = unsafe { page_virt.add(dst_offset) };
                    unsafe {
                        copy_nonoverlapping(src.as_ptr(), dst_virt, copy_len);
                    }
                }
                
                unsafe {
                    crate::graphics::clean_dcache_range(page_virt as usize, PAGE_SIZE);
                    crate::mm::vmm::map_page_in_root(root_paddr, page_vaddr, paddr, w, x);
                }
            }
        }
    }

    let entry = load_bias + elf.header.pt2.entry_point() as usize;

    let (final_entry, at_base) = if let Some(ref path) = interp_path {
        serial_println!("[ELF] Dynamic executable '{}' requests interpreter: {}", prog_name, path);
        let interp_node_opt = crate::fs::vfs::lookup(path).or_else(|| {
            if path.starts_with("/lib/") {
                let mut p = alloc::string::String::from("/usr");
                p.push_str(path);
                crate::fs::vfs::lookup(&p)
            } else if path.starts_with("/usr/lib/") {
                crate::fs::vfs::lookup(&path[4..])
            } else {
                None
            }
        });
        if let Some(interp_node) = interp_node_opt {
            let raw_interp = if let Some(s) = interp_node.static_data {
                alloc::vec::Vec::from(s)
            } else {
                interp_node.data.read().clone()
            };
            let interp_aligned = if (raw_interp.as_ptr() as usize) % 8 == 0 {
                raw_interp
            } else {
                let mut v = alloc::vec::Vec::with_capacity(raw_interp.len() + 16);
                v.extend_from_slice(&raw_interp);
                v
            };
            match ElfFile::new(&interp_aligned) {
                Ok(interp_elf) => {
                    const INTERP_BASE: usize = 0x7000_0000_0000;
                    for iph in interp_elf.program_iter() {
                        if iph.get_type() == Ok(Type::Load) {
                            let ivaddr = INTERP_BASE + iph.virtual_addr() as usize;
                            let imem_size = iph.mem_size() as usize;
                            let ifile_size = iph.file_size() as usize;
                            let ioffset = iph.offset() as usize;

                            if imem_size == 0 {
                                continue;
                            }

                            let w = iph.flags().is_write();
                            let x = iph.flags().is_execute();

                            let istart_page = ivaddr / PAGE_SIZE;
                            let iend_page = (ivaddr + imem_size + PAGE_SIZE - 1) / PAGE_SIZE;

                            for page_idx in istart_page..iend_page {
                                let paddr = alloc_frame().ok_or("OOM alloc interp segment page")?;
                                let page_vaddr = page_idx * PAGE_SIZE;
                                let page_virt = unsafe { crate::mm::vmm::phys_to_virt(paddr) as *mut u8 };

                                unsafe {
                                    crate::mm::himada_page_zero(page_virt, PAGE_SIZE);
                                }

                                if page_vaddr < ivaddr + ifile_size && page_vaddr + PAGE_SIZE > ivaddr {
                                    let page_offset = if page_vaddr >= ivaddr { page_vaddr - ivaddr } else { 0 };
                                    let dst_offset = if ivaddr > page_vaddr { ivaddr - page_vaddr } else { 0 };
                                    let copy_len = (ifile_size - page_offset).min(PAGE_SIZE - dst_offset);

                                    let src = &interp_aligned[ioffset + page_offset..ioffset + page_offset + copy_len];
                                    let dst_virt = unsafe { page_virt.add(dst_offset) };
                                    unsafe {
                                        copy_nonoverlapping(src.as_ptr(), dst_virt, copy_len);
                                    }
                                }

                                unsafe {
                                    crate::graphics::clean_dcache_range(page_virt as usize, PAGE_SIZE);
                                    crate::mm::vmm::map_page_in_root(root_paddr, page_vaddr, paddr, w, x);
                                }
                            }
                        }
                    }
                    let interp_entry = INTERP_BASE + interp_elf.header.pt2.entry_point() as usize;
                    serial_println!("[ELF] Dynamic interpreter loaded at base 0x{:x}, entry 0x{:x}", INTERP_BASE, interp_entry);
                    (interp_entry, INTERP_BASE)
                }
                Err(_) => {
                    serial_println!("[ELF] Failed to parse interpreter ELF, fallback to direct entry");
                    (entry, 0)
                }
            }
        } else {
            serial_println!("[ELF] Interpreter '{}' not found in VFS, fallback to direct entry", path);
            (entry, 0)
        }
    } else {
        (entry, 0)
    };

    // Userspace stack: 1 MB at top of user address space
    let stack_top = 0x_8000_0000_0000;
    let stack_pages = 256; // 1 MB
    let stack_bottom = stack_top - stack_pages * PAGE_SIZE;
    
    let mut last_paddr = 0;
    for i in 0..stack_pages {
        let paddr = alloc_frame().ok_or("OOM alloc stack")?;
        last_paddr = paddr;
        unsafe {
            let page_virt = crate::mm::vmm::phys_to_virt(paddr) as *mut u8;
            crate::mm::himada_page_zero(page_virt, PAGE_SIZE);
            crate::graphics::clean_dcache_range(page_virt as usize, PAGE_SIZE);
            // Stack is writable, non-executable (W^X)
            crate::mm::vmm::map_page_in_root(root_paddr, stack_bottom + i * PAGE_SIZE, paddr, true, false);
        }
    }

    let last_page_virt = unsafe { crate::mm::vmm::phys_to_virt(last_paddr) as *mut u8 };
    let mut alloc_top = 4096usize;

    // 1. 16 random bytes for AT_RANDOM (used for stack canary)
    alloc_top -= 16;
    let random_offset = alloc_top;
    let random_vaddr = stack_top - (4096 - random_offset);
    let mut seed: u64 = 0x1234_5678_9abc_def0;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) seed);
    }
    for i in 0..16 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        unsafe {
            *last_page_virt.add(random_offset + i) = (seed >> 32) as u8;
        }
    }

    // 2. Platform string "aarch64\0"
    let platform_bytes = b"aarch64\0";
    alloc_top -= platform_bytes.len();
    let platform_offset = alloc_top;
    let platform_vaddr = stack_top - (4096 - platform_offset);
    unsafe {
        copy_nonoverlapping(platform_bytes.as_ptr(), last_page_virt.add(platform_offset), platform_bytes.len());
    }

    // 3. Environment strings
    let mut env_vaddrs = alloc::vec::Vec::new();
    for env_str in envs.iter() {
        let b = env_str.as_bytes();
        alloc_top -= b.len() + 1;
        let off = alloc_top;
        let vaddr = stack_top - (4096 - off);
        unsafe {
            copy_nonoverlapping(b.as_ptr(), last_page_virt.add(off), b.len());
            *last_page_virt.add(off + b.len()) = 0;
        }
        env_vaddrs.push(vaddr);
    }

    // 4. Argument strings
    let mut arg_vaddrs = alloc::vec::Vec::new();
    for arg_str in args.iter() {
        let b = arg_str.as_bytes();
        alloc_top -= b.len() + 1;
        let off = alloc_top;
        let vaddr = stack_top - (4096 - off);
        unsafe {
            copy_nonoverlapping(b.as_ptr(), last_page_virt.add(off), b.len());
            *last_page_virt.add(off + b.len()) = 0;
        }
        arg_vaddrs.push(vaddr);
    }

    // Align alloc_top down to 8 bytes
    alloc_top &= !7;

    // Find phdr_vaddr
    let ph_offset = elf.header.pt2.ph_offset() as usize;
    let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
    let ph_count = elf.header.pt2.ph_count() as usize;

    let mut phdr_vaddr = 0;
    for ph in elf.program_iter() {
        if ph.get_type() == Ok(Type::Load) {
            let vaddr = load_bias + ph.virtual_addr() as usize;
            let offset = ph.offset() as usize;
            let file_size = ph.file_size() as usize;
            if offset <= ph_offset && ph_offset < offset + file_size {
                phdr_vaddr = vaddr + (ph_offset - offset);
                break;
            }
        }
    }
    if phdr_vaddr == 0 {
        phdr_vaddr = load_bias + ph_offset;
    }

    // Auxiliary vector entries: (key, value)
    let auxv = [
        (16usize /* AT_HWCAP */, 0xffusize),
        (6 /* AT_PAGESZ */, 4096),
        (17 /* AT_CLKTCK */, 100),
        (3 /* AT_PHDR */, phdr_vaddr),
        (4 /* AT_PHENT */, ph_entry_size),
        (5 /* AT_PHNUM */, ph_count),
        (7 /* AT_BASE */, at_base),
        (8 /* AT_FLAGS */, 0),
        (9 /* AT_ENTRY */, entry),
        (11 /* AT_UID */, 0),
        (12 /* AT_EUID */, 0),
        (13 /* AT_GID */, 0),
        (14 /* AT_EGID */, 0),
        (23 /* AT_SECURE */, 0),
        (25 /* AT_RANDOM */, random_vaddr),
        (15 /* AT_PLATFORM */, platform_vaddr),
        (0 /* AT_NULL */, 0),
    ];

    // Build the stack words array in memory order:
    // [argc, argv[0], ..., argv[n-1], NULL, envp[0], ..., envp[m-1], NULL, auxv[0].key, auxv[0].val, ...]
    let mut words = alloc::vec::Vec::new();
    words.push(arg_vaddrs.len());
    for &av in &arg_vaddrs {
        words.push(av);
    }
    words.push(0); // argv NULL terminator
    for &ev in &env_vaddrs {
        words.push(ev);
    }
    words.push(0); // envp NULL terminator
    for (k, v) in &auxv {
        words.push(*k);
        words.push(*v);
    }

    let words_bytes = words.len() * core::mem::size_of::<usize>();
    let mut start_offset = alloc_top - words_bytes;
    if start_offset % 16 != 0 {
        alloc_top -= 8;
        start_offset = alloc_top - words_bytes;
    }

    unsafe {
        let dst_ptr = last_page_virt.add(start_offset) as *mut usize;
        for (idx, &w) in words.iter().enumerate() {
            *dst_ptr.add(idx) = w;
        }
        crate::graphics::clean_dcache_range(last_page_virt as usize, PAGE_SIZE);
    }

    let sp = stack_top - (4096 - start_offset);
    Ok((final_entry, sp))
}

pub fn load_and_run(file_data: &[u8]) -> ! {
    let root_paddr: usize;
    unsafe {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) root_paddr);
    }
    
    let default_args = [alloc::string::String::from("/bin/bash")];
    let default_envs = [
        alloc::string::String::from("PATH=/bin:/usr/bin"),
        alloc::string::String::from("USER=root"),
        alloc::string::String::from("HOME=/root"),
        alloc::string::String::from("TERM=xterm-256color"),
    ];
    let (entry, sp) = load_elf_image(root_paddr, file_data, "/bin/bash", &default_args, &default_envs)
        .expect("Failed to load init ELF");

    // Initialize Process 1 (init / himada shell)
    let mut proc = Process::new(1, 0, root_paddr, "init");
    proc.entry_point = entry;
    proc.ustack_top = sp;
    proc.state = ProcessState::Running;
    proc.running_cpu.store(0, core::sync::atomic::Ordering::Release);
    proc.cpu_context.x30 = crate::sys::process::return_from_fork_trampoline as *const () as usize as u64;

    let kstack_top = proc.kstack_top;

    {
        let mut pm = PROCESS_MANAGER.lock();
        pm.procs[0] = Some(proc);
        pm.current_pid = 1;
        pm.current_pids[0] = 1;
    }

    // Release secondary CPU cores from SMP wait gate
    crate::hal::smp::SMP_SCHEDULER_READY.store(true, core::sync::atomic::Ordering::Release);
    unsafe {
        core::arch::asm!("sev");
    }

    crate::log_step("Jumping to Userspace EL0 (HimadaOS Server Process #1)...", Some("OK"));

    unsafe {
        // Jump to ring 3 using the process's dedicated kernel stack
        let kstack_top_ptr = kstack_top;
        core::arch::asm!(
            "msr spsel, #1",
            "mov sp, {0}",
            "msr sp_el0, {1}",
            "msr spsr_el1, {2}",
            "msr elr_el1, {3}",
            "dsb ish",
            "ic ialluis",
            "dsb ish",
            "isb",
            "eret",
            in(reg) kstack_top_ptr,
            in(reg) sp,
            in(reg) 0x3c0u64,
            in(reg) entry,
            options(noreturn)
        );
    }
}
