extern crate lrmalloc_rs;

use std::thread;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::alloc::{GlobalAlloc, Layout};
use lrmalloc_rs::{do_aligned_alloc, do_free, IN_BOOTSTRAP, IN_CACHE};
use core::sync::atomic::Ordering;

struct Dummy;
#[global_allocator]
static ALLOCATOR: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // do_malloc(layout.size())
        let output = do_aligned_alloc(layout.align(), layout.size());
        output
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _layout = layout;
        do_free(ptr)
    }
}


#[test]
fn test_multiple_threads() {

    let mut vec = vec![];
    let boxes = Arc::new(Mutex::new(Vec::new()));

    for i in 0..100 {
        let clone = boxes.clone();
        vec.push(thread::spawn (move || {
            println!("Thread {} says hi!", &i);
            let b = Box::new(0xdeadbeafusize);
            let arc = clone;
            let mut guard = arc.lock().unwrap();
            guard.push(b);
        }));
    }

    for join_handle in vec {
        join_handle.join().unwrap();
    }

    println!();
    for x in & *boxes.lock().unwrap() {
        assert_eq!(**x, 0xdeadbeaf);
    }

    println!("Allocated in bootstrap: {} bytes", IN_BOOTSTRAP.load(Ordering::Relaxed));
    println!("Allocated in cache: {} bytes", IN_CACHE.load(Ordering::Relaxed));
}