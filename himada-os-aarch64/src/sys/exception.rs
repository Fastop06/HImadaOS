use crate::hal::exceptions::ExceptionContext;

pub fn handle_user_fault(context: &mut ExceptionContext, esr: u64, far: u64) {
    let ec = (esr >> 26) & 0x3F;
    let iss = esr & 0x1FFFFFF;

    crate::serial_println!("========================================");
    crate::serial_println!("        USERSPACE FAULT (SIGSEGV)       ");
    crate::serial_println!("========================================");
    crate::serial_println!("Fault Address (FAR): {:#018x}", far);
    crate::serial_println!("Instruction (ELR):   {:#018x}", context.elr);
    for i in 0..31 {
        crate::serial_println!("X{:02} = {:#018x}", i, context.x[i]);
    }
    
    match ec {
        0x20 => crate::serial_println!("Instruction Abort from a lower Exception level"),
        0x21 => crate::serial_println!("Instruction Abort taken without a change in Exception level"),
        0x24 => crate::serial_println!("Data Abort from a lower Exception level"),
        0x25 => crate::serial_println!("Data Abort taken without a change in Exception level"),
        _    => crate::serial_println!("Unknown Fault (EC={:#x})", ec),
    }

    if ec == 0x20 || ec == 0x24 {
        let wnr = (iss & (1 << 6)) != 0;
        let dfsc = iss & 0x3F;
        crate::serial_println!("Access Type: {}", if wnr { "WRITE" } else { "READ" });
        if dfsc == 0b001101 || dfsc == 0b001111 { // Permission fault
            crate::serial_println!("Reason: Permission Violation (W^X / Read-Only)");
        } else {
            crate::serial_println!("Reason: Translation Fault (Unmapped Page)");
        }
    }

    crate::serial_println!("Killing process...");
    
    // In Phase 3, this will call process::exit_current(139) and schedule next.
    // For now, we halt.
    loop {
        unsafe { core::arch::asm!("wfe"); }
    }
}
