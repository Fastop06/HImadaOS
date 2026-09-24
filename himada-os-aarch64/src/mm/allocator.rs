use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_SIZE: usize = 256 * 1024 * 1024; // 256 MiB

pub fn init_heap() {
    let pages = HEAP_SIZE / 4096;
    let paddr = super::pmm::alloc_frames(pages).expect("Failed to allocate physical frames for heap");
    let vaddr = unsafe { super::vmm::phys_to_virt(paddr) };
    unsafe {
        ALLOCATOR.lock().init(vaddr as *mut u8, HEAP_SIZE);
    }
}
