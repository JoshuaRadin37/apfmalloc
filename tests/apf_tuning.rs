extern crate lrmalloc_rs;

use lrmalloc_rs::{ do_aligned_alloc, do_free };
use std::alloc::{ GlobalAlloc, Layout };
use std::thread;

struct Apf;
static ALLOCATOR: Apf = Apf;

unsafe impl GlobalAlloc for Apf {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// dbg!("alloc");
		do_aligned_alloc(layout.align(), layout.size())
	}

	unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
		do_free(ptr);
	}
}

#[test]
fn test_apf_tuning() {
	let mut vec = vec![];

	for i in 0..10 {
		vec.push(thread::spawn(move || {
			println!("Thread {}", &i);
		}));
	}

	for join_handle in vec {
        join_handle.join().unwrap();
    }
}