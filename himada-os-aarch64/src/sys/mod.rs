pub mod syscall;
pub mod linux_abi;
pub mod elf;
pub mod gpu;
pub mod process;
pub mod futex;

pub fn init() {
    syscall::init();
    process::init_idle_contexts();
}

#[repr(C, align(4096))]
pub struct AlignedKernelStack(pub [u8; 65536]);

pub static mut KERNEL_STACK: AlignedKernelStack = AlignedKernelStack([0; 65536]);

pub unsafe fn jump_to_ring3(entry: usize, stack_top: usize) -> ! {
    use core::arch::asm;
    
    let kstack_top = (KERNEL_STACK.0.as_mut_ptr() as usize) + 65536;

    // 1. Switch EL1 to use SP_EL1 so SP_EL0 is no longer active
    // 2. Set SP (which is now SP_EL1) to dedicated aligned kernel stack kstack_top
    // 3. Set SP_EL0 to userspace stack pointer
    // 4. Set SPSR_EL1 to EL0t (DAIF masked: 0x3c0)
    // 5. Set ELR_EL1 to entry point
    // 6. SMP broadcast invalidate all instruction caches and memory barriers
    // 7. eret into userspace EL0!
    asm!(
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
        in(reg) kstack_top,
        in(reg) stack_top,
        in(reg) 0x3c0u64,
        in(reg) entry,
        options(noreturn)
    );
}
pub mod exception;
