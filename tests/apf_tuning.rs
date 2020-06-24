extern crate lrmalloc_rs;

use core::sync::atomic::Ordering;
use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_aligned_alloc, do_free};
use lrmalloc_rs::visualization::test;
use std::alloc::{GlobalAlloc, Layout};
use std::thread;

struct Apf;

unsafe impl GlobalAlloc for Apf {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        do_aligned_alloc(layout.align(), layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        do_free(ptr);
    }
}

#[global_allocator]
static ALLOCATOR: Apf = Apf;

#[test]
fn test_apf_tuning() {
    test().expect("Unable to draw test!");
    let mut vec = vec![];

    for _i in 0..10 {
        vec.push(thread::spawn(move || {
            //println!("Thread {}", &i);
            AutoPtr::new(5)
        }));
    }

    for join_handle in vec {
        println!("{}", join_handle.join().unwrap());
    }
}
