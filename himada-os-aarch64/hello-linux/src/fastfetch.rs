// Fastfetch System Information Tool for HimadaOS

pub fn render_fastfetch<F: FnMut(&str)>(
    mut print_fn: F,
    hostname: &str,
    uptime_str: &str,
    mem_used_mib: u64,
    mem_total_mib: u64,
    pkg_count: usize,
    arch: &str,
) {
    let c_reset = "\x1b[0m";
    let c_bold = "\x1b[1m";
    let c_dim = "\x1b[90m";

    // Row 0
    print_fn("\x1b[38;5;51m            /\\        /\\             ");
    print_fn("\x1b[1;38;5;51mroot\x1b[0m@\x1b[1;38;5;141m"); print_fn(hostname); print_fn(c_reset); print_fn("\n");

    // Row 1
    print_fn("\x1b[38;5;45m           /  \\      /  \\            ");
    print_fn(c_dim); print_fn("-----------------------------------\n");

    // Row 2
    print_fn("\x1b[38;5;39m          / /\\ \\    / /\\ \\           ");
    print_fn(c_bold); print_fn("OS: "); print_fn(c_reset);
    print_fn("HimadaOS 2.0 (Rolling Release) ["); print_fn(arch); print_fn("]\n");

    // Row 3
    print_fn("\x1b[38;5;33m         / /  \\ \\  / /  \\ \\          ");
    print_fn(c_bold); print_fn("Host: "); print_fn(c_reset);
    print_fn("QEMU KVM ARM64 Virtual Machine\n");

    // Row 4
    print_fn("\x1b[38;5;27m        / /    \\ \\/ /    \\ \\         ");
    print_fn(c_bold); print_fn("Kernel: "); print_fn(c_reset);
    print_fn("6.8.0-himada (SMP PREEMPT)\n");

    // Row 5
    print_fn("\x1b[38;5;63m       / /      \\  /      \\ \\        ");
    print_fn(c_bold); print_fn("Uptime: "); print_fn(c_reset);
    print_fn(uptime_str); print_fn("\n");

    // Row 6
    print_fn("\x1b[38;5;99m      / /        \\/        \\ \\       ");
    print_fn(c_bold); print_fn("Packages: "); print_fn(c_reset);
    print_u64(&mut print_fn, pkg_count as u64); print_fn(" (himada-pkg)\n");

    // Row 7
    print_fn("\x1b[38;5;135m     / /   .------------.   \\ \\      ");
    print_fn(c_bold); print_fn("Shell: "); print_fn(c_reset);
    print_fn("himada-sh 2.0 (Kani SMT proven)\n");

    // Row 8
    print_fn("\x1b[38;5;141m    / /    |  HIMADA OS |    \\ \\     ");
    print_fn(c_bold); print_fn("Terminal: "); print_fn(c_reset);
    print_fn("/dev/pts/0 (VT100 ANSI)\n");

    // Row 9
    print_fn("\x1b[38;5;147m   / /     '------------'     \\ \\    ");
    print_fn(c_bold); print_fn("CPU: "); print_fn(c_reset);
    print_fn("4x ARM Cortex-A72 @ 1.50GHz\n");

    // Row 10
    print_fn("\x1b[38;5;141m  / /      /\\          /\\      \\ \\   ");
    print_fn(c_bold); print_fn("Memory: "); print_fn(c_reset);
    print_u64(&mut print_fn, mem_used_mib); print_fn(" MiB / ");
    print_u64(&mut print_fn, mem_total_mib); print_fn(" MiB\n");

    // Row 11
    print_fn("\x1b[38;5;135m / /______/ /\\________/ /\\______\\ \\  ");
    print_fn(c_bold); print_fn("Disk (/): "); print_fn(c_reset);
    print_fn("64 MiB Ext4 (VirtIO-Blk /dev/vda)\n");

    // Row 12
    print_fn("\x1b[38;5;99m/________/ /  \\______/ /  \\_______\\  ");
    print_fn(c_bold); print_fn("Security: "); print_fn(c_reset);
    print_fn("W^X, Safe-Rust Ring0, Kani SMT\n");

    // Row 13
    print_fn("\x1b[38;5;63m\\________\\/            \\/________/   ");
    // Palette blocks
    print_fn("\x1b[40m   \x1b[41m   \x1b[42m   \x1b[43m   \x1b[44m   \x1b[45m   \x1b[46m   \x1b[47m   \x1b[0m\n");

    // Row 14
    print_fn("                                     ");
    print_fn("\x1b[100m   \x1b[101m   \x1b[102m   \x1b[103m   \x1b[104m   \x1b[105m   \x1b[106m   \x1b[107m   \x1b[0m\n");

    print_fn(c_reset);
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
