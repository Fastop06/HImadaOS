use core::ptr;

const NT_THRESHOLD: usize = 2 * 1024 * 1024;

/// # Safety
/// - `dst` must be valid for writes of `size_bytes` bytes
/// - `dst` must be properly aligned for at least u8 access
#[inline]
pub unsafe fn page_zero(dst: *mut u8, size_bytes: usize) {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            if size_bytes >= NT_THRESHOLD {
                return page_zero_avx2_nt(dst, size_bytes);
            } else {
                return page_zero_avx2(dst, size_bytes);
            }
        } 
        #[cfg(target_feature = "sse2")]
        {
            return page_zero_sse2(dst, size_bytes);
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_feature = "neon")]
        {
            return page_zero_neon(dst, size_bytes);
        }
    }
    page_zero_scalar(dst, size_bytes)
}

#[inline(always)]
unsafe fn page_zero_scalar(dst: *mut u8, size_bytes: usize) {
    // SAFETY: The caller guarantees dst is valid and sized correctly.
    ptr::write_bytes(dst, 0, size_bytes);
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "sse2")]
unsafe fn page_zero_sse2(mut dst: *mut u8, mut size_bytes: usize) {
    use core::arch::x86_64::*;
    
    // Prologue: align to 16 bytes
    while (dst as usize) % 16 != 0 && size_bytes > 0 {
        // SAFETY: In bounds per caller
        *dst = 0;
        dst = dst.add(1);
        size_bytes -= 1;
    }

    let zeros = _mm_setzero_si128();
    while size_bytes >= 64 {
        // SAFETY: Aligned to 16, within bounds
        _mm_store_si128(dst as *mut __m128i, zeros);
        _mm_store_si128(dst.add(16) as *mut __m128i, zeros);
        _mm_store_si128(dst.add(32) as *mut __m128i, zeros);
        _mm_store_si128(dst.add(48) as *mut __m128i, zeros);
        dst = dst.add(64);
        size_bytes -= 64;
    }
    
    // Epilogue
    page_zero_scalar(dst, size_bytes);
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
unsafe fn page_zero_avx2(mut dst: *mut u8, mut size_bytes: usize) {
    use core::arch::x86_64::*;
    
    // Prologue: align to 32 bytes
    while (dst as usize) % 32 != 0 && size_bytes > 0 {
        // SAFETY: In bounds
        *dst = 0;
        dst = dst.add(1);
        size_bytes -= 1;
    }

    let zeros = _mm256_setzero_si256();
    while size_bytes >= 128 {
        // SAFETY: Aligned to 32, within bounds
        _mm256_store_si256(dst as *mut __m256i, zeros);
        _mm256_store_si256(dst.add(32) as *mut __m256i, zeros);
        _mm256_store_si256(dst.add(64) as *mut __m256i, zeros);
        _mm256_store_si256(dst.add(96) as *mut __m256i, zeros);
        dst = dst.add(128);
        size_bytes -= 128;
    }
    
    // Epilogue
    page_zero_scalar(dst, size_bytes);
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
unsafe fn page_zero_avx2_nt(mut dst: *mut u8, mut size_bytes: usize) {
    use core::arch::x86_64::*;
    
    // Prologue: align to 32 bytes
    while (dst as usize) % 32 != 0 && size_bytes > 0 {
        // SAFETY: In bounds
        *dst = 0;
        dst = dst.add(1);
        size_bytes -= 1;
    }

    let zeros = _mm256_setzero_si256();
    while size_bytes >= 128 {
        // SAFETY: Aligned to 32, within bounds, non-temporal
        _mm256_stream_si256(dst as *mut __m256i, zeros);
        _mm256_stream_si256(dst.add(32) as *mut __m256i, zeros);
        _mm256_stream_si256(dst.add(64) as *mut __m256i, zeros);
        _mm256_stream_si256(dst.add(96) as *mut __m256i, zeros);
        dst = dst.add(128);
        size_bytes -= 128;
    }
    
    // Epilogue
    page_zero_scalar(dst, size_bytes);
    
    // SAFETY: Flush NT stores
    _mm_sfence();
}

#[cfg(target_arch = "aarch64")]
#[cfg(target_feature = "neon")]
unsafe fn page_zero_neon(mut dst: *mut u8, mut size_bytes: usize) {
    use core::arch::aarch64::*;
    
    // Prologue: align to 16 bytes
    while (dst as usize) % 16 != 0 && size_bytes > 0 {
        // SAFETY: In bounds
        *dst = 0;
        dst = dst.add(1);
        size_bytes -= 1;
    }

    // SAFETY: neon feature is enabled
    let zeros = vdupq_n_u8(0);
    while size_bytes >= 64 {
        // SAFETY: Aligned to 16, within bounds
        vst1q_u8(dst, zeros);
        vst1q_u8(dst.add(16), zeros);
        vst1q_u8(dst.add(32), zeros);
        vst1q_u8(dst.add(48), zeros);
        dst = dst.add(64);
        size_bytes -= 64;
    }
    
    // Epilogue
    page_zero_scalar(dst, size_bytes);
}
