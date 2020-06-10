use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lrmalloc_rs::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_free, do_malloc};
use std::iter::Iterator;
use std::sync::{Arc, Mutex};
use std::thread;

fn allocate_multi_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate multi thread");
    for threads in 1..32 {
        group.throughput(Throughput::Elements(threads));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{} threads", threads as u64)),
            &(threads as usize),
            |b, &size| {
                b.iter_with_large_drop(|| {
                    let mut vec = Vec::with_capacity(size);
                    let mut ptrs = Arc::new(Mutex::new(Vec::with_capacity(size)));
                    for _ in 0..size {
                        let clone = ptrs.clone();
                        vec.push(thread::spawn(move || {
                            clone.lock().unwrap().push(AutoPtr::new(0u8));
                        }));
                    }
                    for join in vec {
                        join.join().unwrap();
                    }
                })
            },
        );
    }
    group.finish()
}

criterion_group!(multi_thread, allocate_multi_thread);
criterion_main!(multi_thread);
