use crate::serial_println;

pub fn init_mmio(base_phys_addr: usize) {
    unsafe {
        // Convert physical address from DTB to virtual HHDM address
        let vaddr = crate::mm::vmm::phys_to_virt(base_phys_addr);
        
        let magic = core::ptr::read_volatile(vaddr as *const u32);
        
        // "virt" in little endian is 0x74726976
        if magic != 0x74726976 {
            serial_println!("  -> Invalid VirtIO magic: 0x{:X}", magic);
            return;
        }

        let version = core::ptr::read_volatile((vaddr + 0x4) as *const u32);
        let device_id = core::ptr::read_volatile((vaddr + 0x8) as *const u32);

        serial_println!("  -> VirtIO Device detected! Version: {}, Device ID: {}", version, device_id);
        
        match device_id {
            1 => serial_println!("     [Network Device (virtio-net)]"),
            2 => serial_println!("     [Block Device (virtio-blk)]"),
            16 => serial_println!("     [GPU Device (virtio-gpu)]"),
            _ => serial_println!("     [Other Device]"),
        }
    }
}
