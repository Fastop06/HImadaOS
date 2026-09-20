use core::ptr;

/// # Safety
/// - `src` must be valid for reads of `size_bytes` bytes
#[inline]
pub unsafe fn page_is_zero(src: *const u8, size_bytes: usize) -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            return page_is_zero_avx2(src, size_bytes);
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_feature = "neon")]
        {
            return page_is_zero_neon(src, size_bytes);
        }
    }
    page_is_zero_scalar(src, size_bytes)
}

#[inline(always)]
unsafe fn page_is_zero_scalar(mut src: *const u8, mut size_bytes: usize) -> bool {
    // Check u64 chunks
    while size_bytes >= 8 {
        // SAFETY: Caller guarantees valid pointers for size_bytes.
        let val = ptr::read_unaligned(src as *const u64);
        if val != 0 {
            return false;
        }
        src = src.add(8);
        size_bytes -= 8;
    }
    // Check remaining bytes
    while size_bytes > 0 {
        // SAFETY: Caller guarantees valid pointers.
        if *src != 0 {
            return false;
        }
        src = src.add(1);
        size_bytes -= 1;
    }
    true
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
unsafe fn page_is_zero_avx2(mut src: *const u8, mut size_bytes: usize) -> bool {
    use core::arch::x86_64::*;
    
    let mut acc = _mm256_setzero_si256();
    let mut i = 0;
    
    while size_bytes >= 32 {
        // SAFETY: bounds checked
        let val = _mm256_loadu_si256(src as *const __m256i);
        acc = _mm256_or_si256(acc, val);
        
        i += 1;
        // Early exit check every 8 iterations
        if i % 8 == 0 {
            if _mm256_testz_si256(acc, acc) == 0 {
                return false;
            }
        }
        
        src = src.add(32);
        size_bytes -= 32;
    }
    
    if _mm256_testz_si256(acc, acc) == 0 {
        return false;
    }
    
    page_is_zero_scalar(src, size_bytes)
}

#[cfg(target_arch = "aarch64")]
#[cfg(target_feature = "neon")]
unsafe fn page_is_zero_neon(mut src: *const u8, mut size_bytes: usize) -> bool {
    use core::arch::aarch64::*;
    
    // SAFETY: neon feature is enabled
    let mut acc = vdupq_n_u8(0);
    let mut i = 0;
    
    while size_bytes >= 16 {
        // SAFETY: bounds checked
        let val = vld1q_u8(src);
        acc = vorrq_u8(acc, val);
        
        i += 1;
        if i % 8 == 0 {
            let max = vmaxvq_u8(acc);
            if max != 0 {
                return false;
            }
        }
        
        src = src.add(16);
        size_bytes -= 16;
    }
    
    let max = vmaxvq_u8(acc);
    if max != 0 {
        return false;
    }
    
    page_is_zero_scalar(src, size_bytes)
}
