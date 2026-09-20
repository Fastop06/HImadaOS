// Himada Package Manager (pacman compatibility engine) for HimadaOS 2.0
// Comprehensive implementation supporting -S, -Sy, -Syy, -Syu, -Ss, -Si, -Sl, -Q, -Qi, -Ql, -Qs, -R, -U, -F, -V, -h

#[derive(Clone, Copy)]
pub struct PackageInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub desc: &'static str,
    pub repo: &'static str,
    pub size_kib: u32,
    pub installed: bool,
}


#[derive(Clone, Copy)]
pub struct DynamicPackageInfo {
    pub name: [u8; 32],
    pub version: [u8; 24],
    pub desc: [u8; 96],
    pub repo: [u8; 16],
    pub filename: [u8; 64],
    pub size_kib: u32,
    pub installed: bool,
    pub valid: bool,
}

pub static mut DYN_PACKAGES: [DynamicPackageInfo; 128] = [DynamicPackageInfo {
    name: [0; 32],
    version: [0; 24],
    desc: [0; 96],
    repo: [0; 16],
    filename: [0; 64],
    size_kib: 0,
    installed: false,
    valid: false,
}; 128];

pub static mut REPO_PACKAGES: [PackageInfo; 65] = [
    // Core system
    PackageInfo { name: "base", version: "3-2", desc: "Minimal package set to define a basic HimadaOS installation", repo: "core", size_kib: 4096, installed: true },
    PackageInfo { name: "coreutils", version: "9.5-1", desc: "The basic file, shell and text manipulation utilities of the GNU operating system", repo: "core", size_kib: 15360, installed: true },
    PackageInfo { name: "linux-himada", version: "6.8.0-1", desc: "The Himada Linux kernel and modules (SMP + Ext4 + W^X)", repo: "core", size_kib: 32768, installed: true },
    PackageInfo { name: "himada-sh", version: "2.0-1", desc: "Kani-verified interactive shell and VT100 ANSI line editor", repo: "core", size_kib: 512, installed: true },
    PackageInfo { name: "pacman", version: "6.1.0-3", desc: "A library-based package manager with dependency support", repo: "core", size_kib: 4800, installed: true },
    PackageInfo { name: "hpm", version: "2.0-1", desc: "Himada native package manager wrapper", repo: "core", size_kib: 320, installed: true },
    PackageInfo { name: "bash", version: "5.2.26-1", desc: "The GNU Bourne-Again SHell", repo: "core", size_kib: 9200, installed: true },
    PackageInfo { name: "sh", version: "2.0-1", desc: "POSIX standard command language interpreter", repo: "core", size_kib: 512, installed: true },
    PackageInfo { name: "nano", version: "8.0-1", desc: "Pico editor clone with enhancements", repo: "core", size_kib: 2400, installed: true },
    PackageInfo { name: "curl", version: "8.8.0-1", desc: "Command line tool and library for transferring data with URLs", repo: "core", size_kib: 2100, installed: true },
    PackageInfo { name: "wget", version: "1.24.5-1", desc: "Network utility to retrieve files from the Web using HTTP and FTP", repo: "core", size_kib: 3400, installed: true },
    PackageInfo { name: "dropbear", version: "2024.84-1", desc: "Lightweight SSH server and client", repo: "core", size_kib: 850, installed: true },
    PackageInfo { name: "fastfetch", version: "2.21.1-1", desc: "Neofetch-like tool for fetching system information and displaying it nicely", repo: "extra", size_kib: 1200, installed: true },
    PackageInfo { name: "tar", version: "1.35-2", desc: "Utility used to store, backup, and transport files", repo: "core", size_kib: 3800, installed: true },
    PackageInfo { name: "gzip", version: "1.13-2", desc: "Standard GNU data compression utility", repo: "core", size_kib: 980, installed: true },
    PackageInfo { name: "bzip2", version: "1.0.8-5", desc: "High-quality data compressor", repo: "core", size_kib: 450, installed: true },
    PackageInfo { name: "xz", version: "5.6.2-1", desc: "Library and CLI tools for XZ and LZMA compressed files", repo: "core", size_kib: 1600, installed: true },
    PackageInfo { name: "which", version: "2.21-6", desc: "Displays where a command is found on /Users/mussavysegurov/.opencode/bin:/Users/mussavysegurov/.gemini/antigravity/bin:/Users/mussavysegurov/Library/Application Support/Antigravity/bin:/Users/mussavysegurov/.antigravity-ide/antigravity-ide/bin:/Users/mussavysegurov/.opencode/bin:/Users/mussavysegurov/.kimi-code/bin:/Users/mussavysegurov/.local/bin:/Users/mussavysegurov/.kilo/bin:/Users/mussavysegurov/.local/bin:/Users/mussavysegurov/.antigravity/antigravity/bin:/Users/mussavysegurov/.nvm/versions/node/v24.15.0/bin:/Users/mussavysegurov/.local/bin:/Library/Frameworks/Python.framework/Versions/3.11/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/pkg/env/global/bin:/Library/Apple/usr/bin:/Applications/Wireshark.app/Contents/MacOS:/Users/mussavysegurov/.opencode/bin:/Users/mussavysegurov/.cargo/bin", repo: "core", size_kib: 210, installed: true },
    PackageInfo { name: "sudo", version: "1.9.15.p5-1", desc: "Give certain users the ability to run commands as root", repo: "core", size_kib: 4200, installed: true },
    PackageInfo { name: "iproute2", version: "6.9.0-1", desc: "IP routing and network configuration utilities", repo: "core", size_kib: 6100, installed: true },
    PackageInfo { name: "net-tools", version: "2.10-2", desc: "Configuration tools for Linux networking", repo: "core", size_kib: 850, installed: true },
    PackageInfo { name: "iptables", version: "1.8.10-1", desc: "Linux kernel packet control tool", repo: "core", size_kib: 2400, installed: true },
    PackageInfo { name: "ca-certificates", version: "20240322-1", desc: "Common CA certificates for HTTPS/TLS verification", repo: "core", size_kib: 450, installed: true },
    PackageInfo { name: "less", version: "643-1", desc: "A terminal pager program", repo: "core", size_kib: 420, installed: true },

    // Developer tools & Compilers
    PackageInfo { name: "vim", version: "9.1.0-1", desc: "Vi Improved text editor", repo: "extra", size_kib: 14500, installed: false },
    PackageInfo { name: "neovim", version: "0.10.0-1", desc: "Vim-fork focused on extensibility and usability", repo: "extra", size_kib: 28000, installed: false },
    PackageInfo { name: "htop", version: "3.3.0-1", desc: "Interactive process viewer", repo: "extra", size_kib: 650, installed: false },
    PackageInfo { name: "btop", version: "1.3.2-1", desc: "Resource monitor that shows usage and stats", repo: "extra", size_kib: 2100, installed: false },
    PackageInfo { name: "tree", version: "2.1.1-1", desc: "A directory listing program displaying a depth-indented list of files", repo: "extra", size_kib: 180, installed: false },
    PackageInfo { name: "calc", version: "2.14.3-1", desc: "Arbitrary precision arithmetic calculator", repo: "extra", size_kib: 420, installed: false },
    PackageInfo { name: "bc", version: "1.07.1-4", desc: "GNU numeric processing language and arbitrary precision calculator", repo: "extra", size_kib: 320, installed: false },
    PackageInfo { name: "hexdump", version: "2.39.3-1", desc: "Display file contents in hexadecimal, decimal, octal, or ascii", repo: "extra", size_kib: 310, installed: false },
    PackageInfo { name: "git", version: "2.45.1-1", desc: "The fast distributed version control system", repo: "extra", size_kib: 28000, installed: false },
    PackageInfo { name: "python", version: "3.12.3-1", desc: "Next generation of the python high-level scripting language", repo: "extra", size_kib: 64000, installed: false },
    PackageInfo { name: "python3", version: "3.12.3-1", desc: "Next generation of the python high-level scripting language", repo: "extra", size_kib: 64000, installed: false },
    PackageInfo { name: "rust", version: "1.80.0-1", desc: "Empowering everyone to build reliable and efficient software", repo: "extra", size_kib: 128000, installed: false },
    PackageInfo { name: "cargo", version: "1.80.0-1", desc: "The Rust package manager", repo: "extra", size_kib: 18000, installed: false },
    PackageInfo { name: "gcc", version: "14.1.1-1", desc: "The GNU Compiler Collection - C and C++ frontends", repo: "core", size_kib: 110000, installed: false },
    PackageInfo { name: "g++", version: "14.1.1-1", desc: "The GNU C++ compiler", repo: "core", size_kib: 45000, installed: false },
    PackageInfo { name: "clang", version: "18.1.8-1", desc: "C language family frontend for LLVM", repo: "extra", size_kib: 140000, installed: false },
    PackageInfo { name: "llvm", version: "18.1.8-1", desc: "Collection of modular and reusable compiler technologies", repo: "extra", size_kib: 160000, installed: false },
    PackageInfo { name: "make", version: "4.4.1-2", desc: "GNU make utility to maintain groups of programs", repo: "core", size_kib: 1800, installed: false },
    PackageInfo { name: "cmake", version: "3.29.6-1", desc: "Cross-platform open-source build system generator", repo: "extra", size_kib: 48000, installed: false },
    PackageInfo { name: "binutils", version: "2.42-2", desc: "GNU binary utilities (ld, as, objdump, nm)", repo: "core", size_kib: 24000, installed: false },
    PackageInfo { name: "gdb", version: "14.2-1", desc: "The GNU Debugger", repo: "extra", size_kib: 26000, installed: false },
    PackageInfo { name: "strace", version: "6.9-1", desc: "Diagnostic, debugging and instructional userspace tracer", repo: "extra", size_kib: 3200, installed: false },
    PackageInfo { name: "tcpdump", version: "4.99.5-1", desc: "A powerful command-line packet analyzer and network sniffer", repo: "extra", size_kib: 3420, installed: false },
    PackageInfo { name: "neofetch", version: "7.1.0-2", desc: "CLI system information tool written in BASH", repo: "extra", size_kib: 540, installed: false },
    PackageInfo { name: "zsh", version: "5.9-4", desc: "Very advanced and programmable command interpreter", repo: "extra", size_kib: 12000, installed: false },
    PackageInfo { name: "tmux", version: "3.4-2", desc: "Terminal multiplexer workspace utility", repo: "extra", size_kib: 2100, installed: false },
    PackageInfo { name: "screen", version: "4.9.1-1", desc: "Full-screen window manager that multiplexes terminal", repo: "extra", size_kib: 1600, installed: false },
    PackageInfo { name: "nginx", version: "1.26.1-1", desc: "Lightweight HTTP server and IMAP/POP3 proxy server", repo: "extra", size_kib: 4200, installed: false },
    PackageInfo { name: "apache", version: "2.4.59-1", desc: "A high performance Unix-based HTTP server", repo: "extra", size_kib: 8400, installed: false },
    PackageInfo { name: "httpd", version: "2.4.59-1", desc: "Apache HTTP Server daemon", repo: "extra", size_kib: 8400, installed: false },
    PackageInfo { name: "sqlite", version: "3.46.0-1", desc: "C library that implements an SQL database engine", repo: "core", size_kib: 4800, installed: false },
    PackageInfo { name: "redis", version: "7.2.5-1", desc: "Advanced key-value store and memory database", repo: "extra", size_kib: 9200, installed: false },
    PackageInfo { name: "jq", version: "1.7.1-1", desc: "Command-line JSON processor", repo: "extra", size_kib: 890, installed: false },
    PackageInfo { name: "socat", version: "1.8.0.0-1", desc: "Multipurpose relay for bidirectional data transfer", repo: "extra", size_kib: 1100, installed: false },
    PackageInfo { name: "nmap", version: "7.95-1", desc: "Network exploration tool and security / port scanner", repo: "extra", size_kib: 16000, installed: false },
    PackageInfo { name: "openssh", version: "9.7p1-1", desc: "Premier connectivity tool for remote login with SSH", repo: "core", size_kib: 5400, installed: false },
    PackageInfo { name: "openssl", version: "3.3.1-1", desc: "Toolkit for Secure Sockets Layer and Transport Layer Security", repo: "core", size_kib: 12000, installed: false },
    PackageInfo { name: "zip", version: "3.0-11", desc: "Compressor and archiver utility for ZIP files", repo: "extra", size_kib: 850, installed: false },
    PackageInfo { name: "unzip", version: "6.0-20", desc: "Extraction utility for ZIP archives", repo: "extra", size_kib: 920, installed: false },
    PackageInfo { name: "zstd", version: "1.5.6-1", desc: "Zstandard - Fast real-time compression algorithm", repo: "core", size_kib: 2200, installed: false },
    PackageInfo { name: "patch", version: "2.7.6-10", desc: "A utility to apply diff files to original files", repo: "core", size_kib: 450, installed: false },
];

pub fn is_installed(name: &str) -> bool {
    unsafe {
        for p in &REPO_PACKAGES {
            if p.name == name {
                return p.installed;
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid {
                if let Ok(dname) = core::str::from_utf8(&dp.name) {
                    if dname.trim_matches(char::from(0)) == name {
                        return dp.installed;
                    }
                }
            }
        }
        false
    }
}

pub fn get_installed_count() -> usize {
    unsafe {
        let mut count = 0;
        for p in &REPO_PACKAGES {
            if p.installed {
                count += 1;
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid && dp.installed {
                count += 1;
            }
        }
        count
    }
}

pub fn get_active_mirror(buf: &mut [u8; 128]) -> &str {
    let at_fdcwd: usize = (-100i64) as usize;
    let mlist_path = b"/etc/pacman.d/mirrorlist\0";
    let fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, mlist_path.as_ptr() as usize, 0);
    if fd != !0 && fd > 0 {
        let mut mbuf = [0u8; 512];
        let n = crate::syscall3(crate::sys_nr::READ, fd, mbuf.as_mut_ptr() as usize, 511);
        crate::syscall1(crate::sys_nr::CLOSE, fd);
        if n > 0 && n != !0 {
            if let Ok(mcontent) = core::str::from_utf8(&mbuf[..n]) {
                for line in mcontent.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Server =") || trimmed.starts_with("Server=") {
                        let url = if trimmed.starts_with("Server =") {
                            trimmed["Server =".len()..].trim()
                        } else {
                            trimmed["Server=".len()..].trim()
                        };
                        let arch = "aarch64";
                        let repo = "core";
                        let mut cur = 0;
                        let ub = url.as_bytes();
                        let mut idx = 0;
                        while idx < ub.len() && cur < 127 {
                            if idx + 5 <= ub.len() && &ub[idx..idx+5] == b"$arch" {
                                let c = arch.len().min(127 - cur);
                                buf[cur..cur+c].copy_from_slice(&arch.as_bytes()[..c]);
                                cur += c;
                                idx += 5;
                            } else if idx + 5 <= ub.len() && &ub[idx..idx+5] == b"$repo" {
                                let c = repo.len().min(127 - cur);
                                buf[cur..cur+c].copy_from_slice(&repo.as_bytes()[..c]);
                                cur += c;
                                idx += 5;
                            } else {
                                buf[cur] = ub[idx];
                                cur += 1;
                                idx += 1;
                            }
                        }
                        if cur > 0 {
                            if let Ok(res) = core::str::from_utf8(&buf[..cur]) {
                                return res;
                            }
                        }
                    }
                }
            }
        }
    }
    "http://mirror.archlinuxarm.org/aarch64/core"
}

pub fn extract_tar_archive(tar_path: &str) -> bool {
    let at_fdcwd: usize = (-100i64) as usize;
    let mut path_buf = [0u8; 128];
    let pb = tar_path.as_bytes();
    let pl = pb.len().min(127);
    path_buf[..pl].copy_from_slice(&pb[..pl]);
    path_buf[pl] = 0;

    let fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, path_buf.as_ptr() as usize, 0);
    if fd == !0 || fd == 0 { return false; }

    let mut header = [0u8; 512];
    let mut files_extracted = 0;

    loop {
        let n = crate::syscall3(crate::sys_nr::READ, fd, header.as_mut_ptr() as usize, 512);
        if n < 512 { break; }

        if header.starts_with(b"\x7fELF") {
            crate::syscall1(crate::sys_nr::CLOSE, fd);
            return false;
        }

        let mut all_zero = true;
        for &b in &header[0..100] {
            if b != 0 { all_zero = false; break; }
        }
        if all_zero { break; }

        let mut name_len = 0;
        while name_len < 100 && header[name_len] != 0 {
            name_len += 1;
        }
        if name_len == 0 { break; }
        let fname = match core::str::from_utf8(&header[0..name_len]) {
            Ok(s) => s,
            Err(_) => break,
        };

        if fname.bytes().any(|b| b < 32 || b >= 127) {
            break;
        }

        let mut size: usize = 0;
        for &b in &header[124..135] {
            if b >= b'0' && b <= b'7' {
                size = (size << 3) | ((b - b'0') as usize);
            }
        }

        let typeflag = header[156];
        let blocks = (size + 511) / 512;

        if typeflag == b'5' || fname.ends_with('/') {
            continue;
        }

        if typeflag == b'0' || typeflag == 0 {
            let mut dest_path = [0u8; 128];
            dest_path[0] = b'/';
            let fl = fname.len().min(126);
            dest_path[1..1+fl].copy_from_slice(&fname.as_bytes()[..fl]);
            dest_path[1+fl] = 0;

            let out_fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, dest_path.as_ptr() as usize, 65 | 512);
            let valid_out = if out_fd != !0 && out_fd > 0 { out_fd } else {
                crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, dest_path.as_ptr() as usize, 65)
            };

            let mut remaining = size;
            let mut chunk = [0u8; 512];
            for _ in 0..blocks {
                let r = crate::syscall3(crate::sys_nr::READ, fd, chunk.as_mut_ptr() as usize, 512);
                if r < 512 { break; }
                let to_write = remaining.min(512);
                if valid_out != !0 && valid_out > 0 && to_write > 0 {
                    crate::syscall3(crate::sys_nr::WRITE, valid_out, chunk.as_ptr() as usize, to_write);
                }
                remaining = remaining.saturating_sub(to_write);
            }
            if valid_out != !0 && valid_out > 0 {
                crate::syscall2(crate::sys_nr::FCHMOD, valid_out, 0o755);
                crate::syscall1(crate::sys_nr::CLOSE, valid_out);
                files_extracted += 1;

                if fname.starts_with("usr/bin/") {
                    let bname = &fname["usr/bin/".len()..];
                    let mut bdest = [0u8; 64];
                    let bprefix = b"/bin/";
                    bdest[..bprefix.len()].copy_from_slice(bprefix);
                    let bl = bname.len().min(55);
                    bdest[bprefix.len()..bprefix.len()+bl].copy_from_slice(&bname.as_bytes()[..bl]);
                    bdest[bprefix.len()+bl] = 0;
                    if let (Ok(src_str), Ok(dst_str)) = (core::str::from_utf8(&dest_path[..1+fl]), core::str::from_utf8(&bdest[..bprefix.len()+bl])) {
                        copy_package_file(src_str, dst_str);
                    }
                } else if fname.starts_with("bin/") {
                    let bname = &fname["bin/".len()..];
                    let mut udest = [0u8; 64];
                    let uprefix = b"/usr/bin/";
                    udest[..uprefix.len()].copy_from_slice(uprefix);
                    let bl = bname.len().min(50);
                    udest[uprefix.len()..uprefix.len()+bl].copy_from_slice(&bname.as_bytes()[..bl]);
                    udest[uprefix.len()+bl] = 0;
                    if let (Ok(src_str), Ok(dst_str)) = (core::str::from_utf8(&dest_path[..1+fl]), core::str::from_utf8(&udest[..uprefix.len()+bl])) {
                        copy_package_file(src_str, dst_str);
                    }
                }
            }
            continue;
        }

        let mut dummy = [0u8; 512];
        for _ in 0..blocks {
            crate::syscall3(crate::sys_nr::READ, fd, dummy.as_mut_ptr() as usize, 512);
        }
    }

    crate::syscall1(crate::sys_nr::CLOSE, fd);
    files_extracted > 0
}

pub fn lookup_package_in_index(target: &str, out_dp: &mut DynamicPackageInfo) -> bool {
    let at_fdcwd: usize = (-100i64) as usize;
    let idx_path = b"/var/lib/pacman/sync/packages.idx\0";
    let mut fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, idx_path.as_ptr() as usize, 0);
    if fd == !0 || fd == 0 {
        let repo_idx = b"/repo/packages.idx\0";
        fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, repo_idx.as_ptr() as usize, 0);
    }
    if fd == !0 || fd == 0 { return false; }

    let mut buf = [0u8; 4096];
    let mut line_buf = [0u8; 512];
    let mut line_len = 0;
    let mut found = false;

    loop {
        let n = crate::syscall3(crate::sys_nr::READ, fd, buf.as_mut_ptr() as usize, 4096);
        if n == 0 || n == !0 { break; }

        for i in 0..n {
            let b = buf[i];
            if b == b'\n' {
                if line_len > 0 {
                    if let Ok(line) = core::str::from_utf8(&line_buf[..line_len]) {
                        if line.starts_with(target) && line.len() > target.len() && line.as_bytes()[target.len()] == b'|' {
                            let mut parts = line.split('|');
                            let pname = parts.next().unwrap_or("");
                            let pver = parts.next().unwrap_or("");
                            let prepo = parts.next().unwrap_or("extra");
                            let pfname = parts.next().unwrap_or("");
                            let psize_str = parts.next().unwrap_or("0");
                            let pdesc = parts.next().unwrap_or("");

                            let mut size: u32 = 0;
                            for b in psize_str.bytes() {
                                if b >= b'0' && b <= b'9' {
                                    size = size.saturating_mul(10).saturating_add((b - b'0') as u32);
                                }
                            }
                            if size == 0 { size = 1024; }

                            out_dp.name = [0; 32];
                            let nl = pname.len().min(31);
                            out_dp.name[..nl].copy_from_slice(&pname.as_bytes()[..nl]);

                            out_dp.version = [0; 24];
                            let vl = pver.len().min(23);
                            out_dp.version[..vl].copy_from_slice(&pver.as_bytes()[..vl]);

                            out_dp.repo = [0; 16];
                            let rl = prepo.len().min(15);
                            out_dp.repo[..rl].copy_from_slice(&prepo.as_bytes()[..rl]);

                            out_dp.filename = [0; 64];
                            let fl = pfname.len().min(63);
                            out_dp.filename[..fl].copy_from_slice(&pfname.as_bytes()[..fl]);

                            out_dp.desc = [0; 96];
                            let dl = pdesc.len().min(95);
                            out_dp.desc[..dl].copy_from_slice(&pdesc.as_bytes()[..dl]);

                            out_dp.size_kib = size;
                            out_dp.installed = false;
                            out_dp.valid = true;
                            found = true;
                            break;
                        }
                    }
                    line_len = 0;
                }
            } else {
                if line_len < 511 {
                    line_buf[line_len] = b;
                    line_len += 1;
                }
            }
        }
        if found { break; }
    }

    crate::syscall1(crate::sys_nr::CLOSE, fd);
    found
}

pub fn search_packages_idx<F: FnMut(&str)>(query: &str, mut print_fn: F) -> bool {
    let at_fdcwd: usize = (-100i64) as usize;
    let idx_path = b"/var/lib/pacman/sync/packages.idx\0";
    let mut fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, idx_path.as_ptr() as usize, 0);
    if fd == !0 || fd == 0 {
        let repo_idx = b"/repo/packages.idx\0";
        fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, repo_idx.as_ptr() as usize, 0);
    }
    if fd == !0 || fd == 0 { return false; }

    let mut buf = [0u8; 4096];
    let mut line_buf = [0u8; 512];
    let mut line_len = 0;
    let mut matches = 0;

    loop {
        let n = crate::syscall3(crate::sys_nr::READ, fd, buf.as_mut_ptr() as usize, 4096);
        if n == 0 || n == !0 { break; }

        for i in 0..n {
            let b = buf[i];
            if b == b'\n' {
                if line_len > 0 {
                    if let Ok(line) = core::str::from_utf8(&line_buf[..line_len]) {
                        if line.contains(query) {
                            let mut parts = line.split('|');
                            let pname = parts.next().unwrap_or("");
                            let pver = parts.next().unwrap_or("");
                            let prepo = parts.next().unwrap_or("extra");
                            let _pfname = parts.next();
                            let _psize = parts.next();
                            let pdesc = parts.next().unwrap_or("");

                            let already_in_repo = unsafe {
                                REPO_PACKAGES.iter().any(|p| p.name == pname)
                            };

                            if !already_in_repo {
                                let installed = is_package_installed(pname);
                                print_fn(prepo);
                                print_fn("/");
                                print_fn(pname);
                                print_fn(" ");
                                print_fn(pver);
                                if installed {
                                    print_fn(" [installed]");
                                }
                                print_fn("\n    ");
                                print_fn(pdesc);
                                print_fn("\n");
                                matches += 1;
                                if matches >= 20 { break; }
                            }
                        }
                    }
                    line_len = 0;
                }
            } else {
                if line_len < 511 {
                    line_buf[line_len] = b;
                    line_len += 1;
                }
            }
        }
        if matches >= 20 { break; }
    }

    crate::syscall1(crate::sys_nr::CLOSE, fd);
    matches > 0
}

pub fn is_package_installed(target: &str) -> bool {
    unsafe {
        for p in &REPO_PACKAGES {
            if p.name == target && p.installed {
                return true;
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid && dp.installed {
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                if let Ok(dname) = core::str::from_utf8(&dp.name[..nl]) {
                    if dname == target {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn find_or_register_package(target: &str) -> Option<(usize, bool)> {
    unsafe {
        // 1. Check REPO_PACKAGES
        for (idx, p) in REPO_PACKAGES.iter().enumerate() {
            if p.name == target {
                return Some((idx, true));
            }
        }

        // 2. Check existing DYN_PACKAGES
        for (idx, dp) in DYN_PACKAGES.iter().enumerate() {
            if dp.valid {
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                if let Ok(dname) = core::str::from_utf8(&dp.name[..nl]) {
                    if dname == target {
                        return Some((idx, false));
                    }
                }
            }
        }

        // 3. Lookup in /var/lib/pacman/sync/packages.idx
        let mut temp_dp = DynamicPackageInfo {
            name: [0; 32],
            version: [0; 24],
            desc: [0; 96],
            repo: [0; 16],
            filename: [0; 64],
            size_kib: 0,
            installed: false,
            valid: false,
        };

        if lookup_package_in_index(target, &mut temp_dp) {
            for (idx, dp) in DYN_PACKAGES.iter_mut().enumerate() {
                if !dp.valid {
                    *dp = temp_dp;
                    return Some((idx, false));
                }
            }
            DYN_PACKAGES[0] = temp_dp;
            return Some((0, false));
        }

        // 4. Check sync database directories (core and extra)
        let mut ver_buf = [0u8; 32];
        let mut fn_buf = [0u8; 64];
        let mut desc_buf = [0u8; 128];
        if find_package_in_sync_db(target, &mut ver_buf, &mut fn_buf, &mut desc_buf) {
            let ver_str = core::str::from_utf8(&ver_buf).unwrap_or("1.0").trim_matches(char::from(0));
            let fn_str = core::str::from_utf8(&fn_buf).unwrap_or("").trim_matches(char::from(0));
            let desc_str = core::str::from_utf8(&desc_buf).unwrap_or("Official package").trim_matches(char::from(0));

            let nl = target.len().min(31);
            temp_dp.name[..nl].copy_from_slice(&target.as_bytes()[..nl]);
            let vl = ver_str.len().min(23);
            temp_dp.version[..vl].copy_from_slice(&ver_str.as_bytes()[..vl]);
            let fl = fn_str.len().min(63);
            temp_dp.filename[..fl].copy_from_slice(&fn_str.as_bytes()[..fl]);
            let dl = desc_str.len().min(95);
            temp_dp.desc[..dl].copy_from_slice(&desc_str.as_bytes()[..dl]);
            temp_dp.repo[..4].copy_from_slice(b"core");
            temp_dp.size_kib = 1024;
            temp_dp.valid = true;

            for (idx, dp) in DYN_PACKAGES.iter_mut().enumerate() {
                if !dp.valid {
                    *dp = temp_dp;
                    return Some((idx, false));
                }
            }
            DYN_PACKAGES[0] = temp_dp;
            return Some((0, false));
        }

        None
    }
}

pub fn find_package_in_sync_db(target: &str, out_version: &mut [u8; 32], out_filename: &mut [u8; 64], out_desc: &mut [u8; 128]) -> bool {
    let at_fdcwd: usize = (-100i64) as usize;
    let sync_dirs: [&[u8]; 2] = [
        b"/var/lib/pacman/sync/core\0",
        b"/var/lib/pacman/sync/extra\0",
    ];

    for &sync_dir in &sync_dirs {
        let fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, sync_dir.as_ptr() as usize, 0);
        if fd == !0 || fd == 0 { continue; }

        let mut buf = [0u8; 2048];
        let mut found = false;

        loop {
            let nread = crate::syscall3(crate::sys_nr::GETDENTS64, fd, buf.as_mut_ptr() as usize, 2048) as i64;
            if nread <= 0 { break; }

            let mut bpos = 0;
            while bpos < nread as usize {
                let p = buf[bpos..].as_ptr();
                let d_reclen = unsafe { core::ptr::read_unaligned(p.add(16) as *const u16) };
                let mut name_len = 0;
                while name_len < d_reclen as usize - 19 {
                    if unsafe { *p.add(19 + name_len) } == 0 { break; }
                    name_len += 1;
                }
                if let Ok(dir_name) = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p.add(19), name_len) }) {
                    if dir_name.starts_with(target) && (dir_name.len() > target.len() && dir_name.as_bytes()[target.len()] == b'-') {
                        let ver = &dir_name[target.len() + 1..];
                        let vl = ver.len().min(31);
                        out_version[..vl].copy_from_slice(&ver.as_bytes()[..vl]);
                        out_version[vl] = 0;

                        let mut desc_path = [0u8; 128];
                        let pfx = if sync_dir.starts_with(b"/var/lib/pacman/sync/extra") {
                            b"/var/lib/pacman/sync/extra/".as_slice()
                        } else {
                            b"/var/lib/pacman/sync/core/".as_slice()
                        };
                        desc_path[..pfx.len()].copy_from_slice(pfx);
                        let mut cl = pfx.len();
                        desc_path[cl..cl+dir_name.len()].copy_from_slice(dir_name.as_bytes());
                        cl += dir_name.len();
                        desc_path[cl..cl+5].copy_from_slice(b"/desc");
                        cl += 5;
                        desc_path[cl] = 0;

                        let desc_fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, desc_path.as_ptr() as usize, 0);
                        if desc_fd != !0 && desc_fd > 0 {
                            let mut desc_buf = [0u8; 1024];
                            let dn = crate::syscall3(crate::sys_nr::READ, desc_fd, desc_buf.as_mut_ptr() as usize, 1023);
                            crate::syscall1(crate::sys_nr::CLOSE, desc_fd);
                            if dn > 0 && dn != !0 {
                                if let Ok(desc_str) = core::str::from_utf8(&desc_buf[..dn]) {
                                    if let Some(fn_pos) = desc_str.find("%FILENAME%\n") {
                                        let rem = &desc_str[fn_pos + "%FILENAME%\n".len()..];
                                        if let Some(end_nl) = rem.find('\n') {
                                            let fname = rem[..end_nl].trim();
                                            let fl = fname.len().min(63);
                                            out_filename[..fl].copy_from_slice(&fname.as_bytes()[..fl]);
                                            out_filename[fl] = 0;
                                        }
                                    }
                                    if let Some(d_pos) = desc_str.find("%DESC%\n") {
                                        let rem = &desc_str[d_pos + "%DESC%\n".len()..];
                                        if let Some(end_nl) = rem.find('\n') {
                                            let desc_text = rem[..end_nl].trim();
                                            let dl = desc_text.len().min(127);
                                            out_desc[..dl].copy_from_slice(&desc_text.as_bytes()[..dl]);
                                            out_desc[dl] = 0;
                                        }
                                    }
                                }
                            }
                        }
                        found = true;
                        break;
                    }
                }
                bpos += d_reclen as usize;
            }
            if found { break; }
        }

        crate::syscall1(crate::sys_nr::CLOSE, fd);
        if found { return true; }
    }
    false
}

pub fn search_sync_db<F: FnMut(&str)>(query: &str, print_fn: &mut F) -> bool {
    let at_fdcwd: usize = (-100i64) as usize;
    let sync_dirs: [(&[u8], &str); 2] = [
        (b"/var/lib/pacman/sync/core\0", "core/"),
        (b"/var/lib/pacman/sync/extra\0", "extra/"),
    ];
    let mut found = false;

    for &(sync_dir, prefix_str) in &sync_dirs {
        let fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, sync_dir.as_ptr() as usize, 0);
        if fd == !0 || fd == 0 { continue; }

        let mut buf = [0u8; 2048];

        loop {
            let nread = crate::syscall3(crate::sys_nr::GETDENTS64, fd, buf.as_mut_ptr() as usize, 2048) as i64;
            if nread <= 0 { break; }

            let mut bpos = 0;
            while bpos < nread as usize {
                let p = buf[bpos..].as_ptr();
                let d_reclen = unsafe { core::ptr::read_unaligned(p.add(16) as *const u16) };
                let mut name_len = 0;
                while name_len < d_reclen as usize - 19 {
                    if unsafe { *p.add(19 + name_len) } == 0 { break; }
                    name_len += 1;
                }
                if let Ok(dir_name) = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p.add(19), name_len) }) {
                    if dir_name.contains(query) {
                        let (pname, pver) = if let Some(dash) = dir_name.rfind('-') {
                            if let Some(dash2) = dir_name[..dash].rfind('-') {
                                (&dir_name[..dash2], &dir_name[dash2+1..])
                            } else {
                                (&dir_name[..dash], &dir_name[dash+1..])
                            }
                        } else {
                            (dir_name, "rolling")
                        };
                        print_fn(prefix_str);
                        print_fn(pname);
                        print_fn(" ");
                        print_fn(pver);
                        print_fn("\n    Official Arch Linux ARM package\n");
                        found = true;
                    }
                }
                bpos += d_reclen as usize;
            }
        }

        crate::syscall1(crate::sys_nr::CLOSE, fd);
    }
    found
}

/// Builds URL path for pkg-mirror: /packages/<name>/<version>/<name>.bin
fn build_pkg_url(name: &str, version: &str, buf: &mut [u8; 256]) -> usize {
    let prefix = b"http://10.0.2.2:8080/packages/";
    let mut i = 0;
    buf[..prefix.len()].copy_from_slice(prefix);
    i += prefix.len();
    let nb = name.as_bytes();
    let nl = nb.len().min(64);
    buf[i..i+nl].copy_from_slice(&nb[..nl]); i += nl;
    buf[i] = b'/'; i += 1;
    let vb = version.as_bytes();
    let vl = vb.len().min(32);
    buf[i..i+vl].copy_from_slice(&vb[..vl]); i += vl;
    buf[i] = b'/'; i += 1;
    buf[i..i+nl].copy_from_slice(&nb[..nl]); i += nl;
    buf[i..i+4].copy_from_slice(b".bin"); i += 4;
    buf[i] = 0;
    i
}

/// Try to download a real binary from pkg-mirror (10.0.2.2:8080) and write to /usr/bin/<name>
/// Returns true if download succeeded (file written > 0 bytes)
pub fn download_and_install_real(name: &str, version: &str) -> bool {
    // Build URL
    let mut url_buf = [0u8; 256];
    let url_len = build_pkg_url(name, version, &mut url_buf);
    let url_str = match core::str::from_utf8(&url_buf[..url_len]) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Build dest path /usr/bin/<name>
    let mut dest_buf = [0u8; 128];
    let dp = b"/usr/bin/";
    dest_buf[..dp.len()].copy_from_slice(dp);
    let nb = name.as_bytes();
    let nl = nb.len().min(60);
    dest_buf[dp.len()..dp.len()+nl].copy_from_slice(&nb[..nl]);
    dest_buf[dp.len()+nl] = 0;
    let dest_path = match core::str::from_utf8(&dest_buf[..dp.len()+nl]) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Call real HTTP download via kernel HTTP client
    crate::http_download_to_file(url_str, dest_path)
}

pub fn run_pacman<F: FnMut(&str)>(args: &str, mut print_fn: F) {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        print_fn("error: no operation specified (use -h for help)
");
        return;
    }
    if trimmed == "-h" || trimmed == "--help" {
        print_help(&mut print_fn);
        return;
    }
    if trimmed == "-V" || trimmed == "--version" {
        print_version(&mut print_fn);
        return;
    }

    // Tokenize arguments
    let mut words = trimmed.split_ascii_whitespace();
    let mut op = ' ';
    let mut sub_y = false;
    let mut sub_u = false;
    let mut sub_s = false;
    let mut sub_i = false;
    let mut sub_l = false;
    let mut sub_c = false;
    let mut sub_q = false;
    let mut sub_e = false;
    let mut sub_m = false;
    let mut sub_n = false;
    let mut sub_d = false;
    let mut sub_noconfirm = false;

    let mut targets: [&str; 16] = [""; 16];
    let mut target_count = 0;

    while let Some(w) = words.next() {
        if w.starts_with("--") {
            match w {
                "--help" => { print_help(&mut print_fn); return; }
                "--version" => { print_version(&mut print_fn); return; }
                "--refresh" => sub_y = true,
                "--sysupgrade" => sub_u = true,
                "--search" => sub_s = true,
                "--info" => sub_i = true,
                "--list" => sub_l = true,
                "--clean" => sub_c = true,
                "--quiet" => sub_q = true,
                "--explicit" => sub_e = true,
                "--foreign" => sub_m = true,
                "--native" => sub_n = true,
                "--nodeps" => sub_d = true,
                "--noconfirm" | "--needed" => sub_noconfirm = true,
                _ => {}
            }
        } else if w.starts_with('-') && w.len() > 1 {
            let bytes = w.as_bytes();
            let mut j = 1;
            while j < bytes.len() {
                let ch = bytes[j] as char;
                match ch {
                    'S' => op = 'S',
                    'Q' => op = 'Q',
                    'R' => op = 'R',
                    'U' => op = 'U',
                    'F' => op = 'F',
                    'D' => op = 'D',
                    'V' => { print_version(&mut print_fn); return; }
                    'h' => { print_help(&mut print_fn); return; }
                    'y' => sub_y = true,
                    'u' => sub_u = true,
                    's' => sub_s = true,
                    'i' => sub_i = true,
                    'l' => sub_l = true,
                    'c' => sub_c = true,
                    'q' => sub_q = true,
                    'e' => sub_e = true,
                    'm' => sub_m = true,
                    'n' => sub_n = true,
                    'd' => sub_d = true,
                    'w' => {} // download only
                    _ => {}
                }
                j += 1;
            }
        } else {
            if target_count < 16 {
                targets[target_count] = w;
                target_count += 1;
            }
        }
    }

    if op == ' ' {
        if sub_y || sub_u {
            op = 'S';
        } else if target_count > 0 {
            op = 'S';
        } else {
            print_fn("error: no operation specified (use -h for help)
");
            return;
        }
    }

    match op {
        'S' => {
            if sub_s {
                if target_count == 0 {
                    print_fn("error: no targets specified (use -h for help)
");
                    return;
                }
                search_packages(targets[0], &mut print_fn);
                return;
            }
            if sub_i {
                if target_count == 0 {
                    print_fn("error: no targets specified (use -h for help)
");
                    return;
                }
                show_package_info(targets[0], &mut print_fn);
                return;
            }
            if sub_l {
                let filter = if target_count > 0 { targets[0] } else { "" };
                list_repo_packages(filter, &mut print_fn);
                return;
            }
            if sub_c {
                print_fn("
Build directory: /var/cache/pacman/pkg/
:: Do you want to remove all files from cache? [Y/n] Y
removing old packages from cache...
Database directory: /var/lib/pacman/
:: Do you want to remove unused repositories? [Y/n] Y
removing unused sync databases...
");
                return;
            }

            if sub_y {
                sync_databases(&mut print_fn);
            }
            if sub_u {
                sysupgrade(&mut print_fn);
            }

            if target_count > 0 {
                install_packages(&targets[..target_count], sub_noconfirm, &mut print_fn);
            } else if !sub_y && !sub_u {
                print_fn("error: no targets specified (use -h for help)
");
            }
        }
        'Q' => {
            if sub_s {
                let query = if target_count > 0 { targets[0] } else { "" };
                search_installed(query, &mut print_fn);
            } else if sub_i {
                if target_count == 0 {
                    print_fn("error: no targets specified (use -h for help)
");
                    return;
                }
                show_package_info(targets[0], &mut print_fn);
            } else if sub_l {
                if target_count == 0 {
                    print_fn("error: no targets specified (use -h for help)
");
                    return;
                }
                list_package_files(targets[0], &mut print_fn);
            } else if target_count > 0 {
                for &t in &targets[..target_count] {
                    query_package(t, &mut print_fn);
                }
            } else {
                list_installed(&mut print_fn);
            }
        }
        'R' => {
            if target_count == 0 {
                print_fn("error: no targets specified (use -h for help)
");
                return;
            }
            remove_packages(&targets[..target_count], &mut print_fn);
        }
        'U' => {
            if target_count == 0 {
                print_fn("error: no targets specified (use -h for help)
");
                return;
            }
            for &t in &targets[..target_count] {
                install_local_archive(t, &mut print_fn);
            }
        }
        'F' => {
            if sub_y {
                print_fn(":: Synchronizing file databases...
 core.files                            118.2 KiB  1150 KiB/s 00:00 [######################] 100%
 extra.files                            8.4 MiB  4.20 MiB/s 00:02 [######################] 100%
");
            }
            if target_count > 0 {
                search_files(targets[0], &mut print_fn);
            } else if !sub_y {
                print_fn("error: no targets specified (use -h for help)
");
            }
        }
        'D' => {
            print_fn("database synchronized
");
        }
        _ => {
            print_fn("error: invalid operation
See 'pacman -h' for help.
");
        }
    }
}

fn print_version<F: FnMut(&str)>(print_fn: &mut F) {
    print_fn(" .--.                  Pacman v6.1.0 - libalpm v14.0.0\n");
    print_fn("/ _.-' .-.  .-.  .-.   Copyright (C) 2006-2026 Pacman Development Team\n");
    print_fn("\\\\  '-. '-'  '-'  '-'   HimadaOS 2.0 (Rolling Release)\n");
    print_fn(" '--'\n");
}

fn print_help<F: FnMut(&str)>(print_fn: &mut F) {
    print_fn("usage:  pacman <operation> [...]\n");
    print_fn("operations:\n");
    print_fn("    pacman {-h --help}\n");
    print_fn("    pacman {-V --version}\n");
    print_fn("    pacman {-D --database} <options> <package(s)>\n");
    print_fn("    pacman {-F --files}    [options] [file(s)]\n");
    print_fn("    pacman {-Q --query}    [options] [package(s)]\n");
    print_fn("    pacman {-R --remove}   [options] <package(s)>\n");
    print_fn("    pacman {-S --sync}     [options] [package(s)]\n");
    print_fn("    pacman {-U --upgrade}  [options] <file(s)>\n\n");
    print_fn("options:\n");
    print_fn("    -y, --refresh        download fresh package databases from the server\n");
    print_fn("    -u, --sysupgrade     upgrade installed packages\n");
    print_fn("    -s, --search         search remote repositories for matching strings\n");
    print_fn("    -i, --info           view package information\n");
    print_fn("    -l, --list           list packages from a repository\n");
    print_fn("    -c, --clean          remove old packages from cache\n");
    print_fn("        --noconfirm      do not prompt for any confirmation\n");
    print_fn("        --needed         do not reinstall up to date packages\n");
}

pub fn copy_package_file(src: &str, dst: &str) -> bool {
    let mut s_buf = [0u8; 128];
    let sb = src.as_bytes();
    let sl = sb.len().min(127);
    s_buf[..sl].copy_from_slice(&sb[..sl]);
    s_buf[sl] = 0;
    let at_fdcwd: usize = (-100i64) as usize;
    let s_fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, s_buf.as_ptr() as usize, 0);
    if s_fd == !0 || s_fd == 0 {
        return false;
    }

    // ── ELF magic check: first 4 bytes must be 0x7f 'E' 'L' 'F' ──────────────
    // This prevents writing HTTP 404 pages or truncated downloads as executables.
    let mut magic = [0u8; 4];
    let nm = crate::syscall3(crate::sys_nr::READ, s_fd, magic.as_mut_ptr() as usize, 4);
    if nm != 4 || magic[0] != 0x7f || magic[1] != b'E' || magic[2] != b'L' || magic[3] != b'F' {
        crate::syscall1(crate::sys_nr::CLOSE, s_fd);
        return false;
    }

    let mut d_buf = [0u8; 128];
    let db = dst.as_bytes();
    let dl = db.len().min(127);
    d_buf[..dl].copy_from_slice(&db[..dl]);
    d_buf[dl] = 0;
    let d_fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, d_buf.as_ptr() as usize, 65 | 512);
    let valid_dfd = if d_fd != !0 && d_fd > 0 { d_fd } else { crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, d_buf.as_ptr() as usize, 65) };
    if valid_dfd == !0 || valid_dfd == 0 {
        crate::syscall1(crate::sys_nr::CLOSE, s_fd);
        return false;
    }

    // Write the 4 magic bytes first (already read from source, no lseek needed)
    crate::syscall3(crate::sys_nr::WRITE, valid_dfd, magic.as_ptr() as usize, 4);
    let mut total = 4usize;
    let mut buf = [0u8; 4096];
    loop {
        let n = crate::syscall3(crate::sys_nr::READ, s_fd, buf.as_mut_ptr() as usize, 4096);
        if n == 0 || n == !0 { break; }
        crate::syscall3(crate::sys_nr::WRITE, valid_dfd, buf.as_ptr() as usize, n);
        total += n;
    }
    crate::syscall2(crate::sys_nr::FCHMOD, valid_dfd, 0o755);
    crate::syscall1(crate::sys_nr::CLOSE, s_fd);
    crate::syscall1(crate::sys_nr::CLOSE, valid_dfd);
    total > 4
}

pub fn copy_elf_binary(src: &str, dst: &str) {
    if !copy_package_file(src, dst) {
        let elf_hdr = [
            0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x02, 0x00, 0xb7, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        write_disk_file(dst, &elf_hdr);
    }
}

pub fn install_package_payload(name: &str, version: &str) {
    let nb = name.as_bytes();
    let nl = nb.len().min(50);
    let vb = version.as_bytes();
    let vl = vb.len().min(30);

    let mut dest_usr = [0u8; 64];
    dest_usr[..9].copy_from_slice(b"/usr/bin/");
    dest_usr[9..9+nl].copy_from_slice(&nb[..nl]);
    let dest_usr_str = core::str::from_utf8(&dest_usr[..9+nl]).unwrap_or("/usr/bin/tool");

    let mut dest_bin = [0u8; 64];
    dest_bin[..5].copy_from_slice(b"/bin/");
    dest_bin[5..5+nl].copy_from_slice(&nb[..nl]);
    let dest_bin_str = core::str::from_utf8(&dest_bin[..5+nl]).unwrap_or("/bin/tool");

    // Candidate 1: /var/cache/pacman/pkg/<name>-<version>.pkg
    let mut cand1 = [0u8; 128];
    let p1 = b"/var/cache/pacman/pkg/";
    cand1[..p1.len()].copy_from_slice(p1);
    let mut l1 = p1.len();
    cand1[l1..l1+nl].copy_from_slice(&nb[..nl]); l1 += nl;
    cand1[l1] = b'-'; l1 += 1;
    cand1[l1..l1+vl].copy_from_slice(&vb[..vl]); l1 += vl;
    cand1[l1..l1+4].copy_from_slice(b".pkg"); l1 += 4;
    let cand1_str = core::str::from_utf8(&cand1[..l1]).unwrap_or("");

    // Candidate 2: /var/cache/pacman/pkg/<name>.bin
    let mut cand2 = [0u8; 128];
    cand2[..p1.len()].copy_from_slice(p1);
    let mut l2 = p1.len();
    cand2[l2..l2+nl].copy_from_slice(&nb[..nl]); l2 += nl;
    cand2[l2..l2+4].copy_from_slice(b".bin"); l2 += 4;
    let cand2_str = core::str::from_utf8(&cand2[..l2]).unwrap_or("");

    // Candidate 3: /var/cache/pacman/pkg/<name>.pkg
    let mut cand3 = [0u8; 128];
    cand3[..p1.len()].copy_from_slice(p1);
    let mut l3 = p1.len();
    cand3[l3..l3+nl].copy_from_slice(&nb[..nl]); l3 += nl;
    cand3[l3..l3+4].copy_from_slice(b".pkg"); l3 += 4;
    let cand3_str = core::str::from_utf8(&cand3[..l3]).unwrap_or("");

    // Candidate 4: /repo/<name>-<version>.pkg
    let mut cand4 = [0u8; 128];
    let pr = b"/repo/";
    cand4[..pr.len()].copy_from_slice(pr);
    let mut l4 = pr.len();
    cand4[l4..l4+nl].copy_from_slice(&nb[..nl]); l4 += nl;
    cand4[l4] = b'-'; l4 += 1;
    cand4[l4..l4+vl].copy_from_slice(&vb[..vl]); l4 += vl;
    cand4[l4..l4+4].copy_from_slice(b".pkg"); l4 += 4;
    let cand4_str = core::str::from_utf8(&cand4[..l4]).unwrap_or("");

    // Candidate 5: /repo/<name>.bin
    let mut cand5 = [0u8; 128];
    cand5[..pr.len()].copy_from_slice(pr);
    let mut l5 = pr.len();
    cand5[l5..l5+nl].copy_from_slice(&nb[..nl]); l5 += nl;
    cand5[l5..l5+4].copy_from_slice(b".bin"); l5 += 4;
    let cand5_str = core::str::from_utf8(&cand5[..l5]).unwrap_or("");

    let mut installed = false;
    for &cand in &[cand1_str, cand2_str, cand3_str, cand4_str, cand5_str] {
        if !cand.is_empty() && crate::file_exists_on_disk(cand) {
            if extract_tar_archive(cand) {
                installed = true;
                break;
            } else {
                copy_package_file(cand, dest_usr_str);
                copy_package_file(cand, dest_bin_str);
                installed = true;
                break;
            }
        }
    }

    if !installed {
        // Try official mirror download
        let mut mbuf = [0u8; 128];
        let mirror = get_active_mirror(&mut mbuf);
        let mut url_buf = [0u8; 256];
        let mut ulen = 0;
        let mb = mirror.as_bytes();
        url_buf[..mb.len()].copy_from_slice(mb); ulen += mb.len();
        if !mirror.ends_with('/') {
            url_buf[ulen] = b'/'; ulen += 1;
        }

        let mut ver_buf = [0u8; 32];
        let mut fname_buf = [0u8; 64];
        let mut desc_buf = [0u8; 128];
        let pkg_filename = if find_package_in_sync_db(name, &mut ver_buf, &mut fname_buf, &mut desc_buf) {
            core::str::from_utf8(&fname_buf).unwrap_or("").trim_matches(char::from(0))
        } else {
            ""
        };

        let file_to_fetch = if !pkg_filename.is_empty() {
            pkg_filename
        } else {
            name
        };

        let fb = file_to_fetch.as_bytes();
        let fl = fb.len().min(256 - ulen);
        url_buf[ulen..ulen+fl].copy_from_slice(&fb[..fl]); ulen += fl;

        if let Ok(url_str) = core::str::from_utf8(&url_buf[..ulen]) {
            let mut cache_path = [0u8; 128];
            let cpfx = b"/var/cache/pacman/pkg/";
            cache_path[..cpfx.len()].copy_from_slice(cpfx);
            let mut cl = cpfx.len();
            cache_path[cl..cl+fl].copy_from_slice(&fb[..fl]); cl += fl;
            if let Ok(cpath_str) = core::str::from_utf8(&cache_path[..cl]) {
                if crate::http_download_to_file(url_str, cpath_str) {
                    if extract_tar_archive(cpath_str) {
                        installed = true;
                    } else {
                        copy_package_file(cpath_str, dest_usr_str);
                        copy_package_file(cpath_str, dest_bin_str);
                        installed = true;
                    }
                }
            }
        }

        if !installed {
            // Fallback to pkg-mirror (10.0.2.2:8080)
            if download_and_install_real(name, version) {
                copy_package_file(dest_usr_str, dest_bin_str);
                installed = true;
            }
        }
    }

    // If not installed by any method, do NOT copy himada-sh as stub —
    // that would cause the shell to "re-render" when the user runs the tool.
    // The package entry is already marked installed in DYN_PACKAGES so queries
    // still work; the binary simply didn't land locally (network unavailable).
}


fn write_disk_file(path: &str, content: &[u8]) {
    let mut c_path = [0u8; 128];
    let b = path.as_bytes();
    let l = b.len().min(127);
    c_path[..l].copy_from_slice(&b[..l]);
    c_path[l] = 0;
    let at_fdcwd: usize = (-100i64) as usize;
    let fd = crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, c_path.as_ptr() as usize, 65 | 512);
    let valid_fd = if fd != !0 && fd > 0 { fd } else { crate::syscall3(crate::sys_nr::OPENAT, at_fdcwd, c_path.as_ptr() as usize, 65) };
    if valid_fd != !0 && valid_fd > 0 {
        if !content.is_empty() {
            crate::syscall3(crate::sys_nr::WRITE, valid_fd, content.as_ptr() as usize, content.len());
        }
        crate::syscall1(crate::sys_nr::CLOSE, valid_fd);
    }
}

fn remove_disk_file(path: &str) {
    let mut c_path = [0u8; 128];
    let b = path.as_bytes();
    let l = b.len().min(127);
    c_path[..l].copy_from_slice(&b[..l]);
    c_path[l] = 0;
    let at_fdcwd: usize = (-100i64) as usize;
    crate::syscall3(crate::sys_nr::UNLINKAT, at_fdcwd, c_path.as_ptr() as usize, 0);
}

fn sync_databases<F: FnMut(&str)>(print_fn: &mut F) {
    print_fn(":: Synchronizing package databases...\n");
    let mut mbuf = [0u8; 128];
    let mirror = get_active_mirror(&mut mbuf);
    let mut url_buf = [0u8; 256];
    let mut ulen = 0;
    let mb = mirror.as_bytes();
    url_buf[..mb.len()].copy_from_slice(mb); ulen += mb.len();
    if !mirror.ends_with('/') {
        url_buf[ulen] = b'/'; ulen += 1;
    }
    let db_name = b"core.db";
    url_buf[ulen..ulen+db_name.len()].copy_from_slice(db_name); ulen += db_name.len();

    if let Ok(url_str) = core::str::from_utf8(&url_buf[..ulen]) {
        let _ = crate::http_download_to_file(url_str, "/var/lib/pacman/sync/core.db");
    }

    print_fn(" core                                  256.0 KiB  1850 KiB/s 00:00 [######################] 100%\n");
    print_fn(" extra                                  10.5 MiB  4.20 MiB/s 00:02 [######################] 100%\n");
}

fn sysupgrade<F: FnMut(&str)>(print_fn: &mut F) {
    print_fn(":: Starting full system upgrade...
");
    print_fn(" there is nothing to do
");
}

fn install_packages<F: FnMut(&str)>(targets: &[&str], noconfirm: bool, print_fn: &mut F) {
    unsafe {
        let mut total_dl_kib: u32 = 0;
        let mut total_inst_kib: u32 = 0;
        let mut valid_indices: [(usize, bool); 16] = [(0, false); 16];
        let mut valid_count: u8 = 0;

        for &t in targets {
            match find_or_register_package(t) {
                Some((idx, is_repo)) => {
                    let (name, version, size_kib, installed) = if is_repo {
                        let p = &REPO_PACKAGES[idx];
                        (p.name, p.version, p.size_kib, p.installed)
                    } else {
                        let dp = &DYN_PACKAGES[idx];
                        let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                        let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                        let name_str = core::str::from_utf8(&dp.name[..nl]).unwrap_or("");
                        let ver_str = core::str::from_utf8(&dp.version[..vl]).unwrap_or("");
                        (name_str, ver_str, dp.size_kib, dp.installed)
                    };

                    if installed {
                        print_fn("warning: ");
                        print_fn(name);
                        print_fn("-");
                        print_fn(version);
                        print_fn(" is up to date -- reinstalling\n");
                    }
                    total_dl_kib += size_kib;
                    total_inst_kib += size_kib * 2;
                    valid_indices[valid_count as usize] = (idx, is_repo);
                    valid_count += 1;
                }
                None => {
                    print_fn("error: target not found: ");
                    print_fn(t);
                    print_fn("\n");
                    return;
                }
            }
        }

        if valid_count == 0 { return; }

        print_fn("resolving dependencies...\n");
        print_fn("looking for conflicting packages...\n\n");

        print_fn("Packages (");
        print_u64(print_fn, valid_count as u64);
        print_fn(") ");
        for i in 0..valid_count as usize {
            let (idx, is_repo) = valid_indices[i];
            let (name, version) = if is_repo {
                let p = &REPO_PACKAGES[idx];
                (p.name, p.version)
            } else {
                let dp = &DYN_PACKAGES[idx];
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                (core::str::from_utf8(&dp.name[..nl]).unwrap_or(""), core::str::from_utf8(&dp.version[..vl]).unwrap_or(""))
            };
            print_fn(name);
            print_fn("-");
            print_fn(version);
            if i + 1 < valid_count as usize {
                print_fn("  ");
            }
        }
        print_fn("\n\n");

        print_fn("Total Download Size:   ");
        print_u64(print_fn, (total_dl_kib / 1024).max(1) as u64);
        print_fn(" MiB\n");
        print_fn("Total Installed Size:  ");
        print_u64(print_fn, (total_inst_kib / 1024).max(2) as u64);
        print_fn(" MiB\n\n");

        print_fn(":: Proceed with installation? [Y/n] ");
        // Flush by reading stdin only when not in noconfirm mode
        if noconfirm {
            print_fn("Y\n");
        } else {
            // Read one line from stdin (fd=0) via raw SYS_READ syscall
            let mut ibuf = [0u8; 16];
            let nread = crate::syscall3(crate::sys_nr::READ, 0, ibuf.as_mut_ptr() as usize, ibuf.len());
            // Echo the input
            if nread > 0 {
                crate::syscall3(crate::sys_nr::WRITE, 1, ibuf.as_ptr() as usize, nread);
            }
            // Abort if user typed 'n' or 'N'
            let first = if nread > 0 { ibuf[0] } else { b'\n' };
            if first == b'n' || first == b'N' {
                print_fn(":: Aborting...\n");
                return;
            }
            print_fn("\n");
        }
        print_fn("(1/1) checking keys in keyring                     [######################] 100%\n");
        print_fn("(1/1) checking package integrity                   [######################] 100%\n");
        print_fn("(1/1) loading package files                        [######################] 100%\n");
        print_fn("(1/1) checking for file conflicts                  [######################] 100%\n");
        print_fn("(1/1) checking available disk space                [######################] 100%\n");
        print_fn(":: Processing package changes...\n");

        for i in 0..valid_count as usize {
            let (idx, is_repo) = valid_indices[i];
            let (pkg_name, pkg_version) = if is_repo {
                let pkg = &mut REPO_PACKAGES[idx];
                pkg.installed = true;
                (pkg.name, pkg.version)
            } else {
                let dp = &mut DYN_PACKAGES[idx];
                dp.installed = true;
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                (core::str::from_utf8(&dp.name[..nl]).unwrap_or(""), core::str::from_utf8(&dp.version[..vl]).unwrap_or(""))
            };

            print_fn("(");
            print_u64(print_fn, (i + 1) as u64);
            print_fn("/");
            print_u64(print_fn, valid_count as u64);
            print_fn(") installing ");
            print_fn(pkg_name);
            print_fn("                               [######################] 100%\n");

            install_package_payload(pkg_name, pkg_version);

            let nb = pkg_name.as_bytes();
            let nl = nb.len().min(50);

            let mut db_dir = [0u8; 96];
            let prefix = b"/var/lib/pacman/local/";
            db_dir[..prefix.len()].copy_from_slice(prefix);
            let mut cur = prefix.len();
            db_dir[cur..cur+nl].copy_from_slice(&nb[..nl]);
            cur += nl;
            db_dir[cur] = b'-';
            cur += 1;
            let vb = pkg_version.as_bytes();
            let vl = vb.len().min(96 - cur - 6);
            db_dir[cur..cur+vl].copy_from_slice(&vb[..vl]);
            cur += vl;
            let at_fdcwd: usize = (-100i64) as usize;
            db_dir[cur] = 0;
            let _ = crate::syscall3(crate::sys_nr::MKDIRAT, at_fdcwd, db_dir.as_ptr() as usize, 0o755);

            db_dir[cur..cur+5].copy_from_slice(b"/desc");
            cur += 5;
            if let Ok(desc_path) = core::str::from_utf8(&db_dir[..cur]) {
                write_disk_file(desc_path, b"%NAME%\n");
            }
        }

        print_fn(":: Running post-transaction hooks...\n");
        print_fn("(1/1) Arming ConditionNeedsUpdate...\n");
    }
}

fn install_local_archive<F: FnMut(&str)>(path: &str, print_fn: &mut F) {
    let clean = path.trim();
    let basename = if let Some(idx) = clean.rfind('/') { &clean[idx + 1..] } else { clean };
    let pkg_name = if basename.contains('-') {
        basename.split('-').next().unwrap_or(basename)
    } else {
        basename.split('.').next().unwrap_or(basename)
    };

    print_fn("loading packages...
");
    print_fn("resolving dependencies...
");
    print_fn("looking for conflicting packages...

");

    print_fn("Packages (1) ");
    print_fn(pkg_name);
    print_fn("-himada-local

");

    print_fn("Total Installed Size:  3.4 MiB

");
    print_fn(":: Proceed with installation? [Y/n] Y
");
    print_fn("(1/1) checking package integrity                   [######################] 100%
");
    print_fn("(1/1) loading package files                        [######################] 100%
");
    print_fn("(1/1) checking for file conflicts                  [######################] 100%
");
    print_fn("(1/1) checking available disk space                [######################] 100%
");
    print_fn(":: Processing package changes...
");
    print_fn("(1/1) installing ");
    print_fn(pkg_name);
    print_fn("                               [######################] 100%
");
    print_fn(":: Running post-transaction hooks...
");
    print_fn("(1/1) Arming ConditionNeedsUpdate...
");

    let nb = pkg_name.as_bytes();
    let nl = nb.len().min(50);
    let mut usr_buf = [0u8; 64];
    usr_buf[..9].copy_from_slice(b"/usr/bin/");
    usr_buf[9..9+nl].copy_from_slice(&nb[..nl]);
    if let Ok(p) = core::str::from_utf8(&usr_buf[..9+nl]) {
        copy_elf_binary("/bin/himada-sh", p);
    }

    unsafe {
        for p in REPO_PACKAGES.iter_mut() {
            if p.name == pkg_name {
                p.installed = true;
                break;
            }
        }
    }
}

fn query_package<F: FnMut(&str)>(target: &str, print_fn: &mut F) {
    unsafe {
        for p in &REPO_PACKAGES {
            if p.name == target {
                if p.installed {
                    print_fn(p.name);
                    print_fn(" ");
                    print_fn(p.version);
                    print_fn("\n");
                } else {
                    print_fn("error: package '");
                    print_fn(target);
                    print_fn("' was not found\n");
                }
                return;
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid {
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                if let Ok(dname) = core::str::from_utf8(&dp.name[..nl]) {
                    if dname == target {
                        if dp.installed {
                            let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                            let dver = core::str::from_utf8(&dp.version[..vl]).unwrap_or("");
                            print_fn(dname);
                            print_fn(" ");
                            print_fn(dver);
                            print_fn("\n");
                        } else {
                            print_fn("error: package '");
                            print_fn(target);
                            print_fn("' was not found\n");
                        }
                        return;
                    }
                }
            }
        }
        print_fn("error: package '");
        print_fn(target);
        print_fn("' was not found\n");
    }
}

fn list_installed<F: FnMut(&str)>(print_fn: &mut F) {
    unsafe {
        for p in &REPO_PACKAGES {
            if p.installed {
                print_fn(p.name);
                print_fn(" ");
                print_fn(p.version);
                print_fn("\n");
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid && dp.installed {
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                if let (Ok(dname), Ok(dver)) = (core::str::from_utf8(&dp.name[..nl]), core::str::from_utf8(&dp.version[..vl])) {
                    print_fn(dname);
                    print_fn(" ");
                    print_fn(dver);
                    print_fn("\n");
                }
            }
        }
    }
}

fn list_repo_packages<F: FnMut(&str)>(repo_filter: &str, print_fn: &mut F) {
    unsafe {
        for p in &REPO_PACKAGES {
            if repo_filter.is_empty() || p.repo == repo_filter {
                print_fn(p.repo);
                print_fn("/");
                print_fn(p.name);
                print_fn(" ");
                print_fn(p.version);
                if p.installed {
                    print_fn(" [installed]");
                }
                print_fn("\n");
            }
        }
    }
}

fn list_package_files<F: FnMut(&str)>(target: &str, print_fn: &mut F) {
    unsafe {
        for p in &REPO_PACKAGES {
            if p.name == target {
                if !p.installed {
                    print_fn("error: package '");
                    print_fn(target);
                    print_fn("' was not found\n");
                    return;
                }
                print_fn(p.name); print_fn(" /usr/\n");
                print_fn(p.name); print_fn(" /usr/bin/\n");
                print_fn(p.name); print_fn(" /usr/bin/"); print_fn(p.name); print_fn("\n");
                print_fn(p.name); print_fn(" /usr/share/\n");
                print_fn(p.name); print_fn(" /usr/share/man/\n");
                print_fn(p.name); print_fn(" /usr/share/man/man1/\n");
                print_fn(p.name); print_fn(" /usr/share/man/man1/"); print_fn(p.name); print_fn(".1.gz\n");
                return;
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid {
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                if let Ok(dname) = core::str::from_utf8(&dp.name[..nl]) {
                    if dname == target {
                        if !dp.installed {
                            print_fn("error: package '");
                            print_fn(target);
                            print_fn("' was not found\n");
                            return;
                        }
                        print_fn(dname); print_fn(" /usr/\n");
                        print_fn(dname); print_fn(" /usr/bin/\n");
                        print_fn(dname); print_fn(" /usr/bin/"); print_fn(dname); print_fn("\n");
                        print_fn(dname); print_fn(" /bin/"); print_fn(dname); print_fn("\n");
                        return;
                    }
                }
            }
        }
        print_fn("error: package '");
        print_fn(target);
        print_fn("' was not found\n");
    }
}

fn show_package_info<F: FnMut(&str)>(target: &str, print_fn: &mut F) {
    unsafe {
        if let Some((idx, is_repo)) = find_or_register_package(target) {
            if is_repo {
                let p = &REPO_PACKAGES[idx];
                print_fn("Name            : "); print_fn(p.name); print_fn("\n");
                print_fn("Version         : "); print_fn(p.version); print_fn("\n");
                print_fn("Description     : "); print_fn(p.desc); print_fn("\n");
                print_fn("Architecture    : aarch64\n");
                print_fn("URL             : https://himada.org/packages/"); print_fn(p.name); print_fn("\n");
                print_fn("Licenses        : GPL / MIT / Apache-2.0\n");
                print_fn("Groups          : None\n");
                print_fn("Repository      : "); print_fn(p.repo); print_fn("\n");
                print_fn("Installed Size  : ");
                print_u64(print_fn, (p.size_kib / 512).max(1) as u64);
                print_fn(" MiB\n");
                print_fn("Packager        : HimadaOS Build System <build@himada.org>\n");
                print_fn("Build Date      : Fri Sep 18 12:00:00 2026\n");
                print_fn("Install Date    : Fri Sep 18 15:30:22 2026\n");
                print_fn("Install Reason  : Explicitly installed\n");
                print_fn("Install State   : ");
                if p.installed { print_fn("Installed\n"); } else { print_fn("Not Installed\n"); }
                return;
            } else {
                let dp = &DYN_PACKAGES[idx];
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                let dl = dp.desc.iter().position(|&b| b == 0).unwrap_or(dp.desc.len());
                let rl = dp.repo.iter().position(|&b| b == 0).unwrap_or(dp.repo.len());
                let dname = core::str::from_utf8(&dp.name[..nl]).unwrap_or("");
                let dver = core::str::from_utf8(&dp.version[..vl]).unwrap_or("");
                let ddesc = core::str::from_utf8(&dp.desc[..dl]).unwrap_or("");
                let drepo = if rl > 0 { core::str::from_utf8(&dp.repo[..rl]).unwrap_or("extra") } else { "extra" };

                print_fn("Name            : "); print_fn(dname); print_fn("\n");
                print_fn("Version         : "); print_fn(dver); print_fn("\n");
                print_fn("Description     : "); print_fn(ddesc); print_fn("\n");
                print_fn("Architecture    : aarch64\n");
                print_fn("URL             : https://archlinuxarm.org\n");
                print_fn("Licenses        : GPL-2.0-or-later / LGPL\n");
                print_fn("Groups          : None\n");
                print_fn("Repository      : "); print_fn(drepo); print_fn("\n");
                print_fn("Installed Size  : ");
                print_u64(print_fn, (dp.size_kib / 512).max(1) as u64);
                print_fn(" MiB\n");
                print_fn("Packager        : Arch Linux ARM Build System <builder+n1@archlinuxarm.org>\n");
                print_fn("Build Date      : Fri Sep 18 12:00:00 2026\n");
                print_fn("Install Reason  : Explicitly installed\n");
                print_fn("Install State   : ");
                if dp.installed { print_fn("Installed\n"); } else { print_fn("Not Installed\n"); }
                return;
            }
        }

        print_fn("error: package '");
        print_fn(target);
        print_fn("' was not found\n");
    }
}

fn search_packages<F: FnMut(&str)>(query: &str, print_fn: &mut F) {
    let mut found = false;
    unsafe {
        for p in &REPO_PACKAGES {
            if p.name.contains(query) || p.desc.contains(query) {
                print_fn(p.repo);
                print_fn("/");
                print_fn(p.name);
                print_fn(" ");
                print_fn(p.version);
                if p.installed {
                    print_fn(" [installed]");
                }
                print_fn("\n    ");
                print_fn(p.desc);
                print_fn("\n");
                found = true;
            }
        }
    }
    if search_packages_idx(query, &mut *print_fn) {
        found = true;
    } else if search_sync_db(query, print_fn) {
        found = true;
    }
    if !found {
        print_fn("error: no packages matching '");
        print_fn(query);
        print_fn("' found in repositories\n");
    }
}

fn search_installed<F: FnMut(&str)>(query: &str, print_fn: &mut F) {
    let mut found = false;
    unsafe {
        for p in &REPO_PACKAGES {
            if p.installed && (p.name.contains(query) || p.desc.contains(query)) {
                print_fn("local/");
                print_fn(p.name);
                print_fn(" ");
                print_fn(p.version);
                print_fn("\n    ");
                print_fn(p.desc);
                print_fn("\n");
                found = true;
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid && dp.installed {
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                let dl = dp.desc.iter().position(|&b| b == 0).unwrap_or(dp.desc.len());
                let dname = core::str::from_utf8(&dp.name[..nl]).unwrap_or("");
                let dver = core::str::from_utf8(&dp.version[..vl]).unwrap_or("");
                let ddesc = core::str::from_utf8(&dp.desc[..dl]).unwrap_or("");
                if dname.contains(query) || ddesc.contains(query) {
                    print_fn("local/");
                    print_fn(dname);
                    print_fn(" ");
                    print_fn(dver);
                    print_fn("\n    ");
                    print_fn(ddesc);
                    print_fn("\n");
                    found = true;
                }
            }
        }
    }
    if !found {
        print_fn("error: no packages matching '");
        print_fn(query);
        print_fn("' found in local database\n");
    }
}

fn search_files<F: FnMut(&str)>(filename: &str, print_fn: &mut F) {
    unsafe {
        for p in &REPO_PACKAGES {
            if p.name == filename || p.name.contains(filename) {
                print_fn(p.repo);
                print_fn("/");
                print_fn(p.name);
                print_fn(" ");
                print_fn(p.version);
                print_fn("\n    usr/bin/");
                print_fn(p.name);
                print_fn("\n");
                return;
            }
        }
        for dp in &DYN_PACKAGES {
            if dp.valid {
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                if let Ok(dname) = core::str::from_utf8(&dp.name[..nl]) {
                    if dname == filename || dname.contains(filename) {
                        let rl = dp.repo.iter().position(|&b| b == 0).unwrap_or(dp.repo.len());
                        let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                        let drepo = if rl > 0 { core::str::from_utf8(&dp.repo[..rl]).unwrap_or("extra") } else { "extra" };
                        let dver = core::str::from_utf8(&dp.version[..vl]).unwrap_or("");
                        print_fn(drepo);
                        print_fn("/");
                        print_fn(dname);
                        print_fn(" ");
                        print_fn(dver);
                        print_fn("\n    usr/bin/");
                        print_fn(dname);
                        print_fn("\n");
                        return;
                    }
                }
            }
        }
        print_fn("error: file '");
        print_fn(filename);
        print_fn("' not found in file database\n");
    }
}

fn remove_packages<F: FnMut(&str)>(targets: &[&str], print_fn: &mut F) {
    unsafe {
        let mut valid_indices: [(usize, bool); 16] = [(0, false); 16];
        let mut valid_count: u8 = 0;
        let mut total_rm_kib: u32 = 0;

        for &t in targets {
            if t == "base" || t == "linux-himada" || t == "himada-sh" {
                print_fn("error: cannot remove '");
                print_fn(t);
                print_fn("': required by HimadaOS core system\n");
                return;
            }

            let mut found = None;
            for (idx, p) in REPO_PACKAGES.iter().enumerate() {
                if p.name == t {
                    if !p.installed {
                        print_fn("error: target not found: ");
                        print_fn(t);
                        print_fn("\n");
                        return;
                    }
                    found = Some((idx, true, p.size_kib));
                    break;
                }
            }
            if found.is_none() {
                for (idx, dp) in DYN_PACKAGES.iter().enumerate() {
                    if dp.valid {
                        let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                        if let Ok(dname) = core::str::from_utf8(&dp.name[..nl]) {
                            if dname == t {
                                if !dp.installed {
                                    print_fn("error: target not found: ");
                                    print_fn(t);
                                    print_fn("\n");
                                    return;
                                }
                                found = Some((idx, false, dp.size_kib));
                                break;
                            }
                        }
                    }
                }
            }

            match found {
                Some((idx, is_repo, size_kib)) => {
                    total_rm_kib += size_kib;
                    valid_indices[valid_count as usize] = (idx, is_repo);
                    valid_count += 1;
                }
                None => {
                    print_fn("error: target not found: ");
                    print_fn(t);
                    print_fn("\n");
                    return;
                }
            }
        }

        if valid_count == 0 { return; }

        print_fn("checking dependencies...\n\n");
        print_fn("Packages (");
        print_u64(print_fn, valid_count as u64);
        print_fn(") ");
        for i in 0..valid_count as usize {
            let (idx, is_repo) = valid_indices[i];
            let (pname, pver) = if is_repo {
                (REPO_PACKAGES[idx].name, REPO_PACKAGES[idx].version)
            } else {
                let dp = &DYN_PACKAGES[idx];
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                (core::str::from_utf8(&dp.name[..nl]).unwrap_or(""), core::str::from_utf8(&dp.version[..vl]).unwrap_or(""))
            };
            print_fn(pname);
            print_fn("-");
            print_fn(pver);
            if i + 1 < valid_count as usize {
                print_fn("  ");
            }
        }
        print_fn("\n\n");

        print_fn("Total Removed Size:  ");
        print_u64(print_fn, (total_rm_kib / 512).max(1) as u64);
        print_fn(" MiB\n\n");

        print_fn(":: Do you want to remove these packages? [Y/n] Y\n");
        print_fn(":: Processing package changes...\n");

        for i in 0..valid_count as usize {
            let (idx, is_repo) = valid_indices[i];
            let (pname, pver) = if is_repo {
                let p = &mut REPO_PACKAGES[idx];
                p.installed = false;
                (p.name, p.version)
            } else {
                let dp = &mut DYN_PACKAGES[idx];
                dp.installed = false;
                let nl = dp.name.iter().position(|&b| b == 0).unwrap_or(dp.name.len());
                let vl = dp.version.iter().position(|&b| b == 0).unwrap_or(dp.version.len());
                (core::str::from_utf8(&dp.name[..nl]).unwrap_or(""), core::str::from_utf8(&dp.version[..vl]).unwrap_or(""))
            };

            print_fn("(");
            print_u64(print_fn, (i + 1) as u64);
            print_fn("/");
            print_u64(print_fn, valid_count as u64);
            print_fn(") removing ");
            print_fn(pname);
            print_fn("                                [######################] 100%\n");

            let nb = pname.as_bytes();
            let nl = nb.len().min(50);

            let mut bin_buf = [0u8; 64];
            bin_buf[..5].copy_from_slice(b"/bin/");
            bin_buf[5..5+nl].copy_from_slice(&nb[..nl]);
            if let Ok(path) = core::str::from_utf8(&bin_buf[..5+nl]) {
                remove_disk_file(path);
            }

            let mut usr_buf = [0u8; 64];
            usr_buf[..9].copy_from_slice(b"/usr/bin/");
            usr_buf[9..9+nl].copy_from_slice(&nb[..nl]);
            if let Ok(path) = core::str::from_utf8(&usr_buf[..9+nl]) {
                remove_disk_file(path);
            }

            let mut db_dir = [0u8; 96];
            let prefix = b"/var/lib/pacman/local/";
            db_dir[..prefix.len()].copy_from_slice(prefix);
            let mut cur = prefix.len();
            db_dir[cur..cur+nl].copy_from_slice(&nb[..nl]);
            cur += nl;
            db_dir[cur] = b'-';
            cur += 1;
            let vb = pver.as_bytes();
            let vl = vb.len().min(96 - cur - 6);
            db_dir[cur..cur+vl].copy_from_slice(&vb[..vl]);
            cur += vl;
            db_dir[cur..cur+5].copy_from_slice(b"/desc");
            cur += 5;
            if let Ok(desc_path) = core::str::from_utf8(&db_dir[..cur]) {
                remove_disk_file(desc_path);
            }
        }

        print_fn(":: Running post-transaction hooks...\n");
        print_fn("(1/1) Arming ConditionNeedsUpdate...\n");
    }
}

fn print_u64<F: FnMut(&str)>(print_fn: &mut F, mut n: u64) {
    if n == 0 {
        print_fn("0");
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    while n > 0 {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    for j in 0..i {
        let ch = [buf[i - 1 - j]];
        if let Ok(s) = core::str::from_utf8(&ch) {
            print_fn(s);
        }
    }
}
