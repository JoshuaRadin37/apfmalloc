use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use std::iter::Iterator;
use lrmalloc_rs::{do_malloc, do_free};
use std::thread;


fn allocate_multi_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate multi thread");
    for bytes in (8..=14).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes * 8));
        group.bench_with_input(
            BenchmarkId::from_parameter(bytes as u64),
            &bytes,
            |b, &size| {
                b.iter(|| {
                    let mut vec = vec![];
                    for _ in 0..8 {
                        vec.push(thread::spawn( move ||
                            {
                                do_malloc(size as usize);
                            }));
                    }
                    for join in vec {
                        join.join().unwrap();
                    }
                })
            });
    };
    group.finish()
}

criterion_group!(multi_thread, allocate_multi_thread);
// criterion_main!(multi_thread);