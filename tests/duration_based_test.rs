use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, RwLock};
use std::thread;
use std::time::Duration;
use std::time::Instant;

fn separate_duration_test(duration: Duration) {
    println!("Allocating as much memory as possible, where some amount of threads are spawned and each are instructed to allocate for {} \n\
    seconds straight. Each thread has their own start time.", duration.as_secs_f64());
    for num_threads in (0..6).map(|pow| 1 << pow) {
        println!("Allocating with {} threads", num_threads);
        let mut vec = Vec::with_capacity(num_threads);
        let mut bytes_allocated = 0usize;
        let total_time = Instant::now();
        for i in 0..num_threads {
            vec.push(
                thread::Builder::new()
                    .name(format!("Thread {}", i))
                    .spawn(move || {
                        let mut temp = Vec::with_capacity(10000);

                        let start = Instant::now();
                        let mut loops = 0;
                        while start.elapsed() < duration {
                            temp.push(AutoPtr::new(0usize));
                            loops += 1;
                        }
                        loops * 8
                    })
                    .unwrap(),
            );
        }
        for join in vec {
            bytes_allocated += join.join().unwrap();
        }
        println!(
            "Allocated {} bytes with {} threads in {:.5?} seconds",
            bytes_allocated,
            num_threads,
            total_time.elapsed().as_secs_f64()
        )
    }
}

fn shared_duration_test(duration: Duration, wait: bool) {
    println!("Allocating as much memory as possible, where some amount of threads are spawned and each are instructed to allocate until {} \n\
     seconds have passed. Each thread has shares the same start time. The threads will{}wait until all threads are ready to start the timer.",
             duration.as_secs_f64(),
             if wait {
                 " "
             } else {
                 " not "
             }
    );
    for num_threads in (0..6).map(|pow| 1 << pow) {
        println!("Allocating with {} threads", num_threads);
        let mut vec = Vec::with_capacity(num_threads);
        let mut bytes_allocated = 0usize;
        let total_time = Instant::now();
        let start: Arc<RwLock<Option<Instant>>> =
            Arc::new(RwLock::new(if wait { None } else { Some(Instant::now()) }));
        let barrier = Arc::new(Barrier::new(num_threads + 1));
        let go = Arc::new(AtomicBool::new(!wait));
        for i in 0..num_threads {
            let begin = start.clone();
            let b = barrier.clone();
            let go = go.clone();
            vec.push(
                thread::Builder::new()
                    .name(format!("Thread {}", i))
                    .spawn(move || {
                        let mut temp = Vec::with_capacity(10000);
                        if wait {
                            b.wait();
                            while !go.load(Ordering::Acquire) {}
                        }
                        let start: Instant = begin.read().unwrap().unwrap();

                        let mut loops = 0;
                        while start.elapsed() < duration {
                            temp.push(AutoPtr::new(0usize));
                            loops += 1;
                        }
                        loops * 8
                    })
                    .unwrap(),
            );
        }

        if wait {
            barrier.wait();
            let mut write_guard = start.write().unwrap();
            *write_guard = Some(Instant::now());
            go.store(true, Ordering::Release);
        }
        for join in vec {
            bytes_allocated += join
                .join()
                .map_err(|e| e.downcast::<&'static str>().unwrap())
                .unwrap();
        }
        println!(
            "Allocated {} bytes with {} threads in {:.5?} seconds",
            bytes_allocated,
            num_threads,
            total_time.elapsed().as_secs_f64()
        )
    }
}

#[test]
#[ignore]
fn separate_duration_test_10_sec() {
    separate_duration_test(Duration::from_secs(10));
}

/*
Appears to have the exact same result as the separate duration test
 */
#[test]
#[ignore]
fn shared_duration_test_10_sec_with_wait() {
    shared_duration_test(Duration::from_secs(10), true);
}

#[test]
#[ignore]
fn shared_duration_test_10_sec_with_no_wait() {
    shared_duration_test(Duration::from_secs(10), false);
}
