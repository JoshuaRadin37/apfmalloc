extern crate lrmalloc_rs;

use core::sync::atomic::Ordering;
use core::time::Duration;
use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_aligned_alloc, do_free, IN_BOOTSTRAP, IN_CACHE};
use std::alloc::{GlobalAlloc, Layout};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread;

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

    for i in 0..30 {
        let clone = boxes.clone();
        vec.push(thread::spawn(move || {
            println!("Thread {} says hi!", &i);
            // thread::sleep(Duration::from_secs_f64(5.0));
            for j in 0..10 {
                let b = Box::new(0xdeadbeafusize);
                println!("Thread {} created box {}", &i, j);
                let arc = &clone;
                let mut guard = arc.lock().unwrap();
                guard.push(b);
                println!("Boxes length: {}", guard.len());
            }
        }));
    }

    for join_handle in vec {
        join_handle.join().unwrap();
    }

    println!("All threads were joined");
    for x in &*boxes.lock().unwrap() {
        assert_eq!(**x, 0xdeadbeaf);
    }

    println!(
        "Allocated in bootstrap: {} bytes",
        IN_BOOTSTRAP.load(Ordering::Relaxed)
    );
    println!(
        "Allocated in cache: {} bytes",
        IN_CACHE.load(Ordering::Relaxed)
    );
}

#[test]
fn multi_test_from_bench() {
    let size = 32;
    for t in 0..10 {
        let mut vec = Vec::with_capacity(size);
        for _ in 0..size {
            vec.push(thread::spawn(move || AutoPtr::new(3799i16)));
        }
        for (i, join) in vec.into_iter().enumerate() {
            let _ptr = match join.join() {
                Ok(_) => {}
                Err(e) => {
                    panic!("{}", e.downcast_ref::<&'static str>().unwrap());
                }
            };
        }
    }
}
