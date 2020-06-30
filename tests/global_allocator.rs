use lrmalloc_rs::{do_aligned_alloc, do_free, do_malloc, IN_BOOTSTRAP};
use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
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
fn global_allocator() {
    let mut vec: Vec<_> = (0..100).collect::<Vec<usize>>();

    let mut tuple = HashMap::new();
    *tuple.entry(0).or_insert(1) += 1;
    tuple.insert(1, 5);
    tuple.insert(3, 7);

    for i in 0usize..100 {
        assert_eq!(i, vec[i])
    }

    let v: Vec<_> = vec.drain(0..100).collect();
    assert_eq!(vec.len(), 0);
    assert_eq!(v.len(), 100);
}

#[test]
fn mass_stress_no_harness() {
    for j in 0..10 {
        let mut vec = vec![];
        for i in 0..8 {
            vec.push(thread::spawn(move || {
                do_malloc(8);
                println!("Thread {} says hello", j * 8 + i)
            }));
        }
        for join in vec {
            join.join().unwrap();
        }
    }
}

#[test]
fn panic_uses_direct_allocation() {
    let thread = thread::spawn(|| panic!("I'm picnicking!"));
    match thread.join() {
        Ok(_) => panic!("Should result in an error"),
        Err(_) => {
            assert!(IN_BOOTSTRAP.load(Ordering::Acquire) > 0);
        }
    }
}
