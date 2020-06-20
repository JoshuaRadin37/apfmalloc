use benchmarking_tools::*;
use criterion::{Criterion, criterion_group, criterion_main, Throughput, BenchmarkId};
use std::sync::{Arc, Mutex};
use std::thread;
use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use std::time::{Instant, Duration};
use criterion::measurement::Measurement;

fn set_measurement() -> Criterion<BytesAllocated> {
    Criterion::default().with_measurement(BytesAllocated)
}

fn allocate_for_10_sec(c: &mut Criterion<BytesAllocated>) {
    let mut group = c.benchmark_group("allocate multi thread for 10 sec");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    for threads in 0..3 {
        group.throughput(Throughput::Elements(1 << threads));
        group.bench_with_input(
            BenchmarkId::new("lrmalloc-rs", 1 << threads),
            &(1 << threads as usize),
            |b, &size| {
                b.iter_custom(|iters| {
                    let mut vec = Vec::with_capacity(size);
                    let output = BytesAllocated;
                    let mut bytes_allocated = Arc::new(Mutex::new(BytesAllocated::start(&output)));
                    for _ in 0..size {
                        let b = bytes_allocated.clone();
                        vec.push(thread::spawn(move || {

                            let mut temp = Vec::with_capacity(10000);

                            let start = Instant::now();
                            while start.elapsed().as_secs() < 10 {

                                temp.push(AutoPtr::new(0usize));
                                let mut guard = b.lock().unwrap();
                                *guard += 8;
                            }
                        }));
                    }
                    for join in vec {
                        join.join().unwrap();
                    }
                    let ret = (*bytes_allocated.lock().unwrap()).clone();
                    output.end(ret)
                });
            },
        );
    }
    for threads in 0..3 {
        group.throughput(Throughput::Elements(1 << threads));
        group.bench_with_input(
            BenchmarkId::new("native", 1 << threads),
            &(1 << threads as usize),
            |b, &size| {
                let ptrs = Arc::new(Mutex::new(Vec::new()));
                b.iter_custom(|iters| {
                    let mut vec = Vec::with_capacity(size);
                    let output = BytesAllocated;
                    let mut bytes_allocated = Arc::new(Mutex::new(BytesAllocated::start(&output)));
                    for _ in 0..size {
                        let clone = ptrs.clone();
                        let b = bytes_allocated.clone();
                        vec.push(thread::spawn(move || {

                            let mut temp = Vec::with_capacity(10000);

                            let start = Instant::now();
                            while start.elapsed().as_secs() < 10 {

                                temp.push(Box::new(0usize));
                                let mut guard = b.lock().unwrap();
                                *guard += 8;
                            }
                            clone.lock().unwrap().extend(temp)
                        }));
                    }
                    for join in vec {
                        join.join().unwrap();
                    }
                    let ret = (*bytes_allocated.lock().unwrap()).clone();
                    output.end(ret)
                });
            },
        );
    }
}

criterion_group!(
    name = bytes_measurement;
    config = set_measurement();
    targets = allocate_for_10_sec
);
criterion_main!(bytes_measurement);