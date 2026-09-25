//! Symmetric Multiprocessing (SMP) Subsystem for AArch64 Cortex-A72
//!
//! Manages CPU topology, multi-core boot synchronization, per-CPU stacks,
//! and secondary core execution loops.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::serial_println;

pub const MAX_CPUS: usize = 8;
pub const CPU_STACK_SIZE: usize = 65536; // 64 KiB stack per CPU core

#[repr(C, align(4096))]
pub struct CpuStack(pub [u8; CPU_STACK_SIZE]);

pub static mut CPU_BOOT_STACKS: [CpuStack; MAX_CPUS] = [
    CpuStack([0; CPU_STACK_SIZE]),
    CpuStack([0; CPU_STACK_SIZE]),
    CpuStack([0; CPU_STACK_SIZE]),
    CpuStack([0; CPU_STACK_SIZE]),
    CpuStack([0; CPU_STACK_SIZE]),
    CpuStack([0; CPU_STACK_SIZE]),
    CpuStack([0; CPU_STACK_SIZE]),
    CpuStack([0; CPU_STACK_SIZE]),
];

pub static CPU_ONLINE_COUNT: AtomicUsize = AtomicUsize::new(1); // BSP starts online
pub static CPU_ONLINE: [AtomicBool; MAX_CPUS] = [
    AtomicBool::new(true),  // Core 0 (BSP)
    AtomicBool::new(false), // Core 1
    AtomicBool::new(false), // Core 2
    AtomicBool::new(false), // Core 3
    AtomicBool::new(false), // Core 4
    AtomicBool::new(false), // Core 5
    AtomicBool::new(false), // Core 6
    AtomicBool::new(false), // Core 7
];

pub static SMP_SCHEDULER_READY: AtomicBool = AtomicBool::new(false);
pub static mut CPU_MPIDRS: [u64; MAX_CPUS] = [0; MAX_CPUS];

/// Get currently executing CPU core ID from TPIDR_EL1
#[inline(always)]
pub fn current_cpu_id() -> usize {
    let id: u64;
    unsafe {
        core::arch::asm!("mrs {}, tpidr_el1", out(reg) id);
    }
    if (id as usize) < MAX_CPUS {
        id as usize
    } else {
        0
    }
}

/// Total count of active online CPU cores
#[inline(always)]
pub fn get_cpu_count() -> usize {
    CPU_ONLINE_COUNT.load(Ordering::Relaxed)
}

/// Check if a specific CPU core is online
pub fn is_cpu_online(cpu_id: usize) -> bool {
    if cpu_id < MAX_CPUS {
        CPU_ONLINE[cpu_id].load(Ordering::Acquire)
    } else {
        false
    }
}

/// Primary SMP initialization on Bootstrap Processor (BSP)
pub fn init() {
    // Set BSP TPIDR_EL1 to core 0
    unsafe {
        core::arch::asm!("msr tpidr_el1, {}", in(reg) 0u64);
    }

    if let Some(resp) = crate::MP_REQUEST.response() {
        let bsp_mpidr = resp.bsp_mpidr;
        let cpus = resp.cpus();
        serial_println!(
            "[SMP] Bootloader MP response received: {} CPU core(s) present. BSP MPIDR: {:#x}",
            cpus.len(),
            bsp_mpidr
        );

        unsafe {
            CPU_MPIDRS[0] = bsp_mpidr;
        }

        let mut next_core_idx = 1;
        for cpu in cpus.iter() {
            if cpu.mpidr == bsp_mpidr {
                continue; // Skip BSP
            }
            if next_core_idx >= MAX_CPUS {
                break;
            }

            let core_id = next_core_idx;
            next_core_idx += 1;

            unsafe {
                CPU_MPIDRS[core_id] = cpu.mpidr;
            }

            serial_println!(
                "[SMP] Bootstrapping secondary CPU #{} (Processor ID {}, MPIDR {:#x})...",
                core_id,
                cpu.processor_id,
                cpu.mpidr
            );

            // Bootstrap secondary core using Limine MP protocol
            cpu.bootstrap(secondary_cpu_entry, core_id as u64);
        }

        // Wait for secondary cores to report online
        let mut waited = 0;
        let expected = cpus.len().min(MAX_CPUS);
        while CPU_ONLINE_COUNT.load(Ordering::SeqCst) < expected && waited < 50_000_000 {
            core::hint::spin_loop();
            waited += 1;
        }

        let total_online = CPU_ONLINE_COUNT.load(Ordering::SeqCst);
        serial_println!(
            "[SMP] Multi-Core Bringup complete: {} / {} cores online and synchronized.",
            total_online,
            cpus.len()
        );
        crate::log_step(
            &alloc::format!("SMP: {} Cores Online (Cortex-A72)", total_online),
            Some("OK"),
        );
    } else {
        serial_println!("[SMP] Limine MP response absent; running in single-core mode.");
        crate::log_step("SMP: Single-core mode active.", Some("OK"));
    }
}

/// Entry point executed by secondary processors upon bootstrap
pub unsafe extern "C" fn secondary_cpu_entry(info: &limine::mp::MpInfo) -> ! {
    let cpu_id = info.extra_argument() as usize;
    if cpu_id >= MAX_CPUS {
        loop {
            core::arch::asm!("wfe");
        }
    }

    // 1. Enable NEON/FP SIMD coprocessor in CPACR_EL1 (Bits 20:21 = 0b11)
    let mut cpacr: u64;
    core::arch::asm!("mrs {}, cpacr_el1", out(reg) cpacr);
    cpacr |= 3 << 20;
    core::arch::asm!("msr cpacr_el1, {}", in(reg) cpacr);
    core::arch::asm!("isb");

    // 3. Install Exception Vector Table (VBAR_EL1)
    core::arch::asm!(
        "msr vbar_el1, {}",
        in(reg) &crate::hal::exceptions::exception_vector_table as *const _ as u64
    );
    core::arch::asm!("isb");

    // 4. Store CPU ID in TPIDR_EL1 (Thread Pointer)
    core::arch::asm!("msr tpidr_el1, {}", in(reg) cpu_id as u64);

    // 5. Memory barrier and publish online state
    core::arch::asm!("dsb ish; isb");
    CPU_ONLINE[cpu_id].store(true, Ordering::SeqCst);
    CPU_ONLINE_COUNT.fetch_add(1, Ordering::SeqCst);

    let mpidr = info.mpidr;
    serial_println!(
        "[SMP] CPU #{} (MPIDR {:#x}) brought online successfully.",
        cpu_id,
        mpidr
    );

    // Switch to dedicated CPU boot stack before entering idle loop (configure SP_EL1 and SP_EL0)
    let stack_top = (CPU_BOOT_STACKS[cpu_id].0.as_mut_ptr() as usize) + CPU_STACK_SIZE;
    core::arch::asm!(
        "msr spsel, #1",
        "mov sp, {0}",
        "msr sp_el0, {0}",
        "isb",
        in(reg) stack_top
    );

    // 6. Enter secondary scheduling loop
    secondary_cpu_loop(cpu_id);
}

/// Secondary CPU execution and idle loop
pub unsafe fn secondary_cpu_loop(_cpu_id: usize) -> ! {
    // Park secondary core until BSP finishes full kernel & process 1 initialization
    while !SMP_SCHEDULER_READY.load(Ordering::Acquire) {
        core::arch::asm!("wfe");
    }

    crate::sys::process::cpu_idle_entry();
}

