#!/usr/bin/env python3
"""
HimadaOS — 16-utility QEMU integration test
Tests: yazi, bat, fd, ripgrep, fzf, jq, htop, neofetch, tree, wget,
       curl, git, vim, nano, tmux, rsync
Each utility: install → verify real execution (not UI refresh) → remove
"""
import subprocess, pty, os, time, sys, select

QEMU_ISO = "/Users/mussavysegurov/Desktop/HimadaOS_Final/himada-os-arm64.iso"
DISK_IMG  = "/Users/mussavysegurov/.gemini/antigravity/scratch/disk.img"

UTILITIES = [
    # (pkg_name, bin_name, version_cmd_str, expected_in_output)
    # yazi is 9.5 MB so needs more download time → handled with longer install timeout
    ("yazi",    "yazi",    "yazi --version",     ["yazi"]),
    ("bat",     "bat",     "bat --version",      ["bat"]),
    ("fd",      "fd",      "fd --version",       ["fd"]),
    # ripgrep's binary is 'rg', not 'ripgrep'
    ("ripgrep", "rg",      "rg --version",       ["ripgrep", "rg"]),
    ("fzf",     "fzf",     "fzf --version",      ["fzf"]),
    ("jq",      "jq",      "jq --version",       ["jq"]),
    ("htop",    "htop",    "htop --version",     ["htop"]),
    ("neofetch","neofetch","neofetch --version",  ["neofetch"]),
    ("tree",    "tree",    "tree --version",     ["tree"]),
    ("wget",    "wget",    "wget --version",     ["GNU Wget", "wget"]),
    ("curl",    "curl",    "curl --version",     ["curl"]),
    ("git",     "git",     "git --version",      ["git"]),
    # vim: pass -es to non-interactive mode or use echo :q | vim --version
    # send 'q!' after version output
    ("vim",     "vim",     "vim --version",      ["VIM", "vim"]),
    # nano: --version exits immediately without interactive mode
    ("nano",    "nano",    "nano --version",     ["nano"]),
    ("tmux",    "tmux",    "tmux -V",            ["tmux"]),
    ("rsync",   "rsync",   "rsync --version",    ["rsync"]),
]

# ── Launch QEMU ────────────────────────────────────────────────────────────────
master, slave = pty.openpty()
cmd = [
    "qemu-system-aarch64",
    "-machine", "virt",
    "-cpu", "cortex-a72",
    "-m", "1024M",
    "-bios", "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
    "-cdrom", QEMU_ISO,
    "-drive", f"file={DISK_IMG},format=raw,if=virtio",
    "-device", "virtio-net-device,netdev=net0",
    "-netdev", "user,id=net0",
    "-nographic",
]
print("🚀 Launching QEMU (HimadaOS ARM64) — 16-utility stress test…")
proc = subprocess.Popen(cmd, stdin=slave, stdout=slave, stderr=slave, close_fds=True)
os.close(slave)

# ── I/O helpers ───────────────────────────────────────────────────────────────
def read_until(target: str, timeout: float = 45) -> str:
    buf = ""
    start = time.time()
    while time.time() - start < timeout:
        try:
            r, _, _ = select.select([master], [], [], 0.1)
            if r:
                data = os.read(master, 4096)
                if not data:
                    break
                s = data.decode("utf-8", errors="replace")
                buf += s
                sys.stdout.write(s)
                sys.stdout.flush()
                if target in buf:
                    return buf
        except OSError:
            break
    return buf

def drain():
    while True:
        r, _, _ = select.select([master], [], [], 0.05)
        if not r:
            break
        try:
            os.read(master, 4096)
        except OSError:
            break

def run_cmd(cmd_str: str, timeout: float = 20, stdin_reply: str = "") -> str:
    drain()
    time.sleep(0.15)
    print(f"\n>>> {cmd_str}")
    os.write(master, cmd_str.encode() + b"\n")
    if stdin_reply:
        # Wait for the prompt character then send reply
        read_until("[Y/n]", timeout=10)
        time.sleep(0.1)
        os.write(master, stdin_reply.encode() + b"\n")
    return read_until("]# ", timeout=timeout)

# ── Wait for boot ─────────────────────────────────────────────────────────────
print("\n⏳ Booting… (waiting for shell prompt)")
boot_out = read_until("]# ", timeout=45)
if "]# " not in boot_out:
    print("\n❌ FAILED: Did not reach shell prompt after 45 s")
    proc.terminate()
    sys.exit(1)
print("\n✅ Shell prompt reached!")

# ── Results tracking ──────────────────────────────────────────────────────────
results: dict[str, tuple[bool, str]] = {}   # name → (pass, reason)

def record(name: str, ok: bool, reason: str = ""):
    results[name] = (ok, reason)
    tag = "✅ PASS" if ok else "❌ FAIL"
    print(f"  {tag}  {name}" + (f"  ({reason})" if reason else ""))

# ── Bonus: verify interactive confirmation before real installs ────────────────
print("\n\n════════════════════════════════════════════")
print(" BONUS: Interactive confirmation (type n)")
print("════════════════════════════════════════════")
# Send pacman -S eza without --noconfirm, reply 'n'
drain()
time.sleep(0.15)
os.write(master, b"pacman -S eza\n")
prompt_out = read_until("[Y/n]", timeout=20)
got_prompt = "[Y/n]" in prompt_out
if got_prompt:
    time.sleep(0.1)
    os.write(master, b"n\n")
    abort_out = read_until("]# ", timeout=10)
    abort_ok = "Aborting" in abort_out or "aborting" in abort_out
    record("interactive_n_aborts", abort_ok, "typed 'n', expected Aborting")
else:
    record("interactive_n_aborts", False, "prompt [Y/n] never appeared")

# ── Main 16-utility loop ──────────────────────────────────────────────────────
print("\n\n════════════════════════════════════════════")
print(" Installing & verifying 16 utilities")
print("════════════════════════════════════════════")

for pkg, bin_name, version_cmd, expect_words in UTILITIES:
    print(f"\n──── {pkg.upper()} ────────────────────────────────")
    # yazi is 9.5 MB — give it more time to download
    install_timeout = 90 if pkg == "yazi" else 35

    # 1. Search
    s_out = run_cmd(f"pacman -Ss {pkg}", timeout=15)
    found_in_search = pkg in s_out
    record(f"{pkg}_search", found_in_search, "pkg in pacman -Ss output")

    # 2. Install (--noconfirm to avoid blocking)
    i_out = run_cmd(f"pacman -S --noconfirm {pkg}", timeout=install_timeout)
    install_ok = (f"installing {pkg}" in i_out and "error: target not found" not in i_out)
    record(f"{pkg}_install", install_ok, "installing line present, no target-not-found")

    # 3. Execute — check it's a REAL binary (not UI refresh)
    e_out = run_cmd(version_cmd, timeout=12)

    # "UI refresh" means we got another prompt WITHOUT any tool output
    stripped = e_out.strip()
    lines = [l for l in stripped.splitlines() if bin_name not in l]
    content_lines = [l.strip() for l in lines if l.strip() and "]#" not in l]
    has_real_output = len(content_lines) > 0
    matches_expected = any(w.lower() in e_out.lower() for w in expect_words)
    not_a_stub = "command not found" not in e_out and "not found" not in e_out

    exec_ok = install_ok and has_real_output and not_a_stub
    record(f"{pkg}_exec", exec_ok,
           "real output" if exec_ok else
           ("command not found" if not not_a_stub else
            "no output / UI refresh"))

    # 4. Remove
    r_out = run_cmd(f"pacman -R --noconfirm {pkg}", timeout=15)
    remove_ok = f"removing {pkg}" in r_out
    record(f"{pkg}_remove", remove_ok, "removing line present")

# ── Summary ───────────────────────────────────────────────────────────────────
print("\n\n════════════════════════════════════════════")
print("              FINAL SUMMARY                ")
print("════════════════════════════════════════════")

categories: dict[str, list] = {
    "bonus":   [],
    "search":  [],
    "install": [],
    "exec":    [],
    "remove":  [],
}
for key, (ok, reason) in results.items():
    for cat in categories:
        if key.endswith(cat) or key == "interactive_n_aborts":
            categories["bonus" if "bonus" in key or key == "interactive_n_aborts" else cat].append((key, ok, reason))
            break
    else:
        categories["exec"].append((key, ok, reason))

total = len(results)
passed = sum(1 for ok, _ in results.values() if ok)

for cat, items in categories.items():
    if not items:
        continue
    cat_pass = sum(1 for _, ok, _ in items if ok)
    print(f"\n  [{cat.upper()}]  {cat_pass}/{len(items)}")
    for key, ok, reason in items:
        tag = "✅" if ok else "❌"
        print(f"    {tag} {key:<30} {reason}")

print(f"\n  TOTAL: {passed}/{total} passed", end="")
if passed == total:
    print("  🎉 ALL PASS!")
else:
    print(f"  ⚠️  {total-passed} FAILED")

run_cmd("poweroff")
time.sleep(2)
try:
    proc.terminate()
    proc.wait(timeout=3)
except Exception:
    pass

sys.exit(0 if passed == total else 1)
