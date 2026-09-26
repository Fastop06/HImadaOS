#!/usr/bin/env python3
import subprocess
import pty
import os
import time
import sys
import select
import urllib.request

print("Starting diagnostic QEMU session...")
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

print("[*] Waiting for shell prompt...")
boot_out = read_until("]# ", timeout=90)
if "]# " not in boot_out:
    print("[FAIL] Shell prompt not reached!")
    proc.terminate()
    sys.exit(1)

print("\n[*] Shell prompt reached! Creating test index.html...")
time.sleep(0.5)

# Deploy index.html
os.write(master, b"mkdir -p /var/www/localhost/htdocs\n")
read_until("]# ", 10)
os.write(master, b"echo '<h1>Hello HimadaOS Web</h1>' > /var/www/localhost/htdocs/index.html\n")
read_until("]# ", 10)
os.write(master, b"ls -la /var/www/localhost/htdocs\n")
read_until("]# ", 10)

print("\n[*] Testing Request 1 (immediately after creating file)...")
time.sleep(1)
try:
    resp = urllib.request.urlopen("http://127.0.0.1:8080/", timeout=5)
    print("Request 1 SUCCESS:", resp.status, resp.read().decode('utf-8')[:60])
except Exception as e:
    print("Request 1 FAILED:", e)

print("\n[*] Waiting 5 seconds while VM is idle in shell loop...")
time.sleep(5)

print("\n[*] Testing Request 2 (after 5s idle)...")
try:
    resp = urllib.request.urlopen("http://127.0.0.1:8080/", timeout=5)
    print("Request 2 SUCCESS:", resp.status, resp.read().decode('utf-8')[:60])
except Exception as e:
    print("Request 2 FAILED:", e)

print("\n[*] Waiting 5 seconds again...")
time.sleep(5)

print("\n[*] Testing Request 3 (after another 5s idle)...")
try:
    resp = urllib.request.urlopen("http://127.0.0.1:8080/", timeout=5)
    print("Request 3 SUCCESS:", resp.status, resp.read().decode('utf-8')[:60])
except Exception as e:
    print("Request 3 FAILED:", e)

print("\n[*] Terminating test session...")
proc.terminate()
proc.wait()
print("[*] Done!")
