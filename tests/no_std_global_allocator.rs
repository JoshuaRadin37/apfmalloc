#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use lrmalloc_rs::{do_aligned_alloc, do_free};

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

struct Dummy;

#[global_allocator]
static ALLOCATOR: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        do_aligned_alloc(layout.align(), layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = layout;
        do_free(ptr)
    }
}

#[test]
fn no_std_global_allocator() {
    let mut vec: Vec<_> = (0..100).map(|i| Box::new(i)).collect::<Vec<Box<usize>>>();

    for i in 0usize..100 {
        assert_eq!(i, *vec[i])
    }

    let v: Vec<_> = vec.drain(0..100).collect();
    assert_eq!(vec.len(), 0);
    assert_eq!(v.len(), 100);
}
