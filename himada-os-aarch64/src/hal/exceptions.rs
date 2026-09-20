use core::arch::{asm, global_asm};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExceptionContext {
    pub x: [u64; 31],
    pub elr: u64,
    pub spsr: u64,
    pub sp: u64,
}

global_asm!(
r#"
.macro SAVE_CONTEXT
    sub sp, sp, #272 // 31 * 8 (regs) + 3 * 8 (elr, spsr, sp)
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]

    mrs x0, elr_el1
    mrs x1, spsr_el1
    mrs x2, sp_el0
    stp x0, x1, [sp, #248]
    str x2, [sp, #264]
.endm

.macro RESTORE_CONTEXT
    ldp x0, x1, [sp, #248]
    ldr x2, [sp, #264]
    msr elr_el1, x0
    msr spsr_el1, x1
    msr sp_el0, x2

    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    // Skip x8 if we want to return a value from syscall (usually x0 is return, we restore it unless modified)
    // Actually, x0 is modified by C code if it's a syscall. The C code modifies the struct.
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x19, [sp, #144]
    ldp x20, x21, [sp, #160]
    ldp x22, x23, [sp, #176]
    ldp x24, x25, [sp, #192]
    ldp x26, x27, [sp, #208]
    ldp x28, x29, [sp, #224]
    ldr x30, [sp, #240]
    add sp, sp, #272
.endm

.macro VENTRY label
    .align 7
    b \label
.endm

.align 11
.global exception_vector_table
exception_vector_table:
    // Current EL with SP0 (vectors 0..3)
    VENTRY vec_handler_0
    VENTRY vec_handler_1
    VENTRY vec_handler_2
    VENTRY vec_handler_3

    // Current EL with SPx (vectors 4..7)
    VENTRY vec_handler_4
    VENTRY vec_handler_5
    VENTRY vec_handler_6
    VENTRY vec_handler_7

    // Lower EL using AArch64 (vectors 8..11)
    VENTRY vec_handler_8
    VENTRY vec_handler_9
    VENTRY vec_handler_10
    VENTRY vec_handler_11

    // Lower EL using AArch32 (vectors 12..15)
    VENTRY vec_handler_12
    VENTRY vec_handler_13
    VENTRY vec_handler_14
    VENTRY vec_handler_15

.macro DEF_HANDLER id
vec_handler_\id:
    SAVE_CONTEXT
    mov x0, sp
    mov x1, #\id
    bl exception_handler_c
    RESTORE_CONTEXT
    eret
.endm

    DEF_HANDLER 0
    DEF_HANDLER 1
    DEF_HANDLER 2
    DEF_HANDLER 3
    DEF_HANDLER 4
    DEF_HANDLER 5
    DEF_HANDLER 6
    DEF_HANDLER 7
    DEF_HANDLER 8
    DEF_HANDLER 9
    DEF_HANDLER 10
    DEF_HANDLER 11
    DEF_HANDLER 12
    DEF_HANDLER 13
    DEF_HANDLER 14
    DEF_HANDLER 15
"#
);

extern "C" {
    pub static exception_vector_table: u8;
}

pub fn init() {
    unsafe {
        asm!(
            "msr vbar_el1, {}",
            in(reg) &exception_vector_table as *const _ as u64
        );
        asm!("isb");
    }
}

fn write_hex64(mut val: u64, out: &mut [u8], offset: usize) -> usize {
    out[offset] = b'0';
    out[offset + 1] = b'x';
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for i in (0..16).rev() {
        out[offset + 2 + (15 - i)] = HEX[((val >> (i * 4)) & 0xF) as usize];
    }
    offset + 18
}

fn write_str(s: &str, out: &mut [u8], mut offset: usize) -> usize {
    for &b in s.as_bytes() {
        if offset < out.len() {
            out[offset] = b;
            offset += 1;
        }
    }
    offset
}

#[no_mangle]
pub extern "C" fn exception_handler_c(ctx: *mut ExceptionContext, vector_id: u64) {
    let context = unsafe { &mut *ctx };
    let esr: u64;
    let far: u64;
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) esr);
        asm!("mrs {}, far_el1", out(reg) far);
    }

    let ec = (esr >> 26) & 0x3F; // Exception Class

    if vector_id == 8 && (ec == 0x20 || ec == 0x24) {
        // Instruction Abort or Data Abort from Userspace
        crate::sys::exception::handle_user_fault(context, esr, far);
        return;
    } else if vector_id == 8 && ec == 0x15 {
        // SVC instruction execution in AArch64 state (Syscall)
        let ret = crate::sys::linux_abi::dispatch_syscall_ctx(context);
        context.x[0] = ret; // Return value in x0
    } else {
        let vec_names = [
            "Curr-SP0-Sync", "Curr-SP0-IRQ", "Curr-SP0-FIQ", "Curr-SP0-SError",
            "Curr-SPx-Sync", "Curr-SPx-IRQ", "Curr-SPx-FIQ", "Curr-SPx-SError",
            "Lower-Sync",    "Lower-IRQ",    "Lower-FIQ",    "Lower-SError",
            "Lower32-Sync",  "Lower32-IRQ",  "Lower32-FIQ",  "Lower32-SError",
        ];
        let vname = if (vector_id as usize) < vec_names.len() {
            vec_names[vector_id as usize]
        } else {
            "Unknown"
        };

        crate::serial_println!("CPU EXCEPTION: [{}] ESR={:#018x} FAR={:#018x} ELR={:#018x}", vname, esr, far, context.elr);

        let cpu = crate::hal::smp::current_cpu_id();
        let ksp = ctx as usize + 272;
        crate::serial_println!(
            "[EXCEPTION] CPU: {}, LR: {:#018x}, SP_EL1: {:#018x}, SP_EL0: {:#018x}",
            cpu, context.x[30], ksp, context.sp
        );
        crate::serial_println!(
            "[EXCEPTION] x0..x3:   {:#018x} {:#018x} {:#018x} {:#018x}",
            context.x[0], context.x[1], context.x[2], context.x[3]
        );
        crate::serial_println!(
            "[EXCEPTION] x4..x7:   {:#018x} {:#018x} {:#018x} {:#018x}",
            context.x[4], context.x[5], context.x[6], context.x[7]
        );
        crate::serial_println!(
            "[EXCEPTION] x8..x11:  {:#018x} {:#018x} {:#018x} {:#018x}",
            context.x[8], context.x[9], context.x[10], context.x[11]
        );
        crate::serial_println!(
            "[EXCEPTION] x12..x15: {:#018x} {:#018x} {:#018x} {:#018x}",
            context.x[12], context.x[13], context.x[14], context.x[15]
        );
        crate::serial_println!(
            "[EXCEPTION] x16..x19: {:#018x} {:#018x} {:#018x} {:#018x}",
            context.x[16], context.x[17], context.x[18], context.x[19]
        );
        crate::serial_println!(
            "[EXCEPTION] x20..x23: {:#018x} {:#018x} {:#018x} {:#018x}",
            context.x[20], context.x[21], context.x[22], context.x[23]
        );
        crate::serial_println!(
            "[EXCEPTION] x24..x27: {:#018x} {:#018x} {:#018x} {:#018x}",
            context.x[24], context.x[25], context.x[26], context.x[27]
        );
        crate::serial_println!(
            "[EXCEPTION] x28..x29: {:#018x} {:#018x}",
            context.x[28], context.x[29]
        );
        crate::serial_println!(
            "[EXCEPTION] SPSR: {:#018x}",
            context.spsr
        );

        // Dump 16 words from SP_EL1
        let sp_words = unsafe { core::slice::from_raw_parts(ksp as *const u64, 16) };
        for i in (0..16).step_by(2) {
            crate::serial_println!(
                "[STACK +{:02x}] {:#018x} {:#018x}",
                i * 8, sp_words[i], sp_words[i + 1]
            );
        }

        loop {
            unsafe { core::arch::asm!("wfe"); }
        }
    }
}
