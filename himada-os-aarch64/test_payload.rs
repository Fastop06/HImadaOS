#![no_std]
#![no_main]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello from Linux Userspace inside HimadaOS!\n";
    unsafe {
        core::arch::asm!(
            "mov x8, #64", // sys_write
            "mov x0, #1",  // fd = stdout
            "mov x1, {0}", // buf
            "mov x2, {1}", // count
            "svc #0",
            in(reg) msg.as_ptr(),
            in(reg) msg.len(),
        );
        core::arch::asm!(
            "mov x8, #93", // sys_exit
            "mov x0, #42", // status = 42
            "svc #0",
        );
    }
    loop {}
}
