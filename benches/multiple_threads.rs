use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use std::sync::{Arc, Mutex};
use std::thread;

fn allocate_multi_thread_constant_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate multi thread");
    for threads in 0..3 {
        group.throughput(Throughput::Elements(1 << threads));
        group.bench_with_input(
            BenchmarkId::new("lrmalloc-rs", 1 << threads),
            &(1 << threads as usize),
            |b, &size| {
                let ptrs = Arc::new(Mutex::new(Vec::new()));
                b.iter(|| {
                    let mut vec = Vec::with_capacity(size);
                    for _ in 0..size {
                        let clone = ptrs.clone();
                        vec.push(thread::spawn(move || {
                            let mut temp = Vec::with_capacity(10000);
                            for _ in 0..10000 {
                                temp.push(AutoPtr::new(0usize));
                            }
                            clone.lock().unwrap().extend(temp)
                        }));
                    }
                    for join in vec {
                        join.join().unwrap();
                    }
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
                b.iter(|| {
                    let mut vec = Vec::with_capacity(size);
                    for _ in 0..size {
                        let clone = ptrs.clone();
                        vec.push(thread::spawn(move || {
                            let mut temp = Vec::with_capacity(10000);
                            for _ in 0..10000 {
                                temp.push(Box::new(0usize));
                            }
                            clone.lock().unwrap().extend(temp)
                        }));
                    }
                    for join in vec {
                        join.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish()
}

criterion_group!(multi_thread, allocate_multi_thread_constant_bytes);
criterion_main!(multi_thread);
