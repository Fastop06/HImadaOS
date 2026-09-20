//! Fast Userspace Mutex (Futex) Subsystem
//!
//! Provides the core Linux futex primitives (FUTEX_WAIT, FUTEX_WAKE,
//! FUTEX_REQUEUE, FUTEX_CMP_REQUEUE) required for POSIX pthreads, Rust std::sync::Mutex,
//! Go runtime goroutines, and Musl C library mutexes.

use spin::Mutex;
use crate::serial_println;

pub const FUTEX_WAIT: i32 = 0;
pub const FUTEX_WAKE: i32 = 1;
pub const FUTEX_FD: i32 = 2;
pub const FUTEX_REQUEUE: i32 = 3;
pub const FUTEX_CMP_REQUEUE: i32 = 4;
pub const FUTEX_WAKE_OP: i32 = 5;
pub const FUTEX_LOCK_PI: i32 = 6;
pub const FUTEX_UNLOCK_PI: i32 = 7;
pub const FUTEX_TRYLOCK_PI: i32 = 8;
pub const FUTEX_WAIT_BITSET: i32 = 9;
pub const FUTEX_WAKE_BITSET: i32 = 10;

pub const FUTEX_PRIVATE_FLAG: i32 = 128;
pub const FUTEX_CLOCK_REALTIME: i32 = 256;
pub const FUTEX_CMD_MASK: i32 = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FutexWaiter {
    pub uaddr: u64,
    pub tid: usize,
    pub bitset: u32,
}

pub const MAX_FUTEX_WAITERS: usize = 64;

pub struct FutexManager {
    pub waiters: [Option<FutexWaiter>; MAX_FUTEX_WAITERS],
}

impl FutexManager {
    pub const fn new() -> Self {
        const EMPTY: Option<FutexWaiter> = None;
        Self {
            waiters: [EMPTY; MAX_FUTEX_WAITERS],
        }
    }

    pub fn add_waiter(&mut self, uaddr: u64, tid: usize, bitset: u32) -> bool {
        for slot in &mut self.waiters {
            if slot.is_none() {
                *slot = Some(FutexWaiter { uaddr, tid, bitset });
                return true;
            }
        }
        false
    }

    pub fn remove_waiter(&mut self, tid: usize) {
        for slot in &mut self.waiters {
            if let Some(w) = slot {
                if w.tid == tid {
                    *slot = None;
                }
            }
        }
    }

    pub fn is_waiting(&self, tid: usize, uaddr: u64) -> bool {
        self.waiters.iter().any(|w| {
            if let Some(waiter) = w {
                waiter.tid == tid && waiter.uaddr == uaddr
            } else {
                false
            }
        })
    }

    pub fn wake(&mut self, uaddr: u64, count: u32, bitset: u32) -> (u32, [usize; MAX_FUTEX_WAITERS]) {
        let mut woken = 0u32;
        let mut woken_tids = [0usize; MAX_FUTEX_WAITERS];

        for slot in &mut self.waiters {
            if woken >= count {
                break;
            }
            if let Some(w) = slot {
                if w.uaddr == uaddr && (w.bitset & bitset) != 0 {
                    woken_tids[woken as usize] = w.tid;
                    woken += 1;
                    *slot = None;
                }
            }
        }
        (woken, woken_tids)
    }

    pub fn requeue(
        &mut self,
        uaddr: u64,
        wake_count: u32,
        uaddr2: u64,
        requeue_count: u32,
    ) -> (u32, [usize; MAX_FUTEX_WAITERS]) {
        let mut woken = 0u32;
        let mut woken_tids = [0usize; MAX_FUTEX_WAITERS];
        let mut requeued = 0u32;

        for slot in &mut self.waiters {
            if let Some(ref mut w) = slot {
                if w.uaddr == uaddr {
                    if woken < wake_count {
                        woken_tids[woken as usize] = w.tid;
                        woken += 1;
                        *slot = None;
                    } else if requeued < requeue_count {
                        w.uaddr = uaddr2;
                        requeued += 1;
                    }
                }
            }
        }
        (woken, woken_tids)
    }
}

pub static FUTEX_MANAGER: Mutex<FutexManager> = Mutex::new(FutexManager::new());

/// Put current thread to sleep until condition changes or it is woken up
pub fn futex_wait(uaddr: u64, val: u32, _timeout_ptr: u64, bitset: u32) -> u64 {
    if uaddr == 0 || uaddr % 4 != 0 {
        return (-22i64) as u64; // -EINVAL (unaligned or null pointer)
    }

    // Safely check current atomic value in userspace memory
    let cur_val = unsafe { *(uaddr as *const u32) };
    if cur_val != val {
        // Value changed before entering wait: immediately return -EAGAIN
        return (-11i64) as u64; // -EAGAIN
    }

    let cpu = crate::hal::smp::current_cpu_id();
    let cur_tid = {
        let pm = crate::sys::process::PROCESS_MANAGER.lock();
        pm.current_pids[cpu]
    };

    // Enqueue waiter into futex manager
    {
        let mut fm = FUTEX_MANAGER.lock();
        if !fm.add_waiter(uaddr, cur_tid, bitset) {
            return (-11i64) as u64; // Queue overflow
        }
    }

    // Mark current thread as Blocked
    {
        let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
        if let Some(proc) = pm.get_process_mut(cur_tid) {
            proc.state = crate::sys::process::ProcessState::Blocked;
            proc.running_cpu = None;
        }
    }

    // Yield CPU to allow other threads / processes to execute
    crate::sys::process::schedule();

    // After waking, ensure cleanup
    let mut fm = FUTEX_MANAGER.lock();
    fm.remove_waiter(cur_tid);

    0 // Woken successfully
}

/// Wake up to `count` waiting threads registered on `uaddr`
pub fn futex_wake(uaddr: u64, count: u32, bitset: u32) -> u64 {
    if uaddr == 0 || count == 0 {
        return 0;
    }

    let (woken_count, woken_tids) = {
        let mut fm = FUTEX_MANAGER.lock();
        fm.wake(uaddr, count, bitset)
    };

    if woken_count > 0 {
        let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
        for i in 0..woken_count as usize {
            let tid = woken_tids[i];
            if let Some(proc) = pm.get_process_mut(tid) {
                if proc.state == crate::sys::process::ProcessState::Blocked {
                    proc.state = crate::sys::process::ProcessState::Ready;
                }
            }
        }
        unsafe {
            core::arch::asm!("sev");
        }
    }

    woken_count as u64
}

/// Linux sys_futex entrypoint dispatcher
pub fn sys_futex(
    uaddr: u64,
    op: i32,
    val: u32,
    timeout_ptr: u64,
    uaddr2: u64,
    val3: u32,
) -> u64 {
    let cmd = op & FUTEX_CMD_MASK;
    match cmd {
        FUTEX_WAIT => futex_wait(uaddr, val, timeout_ptr, 0xFFFF_FFFF),
        FUTEX_WAIT_BITSET => futex_wait(uaddr, val, timeout_ptr, val3),
        FUTEX_WAKE => futex_wake(uaddr, val, 0xFFFF_FFFF),
        FUTEX_WAKE_BITSET => futex_wake(uaddr, val, val3),
        FUTEX_REQUEUE => {
            let wake_count = val;
            let req_count = timeout_ptr as u32;
            let (woken_count, woken_tids) = {
                let mut fm = FUTEX_MANAGER.lock();
                fm.requeue(uaddr, wake_count, uaddr2, req_count)
            };
            if woken_count > 0 {
                let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
                for i in 0..woken_count as usize {
                    let tid = woken_tids[i];
                    if let Some(proc) = pm.get_process_mut(tid) {
                        if proc.state == crate::sys::process::ProcessState::Blocked {
                            proc.state = crate::sys::process::ProcessState::Ready;
                        }
                    }
                }
                unsafe {
                    core::arch::asm!("sev");
                }
            }
            woken_count as u64
        }
        FUTEX_CMP_REQUEUE => {
            if uaddr == 0 || uaddr % 4 != 0 {
                return (-22i64) as u64; // -EINVAL
            }
            let cur_val = unsafe { *(uaddr as *const u32) };
            if cur_val != val3 {
                return (-11i64) as u64; // -EAGAIN
            }
            let wake_count = val;
            let req_count = timeout_ptr as u32;
            let (woken_count, woken_tids) = {
                let mut fm = FUTEX_MANAGER.lock();
                fm.requeue(uaddr, wake_count, uaddr2, req_count)
            };
            if woken_count > 0 {
                let mut pm = crate::sys::process::PROCESS_MANAGER.lock();
                for i in 0..woken_count as usize {
                    let tid = woken_tids[i];
                    if let Some(proc) = pm.get_process_mut(tid) {
                        if proc.state == crate::sys::process::ProcessState::Blocked {
                            proc.state = crate::sys::process::ProcessState::Ready;
                        }
                    }
                }
                unsafe {
                    core::arch::asm!("sev");
                }
            }
            woken_count as u64
        }
        _ => {
            serial_println!("[Futex] Unsupported futex op: {:#x}", op);
            0
        }
    }
}
