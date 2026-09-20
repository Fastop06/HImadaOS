//! ARM64 PSCI (Power State Coordination Interface) Implementation
//!
//! Provides standard SMC/HVC interface for CPU core power management
//! on AArch64 systems (QEMU Virt, Cortex-A72, Apple Silicon, real hardware).

#![allow(dead_code)]

pub const PSCI_VERSION: u32 = 0x8400_0000;
pub const PSCI_CPU_ON_32: u32 = 0x8400_0003;
pub const PSCI_CPU_ON_64: u32 = 0xC400_0003;
pub const PSCI_CPU_OFF: u32 = 0x8400_0002;
pub const PSCI_SYSTEM_OFF: u32 = 0x8400_0008;
pub const PSCI_SYSTEM_RESET: u32 = 0x8400_0009;

/// Execute PSCI call using HVC instruction (Hypervisor Call, standard in EL1 virtual machines)
#[inline(always)]
pub unsafe fn psci_hvc(func: u32, arg0: u64, arg1: u64, arg2: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "hvc #0",
        inout("x0") func as u64 => ret,
        in("x1") arg0,
        in("x2") arg1,
        in("x3") arg2,
        options(nomem, nostack)
    );
    ret
}

/// Execute PSCI call using SMC instruction (Secure Monitor Call, standard in bare-metal EL1/EL2)
#[inline(always)]
pub unsafe fn psci_smc(func: u32, arg0: u64, arg1: u64, arg2: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "smc #0",
        inout("x0") func as u64 => ret,
        in("x1") arg0,
        in("x2") arg1,
        in("x3") arg2,
        options(nomem, nostack)
    );
    ret
}

/// Power on target secondary CPU core via PSCI CPU_ON
pub unsafe fn cpu_on(target_mpidr: u64, entry_addr: u64, context_id: u64) -> i64 {
    // Try HVC first (standard for QEMU virt without TrustZone)
    let res = psci_hvc(PSCI_CPU_ON_64, target_mpidr, entry_addr, context_id);
    if res == 0 {
        return 0;
    }
    // Fall back to SMC if HVC returned non-zero / unsupported
    psci_smc(PSCI_CPU_ON_64, target_mpidr, entry_addr, context_id)
}
