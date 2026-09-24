use limine::request::HhdmRequest;

#[used]
#[link_section = ".requests"]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

pub unsafe fn init() {
    let _hhdm_offset = HHDM_REQUEST.response()
        .expect("Limine didn't provide HHDM response")
        .offset;
        
    // Allocate a root page table for TTBR0_EL1 (userspace)
    let root_paddr = super::pmm::alloc_frame().expect("OOM root pt");
    let root_vaddr = phys_to_virt(root_paddr) as *mut u8;
    crate::mm::himada_page_zero(root_vaddr as *mut u8, 4096);
    crate::graphics::clean_dcache_range(root_vaddr as usize, 4096);
    
    // Set TTBR0_EL1
    core::arch::asm!("msr ttbr0_el1, {}", in(reg) root_paddr);
    core::arch::asm!("isb");
    core::arch::asm!("tlbi vmalle1is");
    core::arch::asm!("dsb sy");
    core::arch::asm!("isb");
}

pub unsafe fn phys_to_virt(paddr: usize) -> usize {
    let hhdm_offset = HHDM_REQUEST.response().unwrap().offset;
    paddr + hhdm_offset as usize
}

pub unsafe fn virt_to_phys(vaddr: usize) -> usize {
    let mut par: u64;
    core::arch::asm!(
        "at s1e1r, {0}",
        "isb",
        "mrs {1}, par_el1",
        in(reg) vaddr as u64,
        out(reg) par,
        options(nostack)
    );
    if (par & 1) == 0 {
        let phys_page = par & 0x0000_FFFF_FFFF_F000;
        let offset = (vaddr as u64) & 0xFFF;
        return (phys_page | offset) as usize;
    }

    let hhdm_offset = HHDM_REQUEST.response().unwrap().offset as usize;
    if vaddr >= hhdm_offset {
        vaddr - hhdm_offset
    } else {
        vaddr
    }
}

pub unsafe fn map_user_page(vaddr: usize, paddr: usize) {
    // Legacy fallback: RW, NX
    map_user_page_flags(vaddr, paddr, true, false);
}
pub fn create_user_address_space() -> usize {
    let root_paddr = super::pmm::alloc_frame().expect("OOM root pt");
    let root_vaddr = unsafe { phys_to_virt(root_paddr) as *mut u8 };
    unsafe {
        crate::mm::himada_page_zero(root_vaddr, 4096);
        crate::graphics::clean_dcache_range(root_vaddr as usize, 4096);

        // Always map PL011 UART so kernel EL1 serial logging never faults
        let uart = if crate::hal::serial::UART_BASE != 0 {
            crate::hal::serial::UART_BASE
        } else {
            0x0900_0000
        };
        map_device_page_in_root(root_paddr, uart, uart);

        // Map QEMU VirtIO MMIO slots (0x0a000000..=0x0a004000)
        for page in (0x0a00_0000..=0x0a00_4000).step_by(4096) {
            map_device_page_in_root(root_paddr, page, page);
        }
    }
    root_paddr
}

pub unsafe fn map_page_in_root(root_paddr: usize, vaddr: usize, paddr: usize, writable: bool, executable: bool) {
    if writable && executable {
        panic!("Security Violation: W^X protection triggered. Page cannot be both writable and executable.");
    }
    let uxn = if executable { 0 } else { 1u64 << 54 }; // Unprivileged Execute Never
    let pxn = 1u64 << 53; // Privileged Execute Never (userspace pages shouldn't be executed by kernel)
    let ap = if writable { 0b01u64 << 6 } else { 0b11u64 << 6 }; // RW or RO

    let l0_index = (vaddr >> 39) & 0x1FF;
    let l1_index = (vaddr >> 30) & 0x1FF;
    let l2_index = (vaddr >> 21) & 0x1FF;
    let l3_index = (vaddr >> 12) & 0x1FF;

    let mut table_phys = root_paddr;

    let indices = [l0_index, l1_index, l2_index];
    for &idx in &indices {
        let table_virt = phys_to_virt(table_phys) as *mut u64;
        let entry = core::ptr::read(table_virt.add(idx));
        
        if entry & 1 == 0 {
            // Allocate next level table
            let next_phys = super::pmm::alloc_frame().expect("OOM pt");
            let next_virt = phys_to_virt(next_phys) as *mut u8;
            crate::mm::himada_page_zero(next_virt as *mut u8, 4096);
            crate::graphics::clean_dcache_range(next_virt as usize, 4096);
            
            // Valid | Table (bits 1:0 = 0b11)
            let new_entry = (next_phys as u64) | 0b11;
            core::ptr::write(table_virt.add(idx), new_entry);
            crate::graphics::clean_dcache_range(table_virt.add(idx) as usize, 8);
            table_phys = next_phys;
        } else {
            table_phys = (entry & 0x0000_FFFF_FFFF_F000) as usize;
        }
    }

    let l3_table_virt = phys_to_virt(table_phys) as *mut u64;
    let page_entry = (paddr as u64) | 0b11 | (1 << 10) | ap | (0b11 << 8) | uxn | pxn;
    core::ptr::write(l3_table_virt.add(l3_index), page_entry);
    crate::graphics::clean_dcache_range(l3_table_virt.add(l3_index) as usize, 8);
    
    core::arch::asm!("dsb ish");
    core::arch::asm!("tlbi vmalle1is");
    core::arch::asm!("dsb ish");
    core::arch::asm!("isb");
}

pub unsafe fn clone_user_address_space(src_root: usize) -> usize {
    let dst_root = create_user_address_space();
    let l0_src = phys_to_virt(src_root) as *const u64;

    // Scan userspace half (indices 0..256)
    for l0 in 0..256 {
        let e0 = core::ptr::read(l0_src.add(l0));
        if e0 & 1 == 0 { continue; }
        let l1_phys = (e0 & 0x0000_FFFF_FFFF_F000) as usize;
        let l1_src = phys_to_virt(l1_phys) as *const u64;

        for l1 in 0..512 {
            let e1 = core::ptr::read(l1_src.add(l1));
            if e1 & 1 == 0 { continue; }
            let l2_phys = (e1 & 0x0000_FFFF_FFFF_F000) as usize;
            let l2_src = phys_to_virt(l2_phys) as *const u64;

            for l2 in 0..512 {
                let e2 = core::ptr::read(l2_src.add(l2));
                if e2 & 1 == 0 { continue; }
                let l3_phys = (e2 & 0x0000_FFFF_FFFF_F000) as usize;
                let l3_src = phys_to_virt(l3_phys) as *const u64;

                for l3 in 0..512 {
                    let e3 = core::ptr::read(l3_src.add(l3));
                    if e3 & 0b11 != 0b11 { continue; }

                    let src_page_paddr = (e3 & 0x0000_FFFF_FFFF_F000) as usize;
                    let vaddr = (l0 << 39) | (l1 << 30) | (l2 << 21) | (l3 << 12);
                    let mut writable = (e3 & (0b10 << 6)) == 0;
                    let mut executable = (e3 & (1u64 << 54)) == 0;

                    // If it's an identity mapping (device MMIO, ACPI tables, firmware) or device MMIO space, share physical page
                    if vaddr == src_page_paddr || src_page_paddr < 0x4000_0000 || src_page_paddr >= 0x8000_0000_00 {
                        map_page_in_root(dst_root, vaddr, src_page_paddr, writable, false);
                    } else {
                        // W^X Security enforcement: if both writable and executable, drop executable
                        if writable && executable {
                            executable = false;
                        }

                        // Allocate new frame and copy using Himada SIMD
                        let dst_page_paddr = super::pmm::alloc_frame().expect("OOM cloning user page");
                        let src_virt = phys_to_virt(src_page_paddr) as *const u8;
                        let dst_virt = phys_to_virt(dst_page_paddr) as *mut u8;

                        crate::mm::himada_page_copy(dst_virt, src_virt, 4096);
                        crate::graphics::clean_dcache_range(dst_virt as usize, 4096);

                        map_page_in_root(dst_root, vaddr, dst_page_paddr, writable, executable);
                    }
                }
            }
        }
    }

    dst_root
}

pub unsafe fn destroy_user_address_space(root_paddr: usize) {
    if root_paddr == 0 { return; }
    let l0_src = phys_to_virt(root_paddr) as *mut u64;

    // Scan userspace half (indices 0..256)
    for l0 in 0..256 {
        let e0 = core::ptr::read(l0_src.add(l0));
        if e0 & 1 == 0 { continue; }
        let l1_phys = (e0 & 0x0000_FFFF_FFFF_F000) as usize;
        let l1_src = phys_to_virt(l1_phys) as *mut u64;

        for l1 in 0..512 {
            let e1 = core::ptr::read(l1_src.add(l1));
            if e1 & 1 == 0 { continue; }
            let l2_phys = (e1 & 0x0000_FFFF_FFFF_F000) as usize;
            let l2_src = phys_to_virt(l2_phys) as *mut u64;

            for l2 in 0..512 {
                let e2 = core::ptr::read(l2_src.add(l2));
                if e2 & 1 == 0 { continue; }
                let l3_phys = (e2 & 0x0000_FFFF_FFFF_F000) as usize;
                let l3_src = phys_to_virt(l3_phys) as *mut u64;

                for l3 in 0..512 {
                    let e3 = core::ptr::read(l3_src.add(l3));
                    if e3 & 0b11 != 0b11 { continue; }

                    let paddr = (e3 & 0x0000_FFFF_FFFF_F000) as usize;
                    let vaddr = (l0 << 39) | (l1 << 30) | (l2 << 21) | (l3 << 12);

                    // Skip identity-mapped device MMIO regions (< 1GB or high device space)
                    if !(vaddr == paddr || paddr < 0x4000_0000 || paddr >= 0x8000_0000_00) {
                        super::pmm::free_frame(paddr);
                    }
                }
                super::pmm::free_frame(l3_phys);
            }
            super::pmm::free_frame(l2_phys);
        }
        super::pmm::free_frame(l1_phys);
    }
    super::pmm::free_frame(root_paddr);
}

pub unsafe fn map_user_page_flags(vaddr: usize, paddr: usize, writable: bool, executable: bool) {
    let root_paddr: usize;
    if vaddr >= 0xFFFF_0000_0000_0000 {
        core::arch::asm!("mrs {}, ttbr1_el1", out(reg) root_paddr);
    } else {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) root_paddr);
    }
    map_page_in_root(root_paddr, vaddr, paddr, writable, executable);
}

pub unsafe fn map_device_page_in_root(root_paddr: usize, vaddr: usize, paddr: usize) {
    let l0_index = (vaddr >> 39) & 0x1FF;
    let l1_index = (vaddr >> 30) & 0x1FF;
    let l2_index = (vaddr >> 21) & 0x1FF;
    let l3_index = (vaddr >> 12) & 0x1FF;

    let mut table_phys = root_paddr;

    let indices = [l0_index, l1_index, l2_index];
    for &idx in &indices {
        let table_virt = phys_to_virt(table_phys) as *mut u64;
        let entry = core::ptr::read(table_virt.add(idx));
        
        if entry & 1 == 0 {
            let next_phys = super::pmm::alloc_frame().expect("OOM pt");
            let next_virt = phys_to_virt(next_phys) as *mut u8;
            crate::mm::himada_page_zero(next_virt as *mut u8, 4096);
            crate::graphics::clean_dcache_range(next_virt as usize, 4096);
            
            let new_entry = (next_phys as u64) | 0b11;
            core::ptr::write(table_virt.add(idx), new_entry);
            crate::graphics::clean_dcache_range(table_virt.add(idx) as usize, 8);
            table_phys = next_phys;
        } else {
            table_phys = (entry & 0x0000_FFFF_FFFF_F000) as usize;
        }
    }

    let l3_table_virt = phys_to_virt(table_phys) as *mut u64;
    // Valid (0b11) | AF (bit 10) | AP=01 (RW EL0/EL1) | SH=10 (Outer Shareable) | AttrIndx=0 (Device memory) | UXN | PXN
    let page_entry = (paddr as u64) | 0b11 | (1 << 10) | (0b01 << 6) | (0b10 << 8) | (0 << 2) | (1u64 << 54) | (1u64 << 53);
    let cur_entry = core::ptr::read(l3_table_virt.add(l3_index));
    if cur_entry == page_entry {
        return;
    }
    core::ptr::write(l3_table_virt.add(l3_index), page_entry);
    crate::graphics::clean_dcache_range(l3_table_virt.add(l3_index) as usize, 8);
    
    core::arch::asm!("dsb ish");
    core::arch::asm!("tlbi vmalle1is");
    core::arch::asm!("dsb ish");
    core::arch::asm!("isb");
}

pub unsafe fn map_device_page(vaddr: usize, paddr: usize) {
    let root_paddr: usize;
    if vaddr >= 0xFFFF_0000_0000_0000 {
        core::arch::asm!("mrs {}, ttbr1_el1", out(reg) root_paddr);
    } else {
        core::arch::asm!("mrs {}, ttbr0_el1", out(reg) root_paddr);
    }
    map_device_page_in_root(root_paddr, vaddr, paddr);
}
