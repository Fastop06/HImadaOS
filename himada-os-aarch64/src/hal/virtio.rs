use virtio_drivers::{Hal, BufferDirection, PhysAddr};
use core::ptr::NonNull;
use crate::mm::{pmm, vmm};

pub struct VirtioHal;

unsafe impl Hal for VirtioHal {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let paddr = pmm::alloc_frames(pages).expect("DMA alloc failed");
        let vaddr = unsafe { vmm::phys_to_virt(paddr) };
        // Zero out the DMA memory for security and avoiding stale data
        unsafe {
            crate::sysmem::page_zero(vaddr as *mut u8, pages * 4096);
        }
        (paddr as u64, NonNull::new(vaddr as *mut u8).unwrap())
    }

    unsafe fn dma_dealloc(paddr: PhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        pmm::free_frames(paddr as usize, pages);
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, size: usize) -> NonNull<u8> {
        let offset = paddr as usize & 0xFFF;
        let pages = (offset + size + 4095) / 4096;
        for i in 0..pages {
            let page_paddr = (paddr as usize & !0xFFF) + i * 4096;
            vmm::map_device_page(page_paddr, page_paddr);
        }
        NonNull::new(paddr as *mut u8).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let vaddr = buffer.cast::<u8>().as_ptr() as usize;
        vmm::virt_to_phys(vaddr) as u64
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}
