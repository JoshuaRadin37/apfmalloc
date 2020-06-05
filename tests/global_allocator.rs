use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use lralloc_rs::{do_malloc, do_free, do_aligned_alloc};


struct Dummy;

#[global_allocator]
static allocator: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // do_malloc(layout.size())
        do_aligned_alloc(layout.align(), layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        do_free(ptr)
    }
}

#[test]
fn global_allocator() {

    let _vec = vec![1, 2, 3, 4];


}