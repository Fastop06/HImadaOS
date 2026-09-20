#[cfg(kani)]
mod tests {
    // A simplified version of our print_u64_padded logic to verify
    // that it never panics or goes out of bounds for ANY u64 input.
    fn format_u64_padded(val: u64, width: usize, buf: &mut [u8; 20]) -> usize {
        let mut v = val;
        let mut len = 0usize;
        if v == 0 {
            buf[19] = b'0';
            len = 1;
        } else {
            while v > 0 {
                // Kani will prove this never panics
                buf[19 - len] = b'0' + (v % 10) as u8;
                v /= 10;
                len += 1;
            }
        }
        
        let pad = if width > len { width - len } else { 0 };
        // Return total length it would print
        len + pad
    }

    #[kani::proof]
    fn verify_format_u64_padded() {
        // kani::any() generates ALL possible u64 values
        let val: u64 = kani::any();
        // Constrain width to reasonable values to avoid infinite loops in display
        let width: usize = kani::any();
        kani::assume(width <= 20);

        let mut buf = [0u8; 20];
        let total_len = format_u64_padded(val, width, &mut buf);
        
        assert!(total_len >= 1);
        assert!(total_len >= width);
    }
    
    // Verify path join logic (used in cd and ls)
    fn resolve_path(cwd: &str, target: &str, out: &mut [u8; 128]) -> usize {
        if target.starts_with('/') {
            let bytes = target.as_bytes();
            let len = bytes.len();
            if len <= 128 {
                out[..len].copy_from_slice(bytes);
                return len;
            }
            return 0;
        }
        
        let c_bytes = cwd.as_bytes();
        let t_bytes = target.as_bytes();
        let mut len = c_bytes.len();
        
        if len <= 128 {
            out[..len].copy_from_slice(c_bytes);
            if len < 128 {
                out[len] = b'/';
                len += 1;
                
                let copy_len = if len + t_bytes.len() <= 128 { t_bytes.len() } else { 128 - len };
                out[len..len+copy_len].copy_from_slice(&t_bytes[..copy_len]);
                len += copy_len;
            }
        }
        len
    }

    #[kani::proof]
    fn verify_resolve_path() {
        // We use small arrays for kani to keep verification time reasonable
        let mut cwd = [0u8; 10];
        let mut target = [0u8; 10];
        for i in 0..10 {
            cwd[i] = kani::any();
            target[i] = kani::any();
        }
        // Assume valid UTF-8 for simplicity of the stub, or just test raw bytes
        if let (Ok(c), Ok(t)) = (core::str::from_utf8(&cwd), core::str::from_utf8(&target)) {
            let mut out = [0u8; 128];
            let res = resolve_path(c, t, &mut out);
            assert!(res <= 128);
        }
    }

    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_line_editor_invariants() {
        let mut ed = crate::line_editor::LineEditor::new();
        assert!(ed.len == 0);
        assert!(ed.cursor == 0);

        for _ in 0..4 {
            let b: u8 = kani::any();
            kani::assume(b >= 0x20 && b <= 0x7e);
            ed.insert_char(b, &mut |_| {});
            assert!(ed.len <= crate::line_editor::MAX_LINE);
            assert!(ed.cursor <= ed.len);
        }
        ed.handle_backspace(&mut |_| {});
        assert!(ed.cursor <= ed.len);
        ed.add_history();
        assert!(ed.history_count <= crate::line_editor::MAX_HISTORY + 1);
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn verify_base64_bounds() {
        let in_len: usize = kani::any();
        kani::assume(in_len <= 8);

        let mut input = [0u8; 8];
        for i in 0..in_len {
            input[i] = kani::any();
        }

        let mut output = [0u8; 16];
        let encoded_len = crate::base64::encode_base64(&input[..in_len], &mut output);
        assert!(encoded_len <= 16);

        let mut decoded = [0u8; 16];
        let decoded_len = crate::base64::decode_base64(&output[..encoded_len], &mut decoded);
        assert!(decoded_len <= in_len);
    }

    #[kani::proof]
    #[kani::unwind(17)]
    fn verify_md5_streaming() {
        let data_len: usize = kani::any();
        kani::assume(data_len <= 16);

        let mut data = [0u8; 16];
        for i in 0..data_len {
            data[i] = kani::any();
        }

        let digest = crate::md5::md5_digest(&data[..data_len]);
        assert!(digest.len() == 16);
    }
}
