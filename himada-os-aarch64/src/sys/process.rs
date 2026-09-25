use alloc::string::String;
use alloc::sync::Arc;
use core::arch::global_asm;
use spin::Mutex;
use crate::serial_println;
use crate::sys::linux_abi::FileDescriptor;

pub const MAX_PROCS: usize = 64;
pub const KSTACK_SIZE: usize = 65536; // 64 KiB kernel stack per process

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuContext {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64, // Frame Pointer
    pub x30: u64, // Link Register (Return address)
    pub sp: u64,  // Kernel Stack Pointer (SP_EL1)
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FpuContext {
    pub q: [u128; 32], // 32 x 128-bit NEON registers (Q0..Q31)
    pub fpsr: u32,
    pub fpcr: u32,
    pub _pad: u64,
}

impl Default for FpuContext {
    fn default() -> Self {
        Self {
            q: [0u128; 32],
            fpsr: 0,
            fpcr: 0,
            _pad: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Unused,
    Ready,
    Running,
    Blocked,
    Zombie,
}

pub struct Process {
    pub pid: usize,          // Thread ID (TID)
    pub tgid: usize,         // Thread Group ID (Process ID PID)
    pub ppid: usize,         // Parent PID
    pub state: ProcessState,
    pub exit_code: i32,
    pub ttbr0: usize,
    pub is_thread: bool,     // True if sharing address space (POSIX thread)
    pub clear_child_tid: u64,// CLONE_CHILD_CLEARTID address for futex wake on thread exit
    pub tls: u64,            // User TLS base register (TPIDR_EL0)
    pub running_cpu: core::sync::atomic::AtomicI32, // -1 = None, 0..MAX_CPUS = active CPU core
    pub kstack: usize,
    pub kstack_top: usize,
    pub ustack_top: usize,
    pub entry_point: usize,
    pub cpu_context: CpuContext,
    pub fpu_context: FpuContext,
    pub fd_table: [Option<FileDescriptor>; 64],
    pub cwd: String,
    pub name: [u8; 32],
    pub uid: u32,
    pub gid: u32,
    pub euid: u32,
    pub egid: u32,
    pub brk: usize,
}

impl Process {
    pub fn new(pid: usize, ppid: usize, ttbr0: usize, name_str: &str) -> Self {
        // Allocate a dedicated kernel stack (16 pages = 64 KiB)
        let kstack_pages = KSTACK_SIZE / 4096;
        let kstack_phys = crate::mm::pmm::alloc_frames(kstack_pages).expect("OOM for process kernel stack");
        let kstack_virt = unsafe { crate::mm::vmm::phys_to_virt(kstack_phys) };
        unsafe {
            crate::sysmem::page_zero(kstack_virt as *mut u8, KSTACK_SIZE);
        }
        let kstack_top = kstack_virt + KSTACK_SIZE;

        let mut name = [0u8; 32];
        let bytes = name_str.as_bytes();
        let copy_len = bytes.len().min(31);
        name[..copy_len].copy_from_slice(&bytes[..copy_len]);

        const INIT_FD: Option<FileDescriptor> = None;
        let fd_table = [INIT_FD; 64];

        let mut proc = Self {
            pid,
            tgid: pid,
            ppid,
            state: ProcessState::Ready,
            exit_code: 0,
            ttbr0,
            is_thread: false,
            clear_child_tid: 0,
            tls: 0,
            running_cpu: core::sync::atomic::AtomicI32::new(-1),
            kstack: kstack_virt,
            kstack_top,
            ustack_top: 0,
            entry_point: 0,
            cpu_context: CpuContext::default(),
            fpu_context: FpuContext::default(),
            fd_table,
            cwd: String::from("/"),
            name,
            uid: 0,
            gid: 0,
            euid: 0,
            egid: 0,
            brk: 0x4000_0000_0000,
        };

        // Set default SP to top of kernel stack
        proc.cpu_context.sp = kstack_top as u64;
        READY_TASKS_COUNT.fetch_add(1, core::sync::atomic::Ordering::Release);
        unsafe { core::arch::asm!("sev"); }
        proc
    }

    pub fn get_name(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(self.name.len());
        core::str::from_utf8(&self.name[..len]).unwrap_or("unknown")
    }
}

pub struct ProcessManager {
    pub procs: [Option<Process>; MAX_PROCS],
    pub current_pid: usize,
    pub current_pids: [usize; crate::hal::smp::MAX_CPUS],
    pub next_pid: usize,
}

impl ProcessManager {
    pub const fn new() -> Self {
        const EMPTY_PROC: Option<Process> = None;
        Self {
            procs: [EMPTY_PROC; MAX_PROCS],
            current_pid: 0,
            current_pids: [0; crate::hal::smp::MAX_CPUS],
            next_pid: 1,
        }
    }

    pub fn allocate_pid(&mut self) -> Option<usize> {
        for _ in 0..MAX_PROCS {
            let pid = self.next_pid;
            self.next_pid += 1;
            if self.next_pid >= 65536 {
                self.next_pid = 1;
            }
            if !self.procs.iter().any(|p| p.as_ref().map_or(false, |proc| proc.pid == pid)) {
                return Some(pid);
            }
        }
        None
    }

    pub fn find_free_slot(&self) -> Option<usize> {
        self.procs.iter().position(|p| p.is_none() || p.as_ref().unwrap().state == ProcessState::Unused)
    }

    pub fn get_current_mut(&mut self) -> Option<&mut Process> {
        let cpu = crate::hal::smp::current_cpu_id();
        let cur = self.current_pids[cpu];
        let target_pid = if cur != 0 { cur } else { self.current_pid };
        self.procs.iter_mut().filter_map(|p| p.as_mut()).find(|p| p.pid == target_pid)
    }

    pub fn get_process_mut(&mut self, pid: usize) -> Option<&mut Process> {
        self.procs.iter_mut().filter_map(|p| p.as_mut()).find(|p| p.pid == pid)
    }
}


pub static PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());

pub static mut PER_CPU_IDLE_CONTEXT: [CpuContext; crate::hal::smp::MAX_CPUS] = [
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
    CpuContext { x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0 },
];

pub static READY_TASKS_COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn cpu_idle_entry() -> ! {
    let cpu = crate::hal::smp::current_cpu_id();
    loop {
        // Only CPU 0 polls network during idle to avoid bus contention across cores
        if cpu == 0 {
            crate::net::socket::poll();
            schedule();
            for _ in 0..10_000 {
                core::hint::spin_loop();
            }
        } else {
            // Secondary cores: do not touch scheduler lock unless ready tasks exist
            if READY_TASKS_COUNT.load(core::sync::atomic::Ordering::Acquire) > 0 {
                schedule();
            } else {
                unsafe {
                    core::arch::asm!("wfe");
                }
            }
        }
    }
}

pub fn init_idle_contexts() {
    for cpu in 0..crate::hal::smp::MAX_CPUS {
        let stack_top = unsafe {
            (crate::hal::smp::CPU_BOOT_STACKS[cpu].0.as_mut_ptr() as usize) + crate::hal::smp::CPU_STACK_SIZE
        };
        unsafe {
            PER_CPU_IDLE_CONTEXT[cpu].sp = stack_top as u64;
            PER_CPU_IDLE_CONTEXT[cpu].x30 = cpu_idle_entry as *const () as usize as u64;
        }
    }
}


// ─────────────────────────────────────────────────────────────
// Assembly Context Switch & FPU Guard Routines
// ─────────────────────────────────────────────────────────────
global_asm!(
r#"
.global cpu_switch_to
cpu_switch_to:
    // x0: *mut CpuContext (prev)
    // x1: *const CpuContext (next)
    // x2: *mut i32 (prev_running_cpu flag, or NULL)

    // 1. Save callee-saved registers of prev
    stp x19, x20, [x0, #0]
    stp x21, x22, [x0, #16]
    stp x23, x24, [x0, #32]
    stp x25, x26, [x0, #48]
    stp x27, x28, [x0, #64]
    stp x29, x30, [x0, #80]
    mov x3, sp
    str x3, [x0, #96]

    // 2. Restore callee-saved registers of next
    ldp x19, x20, [x1, #0]
    ldp x21, x22, [x1, #16]
    ldp x23, x24, [x1, #32]
    ldp x25, x26, [x1, #48]
    ldp x27, x28, [x1, #64]
    ldp x29, x30, [x1, #80]
    ldr x3, [x1, #96]
    mov sp, x3

    // 3. Atomically release prev_running_cpu ONLY after prev's context and stack are fully saved
    dmb ish
    cbz x2, 1f
    mov w4, #-1
    str w4, [x2]
    dmb ish
1:
    // Return to next.x30
    ret

.global save_fpu_context
save_fpu_context:
    // x0: *mut FpuContext (points to [u128; 32] + fpsr, fpcr)
    stp q0, q1, [x0, #0]
    stp q2, q3, [x0, #32]
    stp q4, q5, [x0, #64]
    stp q6, q7, [x0, #96]
    stp q8, q9, [x0, #128]
    stp q10, q11, [x0, #160]
    stp q12, q13, [x0, #192]
    stp q14, q15, [x0, #224]
    stp q16, q17, [x0, #256]
    stp q18, q19, [x0, #288]
    stp q20, q21, [x0, #320]
    stp q22, q23, [x0, #352]
    stp q24, q25, [x0, #384]
    stp q26, q27, [x0, #416]
    stp q28, q29, [x0, #448]
    stp q30, q31, [x0, #480]
    mrs x1, fpsr
    mrs x2, fpcr
    add x3, x0, #512
    stp w1, w2, [x3, #0]
    ret

.global restore_fpu_context
restore_fpu_context:
    // x0: *const FpuContext
    ldp q0, q1, [x0, #0]
    ldp q2, q3, [x0, #32]
    ldp q4, q5, [x0, #64]
    ldp q6, q7, [x0, #96]
    ldp q8, q9, [x0, #128]
    ldp q10, q11, [x0, #160]
    ldp q12, q13, [x0, #192]
    ldp q14, q15, [x0, #224]
    ldp q16, q17, [x0, #256]
    ldp q18, q19, [x0, #288]
    ldp q20, q21, [x0, #320]
    ldp q22, q23, [x0, #352]
    ldp q24, q25, [x0, #384]
    ldp q26, q27, [x0, #416]
    ldp q28, q29, [x0, #448]
    ldp q30, q31, [x0, #480]
    add x3, x0, #512
    ldp w1, w2, [x3, #0]
    msr fpsr, x1
    msr fpcr, x2
    ret

.global return_from_fork_trampoline
return_from_fork_trampoline:
    // When a newly forked child starts execution on cpu_switch_to,
    // its kernel stack already contains an ExceptionContext set up by fork.
    // We restore context using the macro and eret into userspace EL0!
    // ExceptionContext is at current sp.
    ldp x0, x1, [sp, #248]
    ldr x2, [sp, #264]
    msr elr_el1, x0
    msr spsr_el1, x1
    msr sp_el0, x2

    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
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
    eret
"#
);

extern "C" {
    pub fn cpu_switch_to(prev: *mut CpuContext, next: *const CpuContext, prev_rcpu: *mut i32);
    pub fn save_fpu_context(ctx: *mut FpuContext);
    pub fn restore_fpu_context(ctx: *const FpuContext);
    pub fn return_from_fork_trampoline();
}

/// Round-Robin Multi-Core Scheduler
pub fn schedule() {
    let cpu = crate::hal::smp::current_cpu_id();
    let mut pm = PROCESS_MANAGER.lock();
    let cur_pid = pm.current_pids[cpu];

    // Find current index on this CPU
    let cur_idx = if cur_pid != 0 {
        pm.procs.iter().position(|p| p.as_ref().map_or(false, |proc| proc.pid == cur_pid))
    } else {
        None
    };

    // Search for next Ready process / thread that is not active on another CPU
    let mut next_idx = None;
    let start = cur_idx.map_or(0, |i| i + 1);

    for offset in 0..MAX_PROCS {
        let idx = (start + offset) % MAX_PROCS;
        if let Some(ref p) = pm.procs[idx] {
            let rcpu = p.running_cpu.load(core::sync::atomic::Ordering::Acquire);
            if p.state == ProcessState::Ready && (rcpu == -1 || rcpu == cpu as i32) {
                next_idx = Some(idx);
                break;
            }
        }
    }

    if next_idx.is_none() {
        if let Some(c_idx) = cur_idx {
            let cur_proc = pm.procs[c_idx].as_ref().unwrap();
            if cur_proc.state == ProcessState::Running {
                return; // Only process running on this core and still running
            }
            // Current task is Zombie or Blocked, and nothing else is ready
            pm.current_pids[cpu] = 0;
            if cpu == 0 {
                pm.current_pid = 0;
            }
            let prev_proc = pm.procs[c_idx].as_mut().unwrap();
            let prev_ctx_ptr = &mut prev_proc.cpu_context as *mut CpuContext;
            let prev_rcpu_ptr = &prev_proc.running_cpu as *const core::sync::atomic::AtomicI32 as *mut i32;
            drop(pm);
            unsafe {
                cpu_switch_to(prev_ctx_ptr, &PER_CPU_IDLE_CONTEXT[cpu] as *const CpuContext, prev_rcpu_ptr);
            }
            return;
        }
        return;
    }

    let next_idx = next_idx.unwrap();

    if Some(next_idx) == cur_idx {
        return; // Current process is the only ready process
    }

    // Prepare context pointers
    let (prev_ctx_ptr, prev_fpu_ptr, prev_rcpu_ptr) = if let Some(c_idx) = cur_idx {
        let prev_proc = pm.procs[c_idx].as_mut().unwrap();
        if prev_proc.state == ProcessState::Running {
            prev_proc.state = ProcessState::Ready;
            READY_TASKS_COUNT.fetch_add(1, core::sync::atomic::Ordering::Release);
            unsafe { core::arch::asm!("sev"); }
            // NOTE: Do NOT set prev_proc.running_cpu here! It remains `cpu as i32`
            // until cpu_switch_to completes saving and switching stack on this core.
        }
        prev_proc.fd_table = crate::sys::linux_abi::get_fd_table();
        (
            &mut prev_proc.cpu_context as *mut CpuContext,
            &mut prev_proc.fpu_context as *mut FpuContext,
            &prev_proc.running_cpu as *const core::sync::atomic::AtomicI32 as *mut i32,
        )
    } else {
        (
            unsafe { &mut PER_CPU_IDLE_CONTEXT[cpu] as *mut CpuContext },
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        )
    };

    let next_proc = pm.procs[next_idx].as_mut().unwrap();
    next_proc.state = ProcessState::Running;
    READY_TASKS_COUNT.fetch_sub(1, core::sync::atomic::Ordering::Release);
    next_proc.running_cpu.store(cpu as i32, core::sync::atomic::Ordering::Release);
    let next_pid = next_proc.pid;
    let next_ttbr0 = next_proc.ttbr0;
    let next_tls = next_proc.tls;

    // Safety guard: ensure next process has a valid return address
    if next_proc.cpu_context.x30 == 0 {
        next_proc.cpu_context.x30 = return_from_fork_trampoline as *const () as usize as u64;
    }

    let next_ctx_ptr = &next_proc.cpu_context as *const CpuContext;
    let next_fpu_ptr = &next_proc.fpu_context as *const FpuContext;
    let next_fd_table = next_proc.fd_table.clone();

    pm.current_pids[cpu] = next_pid;
    if cpu == 0 {
        pm.current_pid = next_pid;
    }

    // Drop lock before switching context
    drop(pm);

    // Restore FD table for next process
    crate::sys::linux_abi::set_fd_table(&next_fd_table);

    // Save and switch FPU context (Himada SIMD safety)
    unsafe {
        if !prev_fpu_ptr.is_null() {
            save_fpu_context(prev_fpu_ptr);
        }
        restore_fpu_context(next_fpu_ptr);

        // Switch address space (TTBR0_EL1)
        if next_ttbr0 != 0 {
            core::arch::asm!(
                "msr ttbr0_el1, {0}",
                "dsb ish",
                "tlbi vmalle1is",
                "dsb ish",
                "isb",
                in(reg) next_ttbr0
            );
        }

        // Restore userspace TLS pointer (TPIDR_EL0) if configured
        if next_tls != 0 {
            core::arch::asm!("msr tpidr_el0, {}", in(reg) next_tls);
        }

        // Switch CPU registers & stack, and atomically release prev
        cpu_switch_to(prev_ctx_ptr, next_ctx_ptr, prev_rcpu_ptr);
    }
}
