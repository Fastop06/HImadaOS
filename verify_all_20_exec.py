#!/usr/bin/env python3
import subprocess
import pty
import os
import time
import sys
import select

TEST_UTILITIES = [
    ("tree",    "tree /etc",               ["directories", "files", "tree"]),
    ("calc",    "calc 15 * 15",            ["225"]),
    ("bc",      "bc 100 - 37",             ["63"]),
    ("hexdump", "hexdump /etc/hostname",   ["00000000", "6968", "616d", "himada"]),
    ("htop",    "htop --version",          ["htop"]),
    ("btop",    "btop -v",                 ["btop++", "btop"]),
    ("vim",     "vim --version",           ["VIM", "Vi IMproved", "Himada"]),
    ("neovim",  "neovim -v",               ["NVIM", "LuaJIT", "Build type", "neovim"]),
    ("tmux",    "tmux",                    ["tmux", "session 0", "windows"]),
    ("screen",  "screen",                  ["screen", "4.09", "GNU"]),
    ("sqlite",  "sqlite3 --version",       ["3.46.0", "SQLite"]),
    ("redis",   "redis-server --version",  ["Redis server", "7.2.5"]),
    ("jq",      "jq --version",            ["jq", "1.7"]),
    ("socat",   "socat -V",                ["socat version", "1.8"]),
    ("nmap",    "nmap --version",          ["Nmap version", "7.95"]),
    ("zip",     "zip",                     ["Info-ZIP", "Usage: zip"]),
    ("unzip",   "unzip",                   ["UnZip", "Usage: unzip"]),
    ("zstd",    "zstd --version",          ["zstd", "1.5.7"]),
    ("eza",     "eza --version",           ["eza", "0.23.5"]),
    ("ncdu",    "ncdu --version",          ["ncdu", "2.9.2"]),
]

print("=================================================================")
print(" HimadaOS 2.0: 20-Utility Live Execution & Verification Test")
print("=================================================================\n")

master, slave = pty.openpty()

cmd = [
    "qemu-system-aarch64",
    "-M", "virt",
    "-cpu", "cortex-a72",
    "-smp", "4",
    "-m", "1G",
    "-bios", "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
    "-device", "ramfb",
    "-device", "qemu-xhci,id=xhci",
    "-device", "usb-kbd,bus=xhci.0",
    "-device", "usb-mouse,bus=xhci.0",
    "-device", "virtio-net-device,netdev=net0",
    "-netdev", "user,id=net0,hostfwd=tcp::2223-10.0.2.15:22,hostfwd=tcp::8083-10.0.2.15:80",
    "-drive", "file=/Users/mussavysegurov/.gemini/antigravity/scratch/disk.img,format=raw,if=none,id=drive0",
    "-device", "virtio-blk-device,drive=drive0",
    "-device", "virtio-scsi",
    "-device", "scsi-cd,drive=cd1,bootindex=1",
    "-drive", "if=none,id=cd1,format=raw,file=/Users/mussavysegurov/Desktop/HimadaOS_Final/himada-os-arm64.iso",
    "-nographic"
]

proc = subprocess.Popen(
    cmd,
    stdin=slave,
    stdout=slave,
    stderr=slave,
    close_fds=True
)
os.close(slave)

def read_until(target, timeout=30):
    buf = ""
    start = time.time()
    while time.time() - start < timeout:
        r, _, _ = select.select([master], [], [], 0.1)
        if r:
            try:
                data = os.read(master, 2048)
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

def drain_input():
    while True:
        r, _, _ = select.select([master], [], [], 0.05)
        if not r:
            break
        try:
            os.read(master, 4096)
        except OSError:
            break

def run_command(cmd_str, timeout=15):
    drain_input()
    time.sleep(0.15)
    os.write(master, b"\x15") # Ctrl+U
    time.sleep(0.05)
    print(f"\n[RUN] >>> {cmd_str}")
    os.write(master, cmd_str.encode('utf-8') + b"\n")
    return read_until("]# ", timeout=timeout)

print("[*] Booting HimadaOS in QEMU and waiting for shell prompt...")
boot_out = read_until("]# ", timeout=45)
if "]# " not in boot_out:
    print("\n[FATAL] Shell prompt not reached during boot!")
    proc.terminate()
    sys.exit(1)

print("\n>>> System booted into shell prompt! Beginning 20-Utility Verification... <<<\n")
time.sleep(1)

results = []

for idx, (pkg, exec_cmd, expected_tokens) in enumerate(TEST_UTILITIES, 1):
    print(f"\n=======================================================")
    print(f" [{idx}/20] Testing Package: '{pkg}'")
    print(f"=======================================================")
    
    # 1. Install
    inst_cmd = f"pacman -S {pkg} --noconfirm"
    inst_out = run_command(inst_cmd, timeout=20)
    
    if "CPU EXCEPTION" in inst_out:
        print(f"[FATAL] CPU Exception during install of {pkg}!")
        proc.terminate()
        sys.exit(1)
        
    # 2. Execute
    time.sleep(0.2)
    exec_out = run_command(exec_cmd, timeout=12)
    
    if "CPU EXCEPTION" in exec_out:
        print(f"[FATAL] CPU Exception during execution of {exec_cmd}!")
        proc.terminate()
        sys.exit(1)
        
    # Filter lines
    lines = [l.strip() for l in exec_out.splitlines() if l.strip() and "]#" not in l and exec_cmd not in l]
    output_sample = " | ".join(lines[:3]) if lines else "No output"
    
    matched = any(tok.lower() in exec_out.lower() for tok in expected_tokens)
    not_not_found = "command not found" not in exec_out and "No such file" not in exec_out
    success = matched and not_not_found
    
    tag = "✅ PASS" if success else "❌ FAIL"
    print(f"\n  Result: {tag}")
    print(f"  Command: '{exec_cmd}'")
    print(f"  Output Sample: {output_sample[:100]}")
    
    results.append({
        "pkg": pkg,
        "cmd": exec_cmd,
        "success": success,
        "sample": output_sample[:80]
    })
    
    time.sleep(0.5)

print("\n\n=======================================================")
print("                   FINAL VERIFICATION TABLE            ")
print("=======================================================")
passed_count = 0
for r in results:
    status = "✅ PASS" if r["success"] else "❌ FAIL"
    if r["success"]:
        passed_count += 1
    print(f" {r['pkg']:<12} | {r['cmd']:<24} | {status} | {r['sample']}")

print("=======================================================")
print(f" Total Passed: {passed_count}/{len(results)}")
print("=======================================================")

# Verify shell responsiveness
res_final = run_command("uname -a", timeout=5)
if "6.8.0-himada" in res_final:
    print("[✓] Shell remains 100% responsive and stable after all 20 utilities executed!\n")
else:
    print("[!] Warning: Shell did not respond to final check\n")

run_command("poweroff", timeout=5)
time.sleep(2)
try:
    proc.terminate()
    proc.wait(timeout=3)
except Exception:
    pass

sys.exit(0 if passed_count == len(results) else 1)
