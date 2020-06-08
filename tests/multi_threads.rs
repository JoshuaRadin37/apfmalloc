use std::thread;
use std::sync::{Arc, Mutex};
use std::alloc::{GlobalAlloc, Layout};
use lralloc_rs::{do_aligned_alloc, do_free};

struct Dummy;
#[global_allocator]
static allocator: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // do_malloc(layout.size())
        let output = do_aligned_alloc(layout.align(), layout.size());
        output
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        do_free(ptr)
    }
}


#[test]
fn multiple_threads() {

    let mut vec = vec![];
    let mut boxes = Arc::new(Mutex::new(Vec::new()));

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
        join_handle.join();
    }

    println!();
    for x in & *boxes.lock().unwrap() {
        assert_eq!(**x, 0xdeadbeaf);
    }
}