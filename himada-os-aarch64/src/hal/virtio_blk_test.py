import os

path = "/Users/mussavysegurov/.gemini/antigravity/scratch/himada-os-aarch64/src/hal/virtio_blk.rs"
with open(path, "r") as f: src = f.read()

test_code = """
        // TEST READ/WRITE
        let mut buf = [0u8; 512];
        let test_str = b"Hello from HimadaOS VirtIO Block Device!";
        buf[0..test_str.len()].copy_from_slice(test_str);
        if wrapper.write_sectors(0, 1, &buf).is_ok() {
            crate::serial_println!("[VirtIO Blk] Wrote sector 0 successfully.");
            let mut read_buf = [0u8; 512];
            if wrapper.read_sectors(0, 1, &mut read_buf).is_ok() {
                if let Ok(s) = core::str::from_utf8(&read_buf[0..test_str.len()]) {
                    crate::serial_println!("[VirtIO Blk] Read back: {}", s);
                }
            }
        }
"""

if "TEST READ/WRITE" not in src:
    src = src.replace("crate::serial_println!(\"[Device Manager] Registered VirtIO Block Device! Capacity: {} bytes\", capacity * 512);", "crate::serial_println!(\"[Device Manager] Registered VirtIO Block Device! Capacity: {} bytes\", capacity * 512);" + test_code)
    with open(path, "w") as f: f.write(src)
    print("Injected block test.")
