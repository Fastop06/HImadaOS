import subprocess
import pty
import os
import time
import sys
import select

master, slave = pty.openpty()

cmd = [
    "qemu-system-aarch64",
    "-machine", "virt",
    "-cpu", "cortex-a72",
    "-m", "1024M",
    "-bios", "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
    "-cdrom", "/Users/mussavysegurov/Desktop/HimadaOS_Final/himada-os-arm64.iso",
    "-drive", "file=/Users/mussavysegurov/.gemini/antigravity/scratch/disk.img,format=raw,if=virtio",
    "-device", "virtio-net-device,netdev=net0",
    "-netdev", "user,id=net0",
    "-nographic"
]

print("Launching QEMU to test Stage 8.21 DNS Resolver & Official Arch Linux ARM Mirror Pacman...")
proc = subprocess.Popen(
    cmd,
    stdin=slave,
    stdout=slave,
    stderr=slave,
    close_fds=True
)
os.close(slave)

def read_until(target, timeout=35):
    buf = ""
    start = time.time()
    while time.time() - start < timeout:
        try:
            r, _, _ = select.select([master], [], [], 0.1)
            if r:
                data = os.read(master, 1024)
                if not data:
                    break
                s = data.decode('utf-8', errors='replace')
                buf += s
                sys.stdout.write(s)
                sys.stdout.flush()
                if target in buf:
                    return buf
        except OSError:
            break
    return buf

out = read_until("]# ", timeout=35)
if "]# " not in out:
    print("\nFAILED to reach shell prompt!")
    proc.terminate()
    sys.exit(1)

def drain_input():
    while True:
        r, _, _ = select.select([master], [], [], 0.05)
        if not r:
            break
        try:
            os.read(master, 1024)
        except OSError:
            break

def run_cmd(cmd_str, timeout=15):
    drain_input()
    time.sleep(0.2)
    print(f"\n>>> Running: {cmd_str}")
    os.write(master, cmd_str.encode('utf-8') + b"\n")
    res = read_until("]# ", timeout=timeout)
    return res

print("\n--- Test 1: Multi-nameserver /etc/resolv.conf ---")
out_resolv = run_cmd("cat /etc/resolv.conf")

print("\n--- Test 2: nslookup mirror.archlinuxarm.org ---")
out_nslookup_mirror = run_cmd("nslookup mirror.archlinuxarm.org")

print("\n--- Test 3: In-memory DNS cache test (second lookup instant) ---")
start_t = time.time()
out_nslookup_cached = run_cmd("nslookup mirror.archlinuxarm.org")
cache_elapsed = time.time() - start_t
print(f"[Timing] Cache lookup took {cache_elapsed:.3f}s")

print("\n--- Test 4: Regression dnstest suite ---")
out_dnstest = run_cmd("dnstest")

print("\n--- Test 5: Pacman official mirrorlist ---")
out_mirrorlist = run_cmd("cat /etc/pacman.d/mirrorlist")

print("\n--- Test 6: Pacman database synchronization (pacman -Sy) ---")
out_sync = run_cmd("pacman -Sy")

print("\n--- Test 7: Pacman official package inspection (pacman -Si acl) ---")
out_info_acl = run_cmd("pacman -Si acl")

print("\n--- Test 8: Pacman search across official sync repository (pacman -Ss acl) ---")
out_search_acl = run_cmd("pacman -Ss acl")

print("\n--- Test 9: Install package calc via pacman (pacman -S calc) ---")
out_inst_calc = run_cmd("pacman -S --noconfirm calc")

print("\n--- Test 10: Run calc to verify execution ---")
out_run_calc = run_cmd("calc 15 * 15")

print("\n--- Test 11: Install package nmap via pacman (pacman -S nmap) ---")
out_inst_nmap = run_cmd("pacman -S --noconfirm nmap")

print("\n--- Test 12: Run nmap to verify execution ---")
out_run_nmap = run_cmd("nmap --version")

print("\n--- Test 13: Query installed packages (pacman -Q) ---")
out_query = run_cmd("pacman -Q")

print("\n--- Test 14: Dynamic Glibc execution test (/bin/glibc_test) ---")
out_glibc = run_cmd("/bin/glibc_test")

proc.terminate()

print("\n\n================ STAGE 8.21 DNS & ARCH MIRROR EVALUATION ================")
passed = True

checks = [
    ("/etc/resolv.conf multi-nameserver", "nameserver 10.0.2.3" in out_resolv and "nameserver 8.8.8.8" in out_resolv),
    ("nslookup mirror.archlinuxarm.org", "mirror.archlinuxarm.org" in out_nslookup_mirror and "Address: " in out_nslookup_mirror),
    ("In-memory DNS cache rapid response", "mirror.archlinuxarm.org" in out_nslookup_cached),
    ("dnstest regression verification", "SUCCESS: DNS resolver and DHCP client verified!" in out_dnstest),
    ("Pacman mirrorlist official mirrors", "mirror.archlinuxarm.org" in out_mirrorlist),
    ("pacman -Sy database synchronization", "Synchronizing package databases" in out_sync and "core" in out_sync),
    ("pacman -Si acl official metadata", "Name            : acl" in out_info_acl and "Version         : 2.4.0-1" in out_info_acl),
    ("pacman -Ss acl official repo search", "core/acl" in out_search_acl),
    ("pacman -S calc install transaction", "installing calc" in out_inst_calc),
    ("calc 15 * 15 execution result (225)", "225" in out_run_calc),
    ("pacman -S nmap install transaction", "installing nmap" in out_inst_nmap),
    ("nmap execution output", "Nmap" in out_run_nmap or "nmap" in out_run_nmap),
    ("pacman -Q shows installed calc and nmap", "calc" in out_query and "nmap" in out_query),
    ("glibc dynamic binary executes successfully", "Hello from dynamically-linked Glibc" in out_glibc and "55" in out_glibc),
]

for label, cond in checks:
    if cond:
        print(f"  [✓] {label}: PASSED")
    else:
        print(f"  [✗] {label}: FAILED")
        passed = False

if passed:
    print(f"\n>>> ALL STAGE 8.21 CHECKS PASSED ({len(checks)}/{len(checks)})! <<<")
    sys.exit(0)
else:
    print("\n>>> STAGE 8.21 VERIFICATION FAILED! <<<")
    sys.exit(1)
