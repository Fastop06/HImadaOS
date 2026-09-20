use crate::fs::vfs::{FileOps, NodeKind, VfsNode};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

fn read_str_offset(s: &str, offset: usize, buf: &mut [u8]) -> usize {
    let bytes = s.as_bytes();
    if offset >= bytes.len() {
        return 0;
    }
    let to_copy = (bytes.len() - offset).min(buf.len());
    buf[..to_copy].copy_from_slice(&bytes[offset..offset + to_copy]);
    to_copy
}

struct MeminfoOps;
impl FileOps for MeminfoOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let (total, free) = crate::mm::pmm::get_memory_stats();
        let total_kb = total / 1024;
        let free_kb = free / 1024;
        let avail_kb = free_kb;
        let buffers_kb = 1024;
        let cached_kb = 8192;
        let s = format!(
            "MemTotal:       {:8} kB\nMemFree:        {:8} kB\nMemAvailable:   {:8} kB\nBuffers:        {:8} kB\nCached:         {:8} kB\nSwapTotal:             0 kB\nSwapFree:              0 kB\n",
            total_kb, free_kb, avail_kb, buffers_kb, cached_kb
        );
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct UptimeOps;
impl FileOps for UptimeOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let mut count: u64 = 0;
        let mut freq: u64 = 0;
        unsafe {
            core::arch::asm!("mrs {0}, cntvct_el0", out(reg) count);
            core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) freq);
        }
        let secs = if freq > 0 { count / freq } else { 0 };
        let rem = if freq > 0 { ((count % freq) * 100) / freq } else { 0 };
        let s = format!("{}.{:02} {}.{:02}\n", secs, rem, secs, rem);
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct LoadavgOps;
impl FileOps for LoadavgOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let pm = crate::sys::process::PROCESS_MANAGER.lock();
        let active = pm.procs.iter().flatten().filter(|p| {
            p.state == crate::sys::process::ProcessState::Running
                || p.state == crate::sys::process::ProcessState::Ready
        }).count();
        let total = pm.procs.iter().flatten().count();
        let cur_pid = pm.current_pid;
        let s = format!("0.02 0.01 0.00 {}/{} {}\n", active, total, cur_pid);
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct CpuinfoOps;
impl FileOps for CpuinfoOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let cpu_count = crate::hal::smp::get_cpu_count().max(1);
        let mut s = String::new();
        for i in 0..cpu_count {
            s.push_str(&format!(
                "processor\t: {}\nBogoMIPS\t: 48.00\nFeatures\t: fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp cpuid\nCPU implementer\t: 0x41\nCPU architecture: 8\nCPU variant\t: 0x0\nCPU part\t: 0xd08\nCPU revision\t: 2\n\n",
                i
            ));
        }
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}


struct VersionOps;
impl FileOps for VersionOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let s = "Linux version 6.8.0-himada-server (root@himada-build) (gcc 13.2.0, GNU ld 2.42) #1 SMP PREEMPT 2026\n";
        read_str_offset(s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct MountsOps;
impl FileOps for MountsOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let mut s = alloc::string::String::from(
            "/dev/root / ext4 rw,relatime 0 0\n\
             proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0\n\
             sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0\n\
             devtmpfs /dev devtmpfs rw,nosuid,size=256m,nr_inodes=65536,mode=755 0 0\n"
        );
        for mp in crate::fs::vfs::MOUNT_POINTS.read().iter() {
            s.push_str(&alloc::format!("{} {} {} rw,relatime 0 0\n", mp.source, mp.target, mp.fstype));
        }
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct StatOps;
impl FileOps for StatOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let s = "cpu  1200 0 450 18500 100 0 10 0 0 0\ncpu0 1200 0 450 18500 100 0 10 0 0 0\nintr 4500\nctxt 12000\nbtime 1789312000\nprocesses 25\nprocs_running 1\nprocs_blocked 0\n";
        read_str_offset(s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct NetDevOps;
impl FileOps for NetDevOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let s = "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n    lo:    2048      16    0    0    0     0          0         0     2048      16    0    0    0     0       0          0\n  eth0:    4096      32    0    0    0     0          0         0     4096      32    0    0    0     0       0          0\n";
        read_str_offset(s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct ProcPidCmdlineOps(usize);
impl FileOps for ProcPidCmdlineOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let pm = crate::sys::process::PROCESS_MANAGER.lock();
        let name = if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == self.0) {
            alloc::string::String::from(p.get_name())
        } else {
            alloc::string::String::from("unknown")
        };
        drop(pm);
        let s = format!("{}\0", name);
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct ProcPidStatusOps(usize);
impl FileOps for ProcPidStatusOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let pm = crate::sys::process::PROCESS_MANAGER.lock();
        let (name, state_char, state_desc, ppid, uid, gid) = if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == self.0) {
            let (sc, sd) = match p.state {
                crate::sys::process::ProcessState::Running => ("R", "running"),
                crate::sys::process::ProcessState::Ready => ("R", "running"),
                crate::sys::process::ProcessState::Blocked => ("S", "sleeping"),
                crate::sys::process::ProcessState::Zombie => ("Z", "zombie"),
                crate::sys::process::ProcessState::Unused => ("X", "dead"),
            };
            (alloc::string::String::from(p.get_name()), sc, sd, p.ppid, p.uid, p.gid)
        } else {
            (alloc::string::String::from("unknown"), "X", "dead", 0, 0, 0)
        };
        drop(pm);
        let s = format!(
            "Name:\t{}\nState:\t{} ({})\nTgid:\t{}\nPid:\t{}\nPPid:\t{}\nUid:\t{}\t{}\t{}\t{}\nGid:\t{}\t{}\t{}\t{}\nThreads:\t1\n",
            name, state_char, state_desc, self.0, self.0, ppid, uid, uid, uid, uid, gid, gid, gid, gid
        );
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

struct ProcPidStatOps(usize);
impl FileOps for ProcPidStatOps {
    fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        let pm = crate::sys::process::PROCESS_MANAGER.lock();
        let (name, state_char, ppid) = if let Some(p) = pm.procs.iter().flatten().find(|p| p.pid == self.0) {
            let sc = match p.state {
                crate::sys::process::ProcessState::Running | crate::sys::process::ProcessState::Ready => "R",
                crate::sys::process::ProcessState::Blocked => "S",
                crate::sys::process::ProcessState::Zombie => "Z",
                crate::sys::process::ProcessState::Unused => "X",
            };
            (alloc::string::String::from(p.get_name()), sc, p.ppid)
        } else {
            (alloc::string::String::from("unknown"), "X", 0)
        };
        drop(pm);
        let s = format!(
            "{} ({}) {} {} {} {} 0 -1 4194304 100 0 0 0 0 0 0 0 20 0 1 0 1000 5242880 256 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0\n",
            self.0, name, state_char, ppid, self.0, self.0
        );
        read_str_offset(&s, offset, buf)
    }
    fn write(&self, _offset: usize, _buf: &[u8]) -> usize { 0 }
}

pub fn lookup_proc(parts: &[&str]) -> Option<Arc<VfsNode>> {
    if parts.is_empty() {
        return None;
    }

    let target_pid = if parts[0] == "self" {
        let pm = crate::sys::process::PROCESS_MANAGER.lock();
        pm.current_pid
    } else if let Ok(pid) = parts[0].parse::<usize>() {
        pid
    } else {
        return None;
    };

    if parts.len() == 1 {
        // Directory /proc/<pid> or /proc/self
        return Some(Arc::new(VfsNode {
            name: parts[0].to_string(),
            kind: NodeKind::Directory,
            size: 0,
            children: RwLock::new(Vec::new()),
            file_ops: None,
            data: RwLock::new(Vec::new()),
            static_data: None,
            uid: RwLock::new(0),
            gid: RwLock::new(0),
            mode: RwLock::new(0o555),
        }));
    }

    let subfile = parts[1];
    let ops: Arc<dyn FileOps> = match subfile {
        "cmdline" => Arc::new(ProcPidCmdlineOps(target_pid)),
        "status" => Arc::new(ProcPidStatusOps(target_pid)),
        "stat" => Arc::new(ProcPidStatOps(target_pid)),
        _ => return None,
    };

    Some(Arc::new(VfsNode {
        name: subfile.to_string(),
        kind: NodeKind::File,
        size: 0,
        children: RwLock::new(Vec::new()),
        file_ops: Some(ops),
        data: RwLock::new(Vec::new()),
        static_data: None,
        uid: RwLock::new(0),
        gid: RwLock::new(0),
        mode: RwLock::new(0o444),
    }))
}

pub fn init() {
    fn make_proc_node(name: &str, ops: Arc<dyn FileOps>) -> Arc<VfsNode> {
        Arc::new(VfsNode {
            name: name.to_string(),
            kind: NodeKind::File,
            size: 0,
            children: RwLock::new(Vec::new()),
            file_ops: Some(ops),
            data: RwLock::new(Vec::new()),
            static_data: None,
            uid: RwLock::new(0),
            gid: RwLock::new(0),
            mode: RwLock::new(0o444),
        })
    }

    crate::fs::vfs::add_node("proc/meminfo", make_proc_node("meminfo", Arc::new(MeminfoOps)));
    crate::fs::vfs::add_node("proc/uptime", make_proc_node("uptime", Arc::new(UptimeOps)));
    crate::fs::vfs::add_node("proc/loadavg", make_proc_node("loadavg", Arc::new(LoadavgOps)));
    crate::fs::vfs::add_node("proc/cpuinfo", make_proc_node("cpuinfo", Arc::new(CpuinfoOps)));
    crate::fs::vfs::add_node("proc/version", make_proc_node("version", Arc::new(VersionOps)));
    crate::fs::vfs::add_node("proc/mounts", make_proc_node("mounts", Arc::new(MountsOps)));
    crate::fs::vfs::add_node("proc/stat", make_proc_node("stat", Arc::new(StatOps)));
    crate::fs::vfs::add_node("proc/net/dev", make_proc_node("dev", Arc::new(NetDevOps)));
}
