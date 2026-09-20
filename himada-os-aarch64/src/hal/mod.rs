pub mod serial;
pub mod exceptions;
pub mod registers;
pub mod dtb;
pub mod dev;

pub fn init() {
    registers::init();
    exceptions::init();
    
    let dtb_ptr = crate::DTB_REQUEST.response()
        .map(|r| r.dtb_ptr as *const u8)
        .unwrap_or(core::ptr::null());
    
    dtb::init(dtb_ptr);
}
pub mod virtio;
pub mod device;
pub mod acpi;
pub mod parallels_kbd;
pub mod xhci;
pub mod kmi;
pub mod virtio_input;
pub mod virtio_blk;
pub mod psci;
pub mod smp;
