use core::alloc::Layout;

use linked_list_allocator::LockedHeap;

const HEAP_SIZE: usize = 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[link_section = ".bss.heap"]
static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// Initializes the kernel heap allocator.
pub fn init_heap() {
    unsafe {
        ALLOCATOR
            .lock()
            .init(core::ptr::addr_of_mut!(HEAP_SPACE) as *mut u8, HEAP_SIZE);
    }
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    loop {}
}
