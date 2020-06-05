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


fn main() {
    println!("Hello, world!");

    let box_test = Box::new(15);
    println!("Box value: {:?}", box_test);
    let box_test2 = Box::new([36; 32]);
    println!("Box value: {:?}", box_test2);
}
