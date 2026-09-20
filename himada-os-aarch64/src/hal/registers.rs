use core::arch::asm;

pub fn init() {
    unsafe {
        // Enable FP/SIMD by setting CPACR_EL1 bits 20:21 to 11
        let mut cpacr: u64;
        asm!("mrs {}, cpacr_el1", out(reg) cpacr);
        cpacr |= (3 << 20);
        asm!("msr cpacr_el1, {}", in(reg) cpacr);
        asm!("isb");

        // Ensure SCTLR_EL1.SPAN (bit 23) is set so PSTATE.PAN is not set on exception entry
        let mut sctlr: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr);
        sctlr |= (1 << 23);
        asm!("msr sctlr_el1, {}", in(reg) sctlr);
        asm!("isb");
    }
}
