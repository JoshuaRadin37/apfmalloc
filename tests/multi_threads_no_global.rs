use lrmalloc_rs::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_malloc, IN_BOOTSTRAP, IN_CACHE};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_multiple_threads() {
    let mut vec = vec![];
    let boxes = Arc::new(Mutex::new(Vec::new()));

    for i in 0..30 {
        let clone = boxes.clone();
        vec.push(thread::spawn(move || {
            println!("Thread {} says hi!", &i);
            // thread::sleep(Duration::from_secs_f64(5.0));
            for _ in 0..10_000 {
                let b = AutoPtr::new(0xdeadbeafusize);
                let arc = &clone;
                let mut guard = arc.lock().unwrap();
                guard.push(b);
            }
        }));
    }

    for join_handle in vec {
        join_handle.join().unwrap();
    }

    println!();
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
fn multi_test_from_bench_no_global() {
    let size = 1;
    for t in 0..10 {
        let mut vec = Vec::with_capacity(size);
        for _ in 0..size {
            vec.push(thread::spawn(move || {
                AutoPtr::new(3799i16)
            }));
        }
        for (i, join) in vec.into_iter().enumerate() {
            let _ptr = match join.join() {
                Ok(_) => {},
                Err(e) => {
                    panic!(e);
                }
            };
        }
    };

}
