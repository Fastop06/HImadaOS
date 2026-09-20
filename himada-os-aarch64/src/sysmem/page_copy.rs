use core::ptr;

const NT_THRESHOLD: usize = 2 * 1024 * 1024;

/// # Safety
/// - `dst` must be valid for writes of `size_bytes` bytes
/// - `src` must be valid for reads of `size_bytes` bytes
/// - `dst` and `src` must not overlap
#[inline]
pub unsafe fn page_copy(dst: *mut u8, src: *const u8, size_bytes: usize) {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            if size_bytes >= NT_THRESHOLD {
                return page_copy_avx2_nt(dst, src, size_bytes);
            } else {
                return page_copy_avx2(dst, src, size_bytes);
            }
        } 
        #[cfg(target_feature = "sse2")]
        {
            return page_copy_sse2(dst, src, size_bytes);
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(target_feature = "neon")]
        {
            return page_copy_neon(dst, src, size_bytes);
        }
    }
    page_copy_scalar(dst, src, size_bytes)
}

#[inline(always)]
unsafe fn page_copy_scalar(dst: *mut u8, src: *const u8, size_bytes: usize) {
    // SAFETY: The caller guarantees dst and src are valid and do not overlap.
    ptr::copy_nonoverlapping(src, dst, size_bytes);
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "sse2")]
unsafe fn page_copy_sse2(mut dst: *mut u8, mut src: *const u8, mut size_bytes: usize) {
    use core::arch::x86_64::*;
    
    // Prologue (align dst to 16 bytes)
    while (dst as usize) % 16 != 0 && size_bytes > 0 {
        // SAFETY: In bounds
        *dst = *src;
        dst = dst.add(1);
        src = src.add(1);
        size_bytes -= 1;
    }

    while size_bytes >= 64 {
        let v0 = _mm_loadu_si128(src as *const __m128i);
        let v1 = _mm_loadu_si128(src.add(16) as *const __m128i);
        let v2 = _mm_loadu_si128(src.add(32) as *const __m128i);
        let v3 = _mm_loadu_si128(src.add(48) as *const __m128i);
        
        _mm_store_si128(dst as *mut __m128i, v0);
        _mm_store_si128(dst.add(16) as *mut __m128i, v1);
        _mm_store_si128(dst.add(32) as *mut __m128i, v2);
        _mm_store_si128(dst.add(48) as *mut __m128i, v3);
        
        dst = dst.add(64);
        src = src.add(64);
        size_bytes -= 64;
    }
    
    page_copy_scalar(dst, src, size_bytes);
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
unsafe fn page_copy_avx2(mut dst: *mut u8, mut src: *const u8, mut size_bytes: usize) {
    use core::arch::x86_64::*;
    
    while (dst as usize) % 32 != 0 && size_bytes > 0 {
        // SAFETY: In bounds
        *dst = *src;
        dst = dst.add(1);
        src = src.add(1);
        size_bytes -= 1;
    }

    while size_bytes >= 128 {
        let v0 = _mm256_loadu_si256(src as *const __m256i);
        let v1 = _mm256_loadu_si256(src.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(src.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(src.add(96) as *const __m256i);
        
        _mm256_store_si256(dst as *mut __m256i, v0);
        _mm256_store_si256(dst.add(32) as *mut __m256i, v1);
        _mm256_store_si256(dst.add(64) as *mut __m256i, v2);
        _mm256_store_si256(dst.add(96) as *mut __m256i, v3);
        
        dst = dst.add(128);
        src = src.add(128);
        size_bytes -= 128;
    }
    
    page_copy_scalar(dst, src, size_bytes);
}

#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
unsafe fn page_copy_avx2_nt(mut dst: *mut u8, mut src: *const u8, mut size_bytes: usize) {
    use core::arch::x86_64::*;
    
    while (dst as usize) % 32 != 0 && size_bytes > 0 {
        // SAFETY: In bounds
        *dst = *src;
        dst = dst.add(1);
        src = src.add(1);
        size_bytes -= 1;
    }

    while size_bytes >= 128 {
        let v0 = _mm256_loadu_si256(src as *const __m256i);
        let v1 = _mm256_loadu_si256(src.add(32) as *const __m256i);
        let v2 = _mm256_loadu_si256(src.add(64) as *const __m256i);
        let v3 = _mm256_loadu_si256(src.add(96) as *const __m256i);
        
        _mm256_stream_si256(dst as *mut __m256i, v0);
        _mm256_stream_si256(dst.add(32) as *mut __m256i, v1);
        _mm256_stream_si256(dst.add(64) as *mut __m256i, v2);
        _mm256_stream_si256(dst.add(96) as *mut __m256i, v3);
        
        dst = dst.add(128);
        src = src.add(128);
        size_bytes -= 128;
    }
    
    page_copy_scalar(dst, src, size_bytes);
    
    // SAFETY: Flush NT stores
    _mm_sfence();
}

#[cfg(target_arch = "aarch64")]
#[cfg(target_feature = "neon")]
unsafe fn page_copy_neon(mut dst: *mut u8, mut src: *const u8, mut size_bytes: usize) {
    use core::arch::aarch64::*;
    
    while (dst as usize) % 16 != 0 && size_bytes > 0 {
        // SAFETY: In bounds
        *dst = *src;
        dst = dst.add(1);
        src = src.add(1);
        size_bytes -= 1;
    }

    while size_bytes >= 64 {
        // Pre-fetch not standardly exposed as PLDL1KEEP in core::arch across all versions simply.
        // We do standard NEON loads/stores.
        // SAFETY: Aligned to 16, within bounds
        let v0 = vld1q_u8(src);
        let v1 = vld1q_u8(src.add(16));
        let v2 = vld1q_u8(src.add(32));
        let v3 = vld1q_u8(src.add(48));
        
        vst1q_u8(dst, v0);
        vst1q_u8(dst.add(16), v1);
        vst1q_u8(dst.add(32), v2);
        vst1q_u8(dst.add(48), v3);
        
        dst = dst.add(64);
        src = src.add(64);
        size_bytes -= 64;
    }
    
    page_copy_scalar(dst, src, size_bytes);
}
