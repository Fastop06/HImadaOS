use limine::request::MemmapResponse;
use limine::memmap::{MEMMAP_USABLE, MEMMAP_BOOTLOADER_RECLAIMABLE};
use spin::Mutex;
use core::ptr;
use crate::sysmem::page_zero;

pub const PAGE_SIZE: usize = 4096;

pub struct BitmapAllocator {
    bitmap: &'static mut [u8],
    total_frames: usize,
    last_alloc_index: usize,
    free_frames: usize,
    refcounts: &'static mut [u8],
}

impl BitmapAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: &mut [],
            total_frames: 0,
            last_alloc_index: 0,
            free_frames: 0,
            refcounts: &mut [],
        }
    }

    pub fn init(&mut self, memmap: &MemmapResponse) {
        // Find highest address to calculate bitmap size
        let mut highest_addr = 0;
        for entry in memmap.entries() {
            if entry.type_ == MEMMAP_USABLE {
                let end = entry.base + entry.length;
                if end > highest_addr {
                    highest_addr = end;
                }
            }
        }

        self.total_frames = (highest_addr as usize) / PAGE_SIZE;
        let bitmap_size_bytes = (self.total_frames + 7) / 8;
        let refcounts_size_bytes = self.total_frames;
        let total_needed = bitmap_size_bytes + refcounts_size_bytes + 2 * PAGE_SIZE;

        // Find a usable region to store both bitmap and refcounts contiguously
        // Avoid physical address 0 (page 0 / IVT / BIOS data)
        let mut base_phys: Option<u64> = None;
        for entry in memmap.entries() {
            if entry.type_ == MEMMAP_USABLE && entry.length >= total_needed as u64 {
                let candidate = if entry.base < 0x1000_000 {
                    0x1000_000 // Skip first 16MB
                } else {
                    entry.base
                };
                if entry.base + entry.length >= candidate + total_needed as u64 {
                    base_phys = Some(candidate);
                    break;
                }
            }
        }

        let base_phys = base_phys.expect("Not enough memory for PMM metadata!");
        let bitmap_phys = base_phys;
        let bitmap_pages = (bitmap_size_bytes + PAGE_SIZE - 1) / PAGE_SIZE;
        let bitmap_ptr = unsafe { crate::mm::vmm::phys_to_virt(bitmap_phys as usize) as *mut u8 };

        self.bitmap = unsafe { core::slice::from_raw_parts_mut(bitmap_ptr, bitmap_size_bytes) };
        self.bitmap.fill(0xFF);
        self.free_frames = 0;

        // Iterate usable regions and free those frames in bulk (fast!)
        for entry in memmap.entries() {
            if entry.type_ == MEMMAP_USABLE {
                let start_frame = (entry.base as usize) / PAGE_SIZE;
                let end_frame = ((entry.base + entry.length) as usize) / PAGE_SIZE;
                
                let mut frame = start_frame;
                while frame < end_frame && (frame % 8) != 0 {
                    self.free(frame);
                    frame += 1;
                }
                
                let full_bytes = (end_frame - frame) / 8;
                if full_bytes > 0 {
                    let start_byte = frame / 8;
                    let end_byte = start_byte + full_bytes;
                    if end_byte <= self.bitmap.len() {
                        self.bitmap[start_byte..end_byte].fill(0x00);
                        self.free_frames += full_bytes * 8;
                        frame += full_bytes * 8;
                    }
                }
                
                while frame < end_frame {
                    self.free(frame);
                    frame += 1;
                }
            }
        }

        let refcounts_phys = base_phys + (bitmap_pages * PAGE_SIZE) as u64;
        let refcounts_pages = (refcounts_size_bytes + PAGE_SIZE - 1) / PAGE_SIZE;
        let refcounts_ptr = unsafe { crate::mm::vmm::phys_to_virt(refcounts_phys as usize) as *mut u8 };

        self.refcounts = unsafe { core::slice::from_raw_parts_mut(refcounts_ptr, refcounts_size_bytes) };
        self.refcounts.fill(0);

        // Re-reserve the frames where bitmap and refcounts are stored
        let bitmap_start_frame = (bitmap_phys as usize) / PAGE_SIZE;
        for frame in bitmap_start_frame..(bitmap_start_frame + bitmap_pages) {
            self.mark_used(frame);
        }
        let ref_start_frame = (refcounts_phys as usize) / PAGE_SIZE;
        for frame in ref_start_frame..(ref_start_frame + refcounts_pages) {
            self.mark_used(frame);
        }

        crate::serial_println!("[PMM] Init: {} MB total, {} MB free", 
            (self.total_frames * PAGE_SIZE) / 1024 / 1024,
            (self.free_frames * PAGE_SIZE) / 1024 / 1024
        );
    }

    fn mark_used(&mut self, frame: usize) {
        if frame < self.total_frames {
            let byte_idx = frame / 8;
            let bit_idx = frame % 8;
            if (self.bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                self.bitmap[byte_idx] |= 1 << bit_idx;
                self.free_frames -= 1;
            }
        }
    }

    fn free(&mut self, frame: usize) {
        if frame < self.total_frames {
            let byte_idx = frame / 8;
            let bit_idx = frame % 8;
            if (self.bitmap[byte_idx] & (1 << bit_idx)) != 0 {
                self.bitmap[byte_idx] &= !(1 << bit_idx);
                self.free_frames += 1;
            }
        }
    }

    pub fn alloc_frame(&mut self) -> Option<usize> {
        if self.free_frames == 0 {
            return None;
        }

        let total_bytes = self.bitmap.len();
        let start_byte = self.last_alloc_index / 8;

        for i in 0..total_bytes {
            let byte_idx = (start_byte + i) % total_bytes;
            if self.bitmap[byte_idx] != 0xFF {
                for bit in 0..8 {
                    let frame = byte_idx * 8 + bit;
                    if frame < self.total_frames && (self.bitmap[byte_idx] & (1 << bit)) == 0 {
                        self.mark_used(frame);
                        self.last_alloc_index = frame;
                        self.refcounts[frame] = 1;

                        unsafe {
                            let virt = crate::mm::vmm::phys_to_virt(frame * PAGE_SIZE) as *mut u8;
                            page_zero(virt, PAGE_SIZE);
                        }

                        return Some(frame * PAGE_SIZE);
                    }
                }
            }
        }
        None
    }


    pub fn inc_ref(&mut self, phys_addr: usize) {
        let frame = phys_addr / PAGE_SIZE;
        if frame < self.total_frames {
            self.refcounts[frame] = self.refcounts[frame].saturating_add(1);
        }
    }

    pub fn free_frame(&mut self, phys_addr: usize) {
        let frame = phys_addr / PAGE_SIZE;
        if frame < self.total_frames {
            if self.refcounts[frame] > 0 {
                self.refcounts[frame] -= 1;
                if self.refcounts[frame] == 0 {
                    self.free(frame);
                }
            }
        }
    }

    pub fn get_ref(&self, phys_addr: usize) -> u8 {
        let frame = phys_addr / PAGE_SIZE;
        if frame < self.total_frames {
            self.refcounts[frame]
        } else {
            0
        }
    }

    pub fn alloc_frames(&mut self, pages: usize) -> Option<usize> {
        if self.free_frames < pages {
            return None;
        }

        let mut count = 0;
        let mut start_frame = 0;

        for frame in 0..self.total_frames {
            let byte_idx = frame / 8;
            let bit_idx = frame % 8;

            if (self.bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                if count == 0 {
                    start_frame = frame;
                }
                count += 1;
                if count == pages {
                    for i in start_frame..(start_frame + pages) {
                        self.mark_used(i);
                        self.refcounts[i] = 1;
                        unsafe {
                            let virt = crate::mm::vmm::phys_to_virt(i * PAGE_SIZE) as *mut u8;
                            page_zero(virt, PAGE_SIZE);
                        }
                    }
                    self.last_alloc_index = start_frame + pages;
                    return Some(start_frame * PAGE_SIZE);
                }
            } else {
                count = 0;
            }
        }
        None
    }

    pub fn free_frames(&mut self, phys_addr: usize, pages: usize) {
        let start_frame = phys_addr / PAGE_SIZE;
        for i in start_frame..(start_frame + pages) {
            if i < self.total_frames {
                if self.refcounts[i] > 0 {
                    self.refcounts[i] -= 1;
                    if self.refcounts[i] == 0 {
                        self.free(i);
                    }
                }
            }
        }
    }
}

pub static PMM: Mutex<BitmapAllocator> = Mutex::new(BitmapAllocator::new());

pub fn init(memmap: &MemmapResponse) {
    PMM.lock().init(memmap);
}

pub fn alloc_frame() -> Option<usize> {
    PMM.lock().alloc_frame()
}

pub fn free_frame(phys_addr: usize) {
    PMM.lock().free_frame(phys_addr);
}

pub fn inc_ref(phys_addr: usize) {
    PMM.lock().inc_ref(phys_addr);
}

pub fn get_ref(phys_addr: usize) -> u8 {
    PMM.lock().get_ref(phys_addr)
}

pub fn alloc_frames(pages: usize) -> Option<usize> {
    PMM.lock().alloc_frames(pages)
}

pub fn free_frames(phys_addr: usize, pages: usize) {
    PMM.lock().free_frames(phys_addr, pages);
}

pub fn get_memory_stats() -> (usize, usize) {
    let pmm = PMM.lock();
    (pmm.total_frames * PAGE_SIZE, pmm.free_frames * PAGE_SIZE)
}
