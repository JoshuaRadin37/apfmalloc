use lrmalloc_rs::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_free, do_malloc};
use std::thread;

#[test]
fn multi_test_from_bench_no_global() {
    let size = 1;
    for t in 0..10 {
        let mut vec = Vec::with_capacity(size);
        for _ in 0..size {
            vec.push(thread::spawn(move || AutoPtr::new(3799i16)));
        }
        for (i, join) in vec.into_iter().enumerate() {
            let _ptr = match join.join() {
                Ok(_) => {}
                Err(e) => {
                    panic!(e);
                }
            };
        }
    }
}

#[test]
fn allocation() {
    let mut vec: Vec<*const u8> = vec![];
    for _ in 0..75000 {
        for _ in 0..256 {
            vec.push(do_malloc(16usize));
        }
    }

    for ptr in vec {
        do_free(ptr);
    }
}
