#!/usr/bin/env python3
import subprocess
import pty
import os
import time
import sys
import select
import urllib.request

master, slave = pty.openpty()

cmd = [
    "qemu-system-aarch64",
    "-M", "virt",
    "-cpu", "cortex-a72",
    "-smp", "4",
    "-m", "2G",
    "-bios", "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
    "-device", "ramfb",
    "-device", "qemu-xhci,id=xhci",
    "-device", "usb-kbd,bus=xhci.0",
    "-device", "usb-mouse,bus=xhci.0",
    "-device", "virtio-net-device,netdev=net0",
    "-netdev", "user,id=net0,hostfwd=tcp::2223-10.0.2.15:22,hostfwd=tcp::8080-10.0.2.15:80,hostfwd=tcp::8083-10.0.2.15:80",
    "-drive", "file=/Users/mussavysegurov/.gemini/antigravity/scratch/disk.img,format=raw,if=none,id=drive0",
    "-device", "virtio-blk-device,drive=drive0",
    "-device", "virtio-scsi",
    "-device", "scsi-cd,drive=cd1,bootindex=1",
    "-drive", "if=none,id=cd1,format=raw,file=/Users/mussavysegurov/Desktop/HimadaOS_Final/himada-os-arm64.iso",
    "-nographic"
]

proc = subprocess.Popen(cmd, stdin=slave, stdout=slave, stderr=slave, close_fds=True)
os.close(slave)

serial_log = []

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
                serial_log.append(s)
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
            d = os.read(master, 4096)
            serial_log.append(d.decode('utf-8', errors='replace'))
        except OSError:
            break

def run_command(cmd_str, timeout=15):
    drain_input()
    time.sleep(0.1)
    os.write(master, b"\x15")
    time.sleep(0.05)
    print(f"\n[RUN] >>> {cmd_str}")
    os.write(master, cmd_str.encode('utf-8') + b"\n")
    return read_until("]# ", timeout=timeout)

print("[*] Waiting for shell prompt...")
boot_out = read_until("]# ", timeout=120)
if "]# " not in boot_out:
    print("[FATAL] Boot failed!")
    proc.terminate()
    sys.exit(1)

print("[*] Creating /var/www/localhost/htdocs/index.html...")
run_command("mkdir -p /var/www/localhost/htdocs")
run_command("echo '<h1>HimadaOS 2.0 Web Server</h1><p>Online</p>' > /var/www/localhost/htdocs/index.html")
run_command("ls -la /var/www/localhost/htdocs")

print("\n[*] Starting 20 sequential HTTP requests...")
successes = 0
for i in range(1, 21):
    url = "http://127.0.0.1:8080/api/stats" if i % 2 == 0 else "http://127.0.0.1:8080/"
    t0 = time.time()
    try:
        with urllib.request.urlopen(url, timeout=3) as resp:
            data = resp.read()
            dt = time.time() - t0
            print(f"[{i:02d}/20] {url} -> HTTP {resp.status} in {dt:.3f}s ({len(data)} bytes)")
            successes += 1
    except Exception as e:
        dt = time.time() - t0
        print(f"[{i:02d}/20] {url} -> FAILED in {dt:.3f}s: {e}")
        # Print serial output since last read
        drain_input()
        recent = "".join(serial_log[-20:])
        print("--- RECENT SERIAL ---")
        print(recent[-1000:])
        print("---------------------")
    time.sleep(0.1)

print(f"\n[*] Result: {successes}/20 successful requests.")
proc.terminate()
sys.exit(0 if successes == 20 else 1)
