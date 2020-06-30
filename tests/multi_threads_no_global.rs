use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use lrmalloc_rs::{IN_BOOTSTRAP, IN_CACHE};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_multiple_threads() {
    let mut vec = vec![];
    let boxes = Arc::new(Mutex::new(Vec::new()));
    let value = 3799usize;
    for i in 0..30 {
        let clone = boxes.clone();
        vec.push(thread::spawn(move || {
            println!("Thread {} says hi!", &i);
            // thread::sleep(Duration::from_secs_f64(5.0));
            for _ in 0..10_000 {
                let b = AutoPtr::new(value);
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
        assert_eq!(**x, value);
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
