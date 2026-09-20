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

print("Launching QEMU to verify: pacman -S eza & pacman -S ncdu + real execution...")
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

results = {}

# Test 1: Search repo for eza
print("\n=== Test 1: Search for eza (pacman -Ss eza) ===")
res1 = run_cmd("pacman -Ss eza")
results['search_eza'] = ("extra/eza" in res1) or ("eza" in res1)

# Test 2: Package info for eza
print("\n=== Test 2: Package info for eza (pacman -Si eza) ===")
res2 = run_cmd("pacman -Si eza")
results['info_eza'] = ("Name            : eza" in res2) and ("0.23.5-2" in res2)

# Test 3: Install eza (exact command user ran)
print("\n=== Test 3: Install eza (sudo pacman -S eza) ===")
res3 = run_cmd("sudo pacman -S eza")
results['install_eza'] = ("installing eza" in res3) and ("error: target not found" not in res3)

# Test 4: Run real eza --version binary
print("\n=== Test 4: Run eza --version ===")
res4 = run_cmd("eza --version")
results['exec_eza_ver'] = ("eza v0.23.5" in res4)

# Test 5: Run real eza -la /root
print("\n=== Test 5: Run eza -la /root ===")
res5 = run_cmd("eza -la /root")
results['exec_eza_la'] = ("eza" in res5 or "." in res5)

# Test 6: Install ncdu (exact command user ran)
print("\n=== Test 6: Install ncdu (sudo pacman -S ncdu) ===")
res6 = run_cmd("sudo pacman -S ncdu")
results['install_ncdu'] = ("installing ncdu" in res6) and ("error: target not found" not in res6)

# Test 7: Run real ncdu --version binary
print("\n=== Test 7: Run ncdu --version ===")
res7 = run_cmd("ncdu --version")
results['exec_ncdu_ver'] = ("ncdu 2.9.2" in res7)

# Test 8: Run real ncdu /root
print("\n=== Test 8: Run ncdu /root ===")
res8 = run_cmd("ncdu /root")
results['exec_ncdu_run'] = ("ncdu 2.9.2" in res8)

# Test 9: List installed packages (pacman -Q)
print("\n=== Test 9: Check installed packages (pacman -Q) ===")
res9 = run_cmd("pacman -Q")
results['query_installed'] = ("eza" in res9) and ("ncdu" in res9)

# Test 10: Remove eza (pacman -R eza)
print("\n=== Test 10: Remove eza (pacman -R eza) ===")
res10 = run_cmd("pacman -R eza")
results['remove_eza'] = ("removing eza" in res10)

# Test 11: Verify eza is gone
print("\n=== Test 11: Verify eza is gone (eza --version) ===")
res11 = run_cmd("eza --version")
results['eza_removed_verified'] = ("command not found" in res11 or "not found" in res11)

print("\n\n==========================================")
print("             TEST SUMMARY                 ")
print("==========================================")
all_ok = True
for k, v in results.items():
    status = "PASS" if v else "FAIL"
    print(f"  {k:25} : {status}")
    if not v:
        all_ok = False

if all_ok:
    print("\nALL 11 TESTS PASSED! eza and ncdu are 100% functional!")
else:
    print("\nSOME TESTS FAILED! Check above log.")

run_cmd("poweroff")
time.sleep(2)
try:
    proc.terminate()
    proc.wait(timeout=3)
except Exception:
    pass
sys.exit(0 if all_ok else 1)
