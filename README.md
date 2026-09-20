# HimadaOS 2.0

**Custom AArch64/x86_64 Operating System written in Rust (no_std)**

A fully independent, minimalist OS built from scratch — custom kernel, bootloader, userspace shell, and package manager.

---

## Architecture

```
himada-os-aarch64/
├── src/                    # Kernel (AArch64 bare-metal)
│   └── sys/linux_abi.rs   # Linux ABI compatibility layer
├── hello-linux/src/        # Userspace (himada-sh shell + all commands)
│   ├── main.rs             # Shell, command dispatch, exec engine
│   ├── pacman.rs           # Full pacman package manager implementation
│   ├── editor.rs           # Built-in text editor (vim/nano compatible)
│   ├── line_editor.rs      # Readline-like line editor with history
│   └── ...                 # 40+ built-in commands
└── build_isos.sh           # Build script → bootable ISO (ARM64 + x86_64)
```

## Features

- **Custom kernel** — AArch64 + x86_64, bare metal, no libc
- **himada-sh** — Full interactive shell with 80+ built-in commands
- **pacman package manager** — 13,223 real Arch Linux ARM packages
  - Real HTTP downloads from official Arch Linux mirrors
  - TAR archive extraction + ELF binary installation
  - Interactive `[Y/n]` confirmation (real stdin read)
  - `--noconfirm` flag support
  - `pacman -Ss`, `-Si`, `-S`, `-R`, `-Q`, `-Sy` all work
- **Real package execution** — installed binaries replace built-in stubs
- **ELF magic validation** — prevents garbage files from being installed as executables

## Building

```bash
# Requirements: Rust nightly, xorriso, qemu-system-aarch64, limine
./build_isos.sh

# Output:
# ~/Desktop/HimadaOS_Final/himada-os-arm64.iso
# ~/Desktop/HimadaOS_Final/himada-os-x86_64.iso
```

## Testing

```bash
# 16-utility end-to-end QEMU test (65/65 pass)
python3 test_16_utils.py
```

## Verified Packages (QEMU tested)

| Package | Version | Binary |
|---------|---------|--------|
| yazi | 26.9.1-2 | yazi |
| bat | 0.25.0-1 | bat |
| fd | 10.2.0-1 | fd |
| ripgrep | 15.2.0-1 | rg |
| fzf | 0.62.0-1 | fzf |
| jq | 1.7.1-3 | jq |
| htop | 3.3.0-4 | htop |
| neofetch | 7.3.0-3 | neofetch |
| tree | 2.2.1-1 | tree |
| wget | 1.25.0-2 | wget |
| curl | 8.13.0-1 | curl |
| git | 2.49.0-1 | git |
| vim | 9.1.1367-1 | vim |
| nano | 8.4-1 | nano |
| tmux | 3.5a-1 | tmux |
| rsync | 3.5.0-1 | rsync |
