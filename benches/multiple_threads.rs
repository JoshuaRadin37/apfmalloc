use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lrmalloc_rs::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_free, do_malloc};
use std::iter::Iterator;
use std::sync::{Arc, Mutex};
use std::thread;

fn allocate_multi_thread_constant_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate multi thread");
    for threads in 1..32 {
        group.throughput(Throughput::Elements(threads));
        group.bench_with_input(
            BenchmarkId::new("lrmalloc-rs", threads),
            &(threads as usize),
            |b, &size| {
                let mut vec = Vec::with_capacity(size);
                b.iter(|| {
                    for _ in 0..size {
                        vec.push(thread::spawn(move || {
                            do_malloc(16);
                        }));
                    }
                });
                for join in vec {
                    join.join().unwrap();
                }
            },
        );
    }
    group.finish()
}

criterion_group!(multi_thread, allocate_multi_thread_constant_bytes);
criterion_main!(multi_thread);
