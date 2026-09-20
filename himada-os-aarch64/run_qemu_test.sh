#!/bin/bash
set -e

cd /Users/mussavysegurov/.gemini/antigravity/scratch

./build_isos.sh > /dev/null
cd himada-os-aarch64

if [ ! -f disk.img ]; then
    qemu-img create -f raw disk.img 64M
fi

qemu-system-aarch64 \
    -M virt \
    -cpu cortex-a72 \
    -m 1G \
    -bios /opt/homebrew/share/qemu/edk2-aarch64-code.fd \
    -serial stdio \
    -device ramfb \
    -device qemu-xhci,id=xhci \
    -device usb-kbd,bus=xhci.0 \
    -netdev user,id=net0,hostfwd=tcp::8081-:80 -device virtio-net-device,netdev=net0 \
    -drive file=disk.img,format=raw,if=none,id=drive0 -device virtio-blk-device,drive=drive0 \
    -device virtio-scsi \
    -device scsi-cd,drive=cd1,bootindex=1 \
    -drive if=none,id=cd1,format=raw,file=/Users/mussavysegurov/Desktop/HimadaOS_Final/himada-os-arm64.iso \
    -display cocoa \
    &

QEMU_PID=$!
sleep 60
kill $QEMU_PID || true
