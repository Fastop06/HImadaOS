use crate::sys::linux_abi;
use crate::hal::exceptions::ExceptionContext;
use crate::{serial_print, serial_println};

pub fn init() {
    // AArch64 syscalls use the SVC instruction which traps to the Exception Vector.
    // Initialization is handled by HAL exception vectors.
}

pub fn handle_svc(ctx: &mut ExceptionContext) {
    let syscall_no = ctx.x[8]; // x8 contains the syscall number
    let arg0 = ctx.x[0];
    let arg1 = ctx.x[1];
    let arg2 = ctx.x[2];
    
    // Call the linux_abi handler
    let ret = linux_abi::dispatch_syscall(syscall_no, arg0, arg1, arg2);
    
    // Store return value in x0
    ctx.x[0] = ret;
}
