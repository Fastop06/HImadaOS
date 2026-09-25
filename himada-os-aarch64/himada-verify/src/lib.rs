// HimadaOS 2.0 Formal Verification Suite (Kani SMT/Model Checker)

pub fn format_u64_padded(val: u64, width: usize, buf: &mut [u8; 20]) -> usize {
    let mut v = val;
    let mut len = 0usize;
    if v == 0 {
        buf[19] = b'0';
        len = 1;
    } else {
        while v > 0 {
            buf[19 - len] = b'0' + (v % 10) as u8;
            v /= 10;
            len += 1;
        }
    }
    let pad = if width > len { width - len } else { 0 };
    len + pad
}

pub fn resolve_path(cwd: &str, target: &str, out: &mut [u8; 128]) -> usize {
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

// ─────────────────────────────────────────────────────────────
// PipeRingBuffer Model for Verification
// ─────────────────────────────────────────────────────────────
pub const TEST_PIPE_CAPACITY: usize = 16;

pub struct VerifPipeRingBuffer {
    pub buffer: [u8; TEST_PIPE_CAPACITY],
    pub read_pos: usize,
    pub write_pos: usize,
    pub count: usize,
    pub readers: usize,
    pub writers: usize,
}

impl VerifPipeRingBuffer {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; TEST_PIPE_CAPACITY],
            read_pos: 0,
            write_pos: 0,
            count: 0,
            readers: 1,
            writers: 1,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        if self.readers == 0 {
            return usize::MAX; // Broken pipe (EPIPE)
        }
        let space = TEST_PIPE_CAPACITY - self.count;
        let to_write = if data.len() < space { data.len() } else { space };
        for i in 0..to_write {
            self.buffer[self.write_pos] = data[i];
            self.write_pos = (self.write_pos + 1) % TEST_PIPE_CAPACITY;
        }
        self.count += to_write;
        to_write
    }

    pub fn read(&mut self, out: &mut [u8]) -> usize {
        let to_read = if self.count < out.len() { self.count } else { out.len() };
        for i in 0..to_read {
            out[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % TEST_PIPE_CAPACITY;
        }
        self.count -= to_read;
        to_read
    }
}

// ─────────────────────────────────────────────────────────────
// Pattern Match / Grep Bounds Model
// ─────────────────────────────────────────────────────────────
pub fn grep_contains(line: &[u8], pat: &[u8], case_insens: bool) -> bool {
    if pat.is_empty() { return true; }
    if line.len() < pat.len() { return false; }
    
    for i in 0..=line.len() - pat.len() {
        let mut ok = true;
        for j in 0..pat.len() {
            let a = if case_insens { line[i + j].to_ascii_lowercase() } else { line[i + j] };
            let b = if case_insens { pat[j].to_ascii_lowercase() } else { pat[j] };
            if a != b {
                ok = false;
                break;
            }
        }
        if ok { return true; }
    }
    false
}

// ─────────────────────────────────────────────────────────────
// Mirror URL Parsing Model for Pacman
// ─────────────────────────────────────────────────────────────
pub fn parse_mirror_url(tmpl: &str, arch: &str, repo: &str, out: &mut [u8; 128]) -> usize {
    let mut cur = 0;
    let mb = tmpl.as_bytes();
    let mut mi = 0;
    while mi < mb.len() && cur < 127 {
        if mi + 5 <= mb.len() && &mb[mi..mi+5] == b"$arch" {
            let c = arch.len().min(127 - cur);
            out[cur..cur+c].copy_from_slice(&arch.as_bytes()[..c]);
            cur += c;
            mi += 5;
        } else if mi + 5 <= mb.len() && &mb[mi..mi+5] == b"$repo" {
            let c = repo.len().min(127 - cur);
            out[cur..cur+c].copy_from_slice(&repo.as_bytes()[..c]);
            cur += c;
            mi += 5;
        } else {
            out[cur] = mb[mi];
            cur += 1;
            mi += 1;
        }
    }
    cur
}

// ─────────────────────────────────────────────────────────────
// Pacman Index Line Parsing Model
// ─────────────────────────────────────────────────────────────
pub fn parse_pacman_index_line(line: &[u8], out_name: &mut [u8; 32], out_file: &mut [u8; 64]) -> bool {
    let mut part = 0;
    let mut nlen = 0;
    let mut flen = 0;
    for &b in line {
        if b == b'|' {
            part += 1;
            continue;
        }
        if part == 0 && nlen < 31 {
            out_name[nlen] = b;
            nlen += 1;
        } else if part == 3 && flen < 63 {
            out_file[flen] = b;
            flen += 1;
        }
    }
    out_name[nlen] = 0;
    out_file[flen] = 0;
    part >= 3 && nlen > 0
}

// ─────────────────────────────────────────────────────────────
// HTTP Redirect Bound Model
// ─────────────────────────────────────────────────────────────
pub fn redirect_step(count: usize, max_redirects: usize) -> Option<usize> {
    if count >= max_redirects {
        None
    } else {
        Some(count + 1)
    }
}

// ─────────────────────────────────────────────────────────────
// ELF Segment Invariants Model
// ─────────────────────────────────────────────────────────────
pub fn check_elf_segment_bounds(vaddr: u64, memsz: u64, max_addr: u64) -> bool {
    if let Some(end) = vaddr.checked_add(memsz) {
        end <= max_addr
    } else {
        false
    }
}

#[cfg(kani)]
mod tests {
    use super::*;

    #[kani::proof]
    #[kani::unwind(21)]
    fn verify_format_u64_padded() {
        let val: u64 = kani::any();
        let width: usize = kani::any();
        kani::assume(width <= 20);

        let mut buf = [0u8; 20];
        let total_len = format_u64_padded(val, width, &mut buf);
        
        assert!(total_len >= 1);
        assert!(total_len >= width);
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn verify_resolve_path() {
        let cwd_len: usize = kani::any();
        let target_len: usize = kani::any();
        kani::assume(cwd_len <= 10 && target_len <= 10);

        let mut cwd_buf = [0u8; 10];
        let mut target_buf = [0u8; 10];
        for i in 0..cwd_len { cwd_buf[i] = kani::any(); }
        for i in 0..target_len { target_buf[i] = kani::any(); }

        if let (Ok(c), Ok(t)) = (core::str::from_utf8(&cwd_buf[..cwd_len]), core::str::from_utf8(&target_buf[..target_len])) {
            let mut out = [0u8; 128];
            let res = resolve_path(c, t, &mut out);
            assert!(res <= 128);
        }
    }

    #[kani::proof]
    #[kani::unwind(18)]
    fn verify_pipe_ring_buffer_invariants() {
        let mut pipe = VerifPipeRingBuffer::new();
        
        let write_len: usize = kani::any();
        kani::assume(write_len <= 10);
        let mut write_data = [0u8; 10];
        for i in 0..write_len { write_data[i] = kani::any(); }

        let written = pipe.write(&write_data[..write_len]);
        assert!(written <= write_len);
        assert!(pipe.count <= TEST_PIPE_CAPACITY);
        assert!(pipe.write_pos < TEST_PIPE_CAPACITY);

        let read_buf_len: usize = kani::any();
        kani::assume(read_buf_len <= 10);
        let mut read_buf = [0u8; 10];
        
        let read = pipe.read(&mut read_buf[..read_buf_len]);
        assert!(read <= read_buf_len);
        assert!(read <= written);
        assert!(pipe.count <= TEST_PIPE_CAPACITY);
        assert!(pipe.read_pos < TEST_PIPE_CAPACITY);
    }

    #[kani::proof]
    #[kani::unwind(10)]
    fn verify_grep_bounds_and_safety() {
        let line_len: usize = kani::any();
        let pat_len: usize = kani::any();
        kani::assume(line_len <= 6 && pat_len <= 4);

        let mut line = [0u8; 6];
        let mut pat = [0u8; 4];
        for i in 0..line_len { line[i] = kani::any(); }
        for i in 0..pat_len { pat[i] = kani::any(); }

        let case_insens: bool = kani::any();

        let _ = grep_contains(&line[..line_len], &pat[..pat_len], case_insens);
    }

    #[kani::proof]
    #[kani::unwind(16)]
    fn verify_mirror_url_parse_safety() {
        let tmpl_len: usize = kani::any();
        kani::assume(tmpl_len <= 15);
        let mut tmpl_buf = [0u8; 15];
        for i in 0..tmpl_len { tmpl_buf[i] = kani::any(); }

        if let Ok(tmpl) = core::str::from_utf8(&tmpl_buf[..tmpl_len]) {
            let mut out = [0u8; 128];
            let len = parse_mirror_url(tmpl, "aarch64", "extra", &mut out);
            assert!(len <= 127);
        }
    }

    #[kani::proof]
    #[kani::unwind(20)]
    fn verify_pacman_index_delimiter_safety() {
        let line_len: usize = kani::any();
        kani::assume(line_len <= 18);
        let mut line_buf = [0u8; 18];
        for i in 0..line_len { line_buf[i] = kani::any(); }

        let mut out_name = [0u8; 32];
        let mut out_file = [0u8; 64];
        let _ = parse_pacman_index_line(&line_buf[..line_len], &mut out_name, &mut out_file);
    }

    #[kani::proof]
    fn verify_redirect_termination() {
        let count: usize = kani::any();
        let max_red: usize = 3;
        let next = redirect_step(count, max_red);
        if count >= max_red {
            assert!(next.is_none());
        } else {
            assert_eq!(next, Some(count + 1));
        }
    }

    #[kani::proof]
    fn verify_elf_segment_bounds_safety() {
        let vaddr: u64 = kani::any();
        let memsz: u64 = kani::any();
        let max_addr: u64 = 0x8000_0000_0000;
        let ok = check_elf_segment_bounds(vaddr, memsz, max_addr);
        if ok {
            assert!(vaddr <= max_addr);
            assert!(vaddr + memsz <= max_addr);
        }
    }
}


