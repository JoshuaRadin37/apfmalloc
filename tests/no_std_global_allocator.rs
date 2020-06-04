#![no_std]


use core::alloc::{GlobalAlloc, Layout};
use lralloc_rs::{do_malloc, do_free};

extern crate alloc;

use alloc::vec::Vec;

struct Dummy;

#[global_allocator]
static allocator: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        do_malloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        do_free(ptr)
    }
}

#[test]
fn no_std_global_allocator() {
    let _vec = Vec::<usize>::with_capacity(8);
}