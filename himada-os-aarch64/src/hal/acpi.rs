extern crate alloc;
use limine::request::RsdpRequest;
use crate::serial_println;
use crate::mm::vmm::phys_to_virt;

#[used]
#[link_section = ".requests"]
pub static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[repr(C, packed)]
pub struct AcpiHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

pub struct AcpiInfo {
    pub ecam_base: Option<usize>,
    pub spcr_base: Option<usize>,
    pub spcr_interface: Option<u8>,
    pub has_ps2: bool,
}

pub static mut ACPI_INFO: AcpiInfo = AcpiInfo {
    ecam_base: None,
    spcr_base: None,
    spcr_interface: None,
    has_ps2: false,
};

#[inline(always)]
unsafe fn ensure_virt(addr: usize) -> *const u8 {
    if (addr as u64) & (1 << 63) != 0 {
        addr as *const u8
    } else {
        let page = addr & !0xFFF;
        crate::mm::vmm::map_device_page(page, page);
        crate::mm::vmm::map_device_page(page + 4096, page + 4096);
        addr as *const u8
    }
}

pub fn init() {
    let rsdp_resp = match RSDP_REQUEST.response() {
        Some(r) => r,
        None => {
            serial_println!("[ACPI] No RSDP response from bootloader.");
            return;
        }
    };

    let rsdp_raw = rsdp_resp.address as usize;
    if rsdp_raw == 0 {
        serial_println!("[ACPI] RSDP pointer is null.");
        return;
    }
    let rsdp_ptr = unsafe { ensure_virt(rsdp_raw) };

    unsafe {
        let sig = core::slice::from_raw_parts(rsdp_ptr, 8);
        if sig != b"RSD PTR " {
            serial_println!("[ACPI] Invalid RSDP signature");
            return;
        }

        let revision = *rsdp_ptr.add(15);
        serial_println!("[ACPI] RSDP found, revision: {}", revision);

        let mut xsdt_vaddr: Option<*const u8> = None;
        if revision >= 2 {
            let xsdt_paddr = core::ptr::read_unaligned(rsdp_ptr.add(24) as *const u64) as usize;
            if xsdt_paddr != 0 {
                let v = ensure_virt(xsdt_paddr);
                xsdt_vaddr = Some(v);
                serial_println!("[ACPI] XSDT physical address: {:#X}", xsdt_paddr);
            }
        }

        let (table_vaddr, is_64) = if let Some(x) = xsdt_vaddr {
            (x, true)
        } else {
            let rsdt_paddr = core::ptr::read_unaligned(rsdp_ptr.add(16) as *const u32);
            if rsdt_paddr == 0 {
                serial_println!("[ACPI] Neither XSDT nor RSDT available.");
                return;
            }
            let v = ensure_virt(rsdt_paddr as usize);
            (v, false)
        };

        let header = &*(table_vaddr as *const AcpiHeader);
        let header_length = core::ptr::read_unaligned(core::ptr::addr_of!(header.length));
        let sig = core::str::from_utf8(&header.signature).unwrap_or("????");
        serial_println!("[ACPI] Root table signature: {}, length: {}", sig, header_length);

        let entry_size = if is_64 { 8 } else { 4 };
        let entries_len = header_length.saturating_sub(36) as usize;
        let entry_count = entries_len / entry_size;

        for i in 0..entry_count {
            let entry_ptr = table_vaddr.add(36 + i * entry_size);
            let table_paddr: usize = if is_64 {
                core::ptr::read_unaligned(entry_ptr as *const u64) as usize
            } else {
                core::ptr::read_unaligned(entry_ptr as *const u32) as usize
            };

            if table_paddr == 0 { continue; }

            let t_vaddr = ensure_virt(table_paddr);

            let t_hdr = &*(t_vaddr as *const AcpiHeader);
            let t_length = core::ptr::read_unaligned(core::ptr::addr_of!(t_hdr.length));
            let t_sig = core::str::from_utf8(&t_hdr.signature).unwrap_or("????");
            serial_println!("[ACPI] Found table: [{}] at {:#X} (len: {})", t_sig, table_paddr, t_length);

            if t_sig == "MCFG" {
                if t_length >= 44 + 16 {
                    let base = core::ptr::read_unaligned(t_vaddr.add(44) as *const u64) as usize;
                    serial_println!("[ACPI] MCFG: PCIe ECAM Base = {:#X}", base);
                    ACPI_INFO.ecam_base = Some(base);
                }
            } else if t_sig == "SPCR" {
                if t_length >= 52 {
                    let interface_type = *t_vaddr.add(36);
                    let base_gas = t_vaddr.add(40);
                    let addr = core::ptr::read_unaligned(base_gas.add(4) as *const u64) as usize;
                    serial_println!("[ACPI] SPCR: Serial Console type = {}, base = {:#X}", interface_type, addr);
                    ACPI_INFO.spcr_interface = Some(interface_type);
                    ACPI_INFO.spcr_base = Some(addr);
                }
            } else if t_sig == "FACP" {
                if t_length >= 111 {
                    let flags = core::ptr::read_unaligned(t_vaddr.add(109) as *const u16);
                    let has_8042 = (flags & (1 << 1)) != 0;
                    serial_println!("[ACPI] FADT: IAPC Boot Arch = {:#X}, 8042 PS/2 = {}", flags, has_8042);
                    ACPI_INFO.has_ps2 = has_8042;
                }
            }
        }
    }
}
