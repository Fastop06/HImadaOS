use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_SIZE: usize = 64 * 1024 * 1024; // 64 MiB default

pub fn init_heap() {
    let candidate_sizes = [128 * 1024 * 1024, 64 * 1024 * 1024, 32 * 1024 * 1024, 16 * 1024 * 1024];
    for &size in &candidate_sizes {
        let pages = size / 4096;
        if let Some(paddr) = super::pmm::alloc_frames(pages) {
            let vaddr = unsafe { super::vmm::phys_to_virt(paddr) };
            unsafe {
                ALLOCATOR.lock().init(vaddr as *mut u8, size);
            }
            return;
        }
    }
    panic!("Failed to allocate physical frames for heap");
}
