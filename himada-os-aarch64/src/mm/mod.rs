use limine::request::MemmapRequest;

#[used]
#[link_section = ".requests"]
static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();

pub mod pmm;
pub mod vmm;
pub mod allocator;

pub fn init() {
    crate::log_step("MM: requesting memmap...", None);
    let mmap_response = match MEMMAP_REQUEST.response() {
        Some(resp) => resp,
        None => {
            crate::log_step("MM: No memmap response from Limine!", Some("FAIL"));
            panic!("No memmap");
        }
    };
    crate::log_step("MM: memmap OK, calling pmm::init...", None);
    pmm::init(mmap_response);
    crate::log_step("MM: pmm::init OK, calling vmm::init...", None);
    unsafe { vmm::init(); }
    crate::log_step("MM: vmm::init OK, calling heap init...", None);
    allocator::init_heap();
    crate::log_step("MM: Heap initialized successfully.", Some("OK"));
}

/// High-performance SIMD memory zeroing derived from the himada engine.
#[inline(always)]
pub unsafe fn himada_page_zero(mut dst: *mut u8, mut size_bytes: usize) {
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::*;
        // Prologue: align to 16 bytes
        while (dst as usize) % 16 != 0 && size_bytes > 0 {
            *dst = 0;
            dst = dst.add(1);
            size_bytes -= 1;
        }
        let zeros = vdupq_n_u8(0);
        while size_bytes >= 64 {
            vst1q_u8(dst, zeros);
            vst1q_u8(dst.add(16), zeros);
            vst1q_u8(dst.add(32), zeros);
            vst1q_u8(dst.add(48), zeros);
            dst = dst.add(64);
            size_bytes -= 64;
        }
        while size_bytes >= 16 {
            vst1q_u8(dst, zeros);
            dst = dst.add(16);
            size_bytes -= 16;
        }
    }
    // Epilogue/Fallback
    core::ptr::write_bytes(dst, 0, size_bytes);
}

/// High-performance SIMD memory copying derived from the himada engine (NEON unrolled 4x).
#[inline(always)]
pub unsafe fn himada_page_copy(mut dst: *mut u8, mut src: *const u8, mut size_bytes: usize) {
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::*;
        while size_bytes >= 64 {
            let v0 = vld1q_u8(src);
            let v1 = vld1q_u8(src.add(16));
            let v2 = vld1q_u8(src.add(32));
            let v3 = vld1q_u8(src.add(48));
            vst1q_u8(dst, v0);
            vst1q_u8(dst.add(16), v1);
            vst1q_u8(dst.add(32), v2);
            vst1q_u8(dst.add(48), v3);
            src = src.add(64);
            dst = dst.add(64);
            size_bytes -= 64;
        }
        while size_bytes >= 16 {
            let v = vld1q_u8(src);
            vst1q_u8(dst, v);
            src = src.add(16);
            dst = dst.add(16);
            size_bytes -= 16;
        }
    }
    core::ptr::copy_nonoverlapping(src, dst, size_bytes);
}
