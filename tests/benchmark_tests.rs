use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_free, do_malloc};
use std::thread;
use std::time::{Duration, Instant};

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
    let _ = AutoPtr::new(0usize);

    let mut time_sum = Duration::from_secs(0);
    let runs = 500;
    for i in 0..runs {
        let mut vec: Vec<AutoPtr<usize>> = vec![];
        let start = Instant::now();
        for _ in 0..256 {
            vec.push(AutoPtr::new(0));
        }
        let end = start.elapsed();
        time_sum += end;
        println!("Run {} took {} ms", i, end.as_micros())
    }

    let avg = time_sum / runs;
    println!("Average run time: {} ms", avg.as_micros());
}
