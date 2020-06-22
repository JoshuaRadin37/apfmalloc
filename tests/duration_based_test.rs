use std::sync::Arc;
use std::time::Instant;
use std::thread;
use lrmalloc_rs::ptr::auto_ptr::AutoPtr;

#[test]
#[ignore]
fn long_duration_test() {
    for num_threads in (0..6).map(|pow| 1 << pow) {
        println!("Allocating with {} threads", num_threads);
        let mut vec = Vec::with_capacity(num_threads);
        let mut bytes_allocated = 0usize;
        let total_time = Instant::now();
        for i in 0..num_threads {
            vec.push(thread::Builder::new().name(format!("Thread {}", i)).spawn(move || {
                let mut temp = Vec::with_capacity(10000);

                let start = Instant::now();
                let mut loops = 0;
                while start.elapsed().as_secs() < 10 {
                    temp.push(AutoPtr::new(0usize));
                    loops += 1;
                }
                loops * 8
            }).unwrap());
        }
        for join in vec {
            bytes_allocated += join.join().unwrap();
        }
        println!("Allocated {} bytes with {} threads in {:.5?} seconds", bytes_allocated, num_threads, total_time.elapsed().as_secs_f64())
    }
}
