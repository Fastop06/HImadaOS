use core::ptr;

/// Returns the byte offset of the first difference, or None if blocks are identical.
/// 
/// # Safety  
/// - `a` and `b` must be valid for reads of `size_bytes` bytes
#[inline]
pub unsafe fn mem_diff(a: *const u8, b: *const u8, size_bytes: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            return mem_diff_avx2(a, b, size_bytes);
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_feature = "neon")]
        {
            return mem_diff_neon(a, b, size_bytes);
        }
    }
    mem_diff_scalar(a, b, size_bytes)
}

#[inline(always)]
unsafe fn mem_diff_scalar(mut a: *const u8, mut b: *const u8, mut size_bytes: usize) -> Option<usize> {
    let mut offset = 0;
    
    while size_bytes >= 8 {
        // SAFETY: Caller guarantees valid pointers.
        let val_a = ptr::read_unaligned(a as *const u64);
        let val_b = ptr::read_unaligned(b as *const u64);
        
        if val_a != val_b {
            // Find exact byte
            for i in 0..8 {
                // SAFETY: Still within bounds
                if *a.add(i) != *b.add(i) {
                    return Some(offset + i);
                }
            }
        }
        
        a = a.add(8);
        b = b.add(8);
        size_bytes -= 8;
        offset += 8;
    }
    
    while size_bytes > 0 {
        // SAFETY: Valid bounds.
        if *a != *b {
            return Some(offset);
        }
        a = a.add(1);
        b = b.add(1);
        size_bytes -= 1;
        offset += 1;
    }
    
    None
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
unsafe fn mem_diff_avx2(mut a: *const u8, mut b: *const u8, mut size_bytes: usize) -> Option<usize> {
    use core::arch::x86_64::*;
    
    let mut offset = 0;
    
    while size_bytes >= 32 {
        // SAFETY: Bounds verified
        let va = _mm256_loadu_si256(a as *const __m256i);
        let vb = _mm256_loadu_si256(b as *const __m256i);
        
        let cmp = _mm256_cmpeq_epi8(va, vb);
        let mask = _mm256_movemask_epi8(cmp) as u32;
        
        if mask != 0xFFFF_FFFF {
            let diff_mask = !mask;
            let first_diff = diff_mask.trailing_zeros() as usize;
            return Some(offset + first_diff);
        }
        
        a = a.add(32);
        b = b.add(32);
        size_bytes -= 32;
        offset += 32;
    }
    
    mem_diff_scalar(a, b, size_bytes).map(|rem| offset + rem)
}

#[cfg(target_arch = "aarch64")]
#[cfg(target_feature = "neon")]
unsafe fn mem_diff_neon(mut a: *const u8, mut b: *const u8, mut size_bytes: usize) -> Option<usize> {
    use core::arch::aarch64::*;
    
    let mut offset = 0;
    
    while size_bytes >= 16 {
        // SAFETY: Bounds verified, neon feature enabled
        let va = vld1q_u8(a);
        let vb = vld1q_u8(b);
        
        let cmp = vceqq_u8(va, vb);
        
        // Find if any elements are NOT equal by extracting min value across elements.
        // Wait, vceqq gives 0xFF for equal, 0x00 for not equal.
        let min = vminvq_u8(cmp);
        
        if min == 0 {
            // Find exact byte
            for i in 0..16 {
                // SAFETY: We are within bounds of the 16 bytes we just checked.
                if *a.add(i) != *b.add(i) {
                    return Some(offset + i);
                }
            }
        }
        
        a = a.add(16);
        b = b.add(16);
        size_bytes -= 16;
        offset += 16;
    }
    
    mem_diff_scalar(a, b, size_bytes).map(|rem| offset + rem)
}
