use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lrmalloc_rs::{do_free, do_malloc};
use std::iter::Iterator;

fn allocate_one_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate");
    for bytes in (4..=12).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes * 256));
        group.bench_with_input(
            BenchmarkId::new("lrmalloc-rs", bytes as u64),
            &bytes,
            |b, &size| {
                let mut vec: Vec<*const u8> = vec![];
                b.iter(|| {
                    for _ in 0..256 {
                        vec.push(do_malloc(size as usize));
                    }
                });
                unsafe {
                    black_box(|| {
                        for ptr in vec {
                            do_free(ptr);
                        }
                    });
                }
            },
        );
    }
    for bytes in (4..=12).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes * 256));
        group.bench_with_input(
            BenchmarkId::new("native", bytes as u64),
            &bytes,
            |b, &size| {
                let mut vec = vec![];
                b.iter(|| {
                    for _ in 0..256 {
                        let mut v = Vec::with_capacity(size as usize);
                        v.push(1);
                        vec.push(v);
                    }
                });
            },
        );
    }
    group.finish()
}

fn allocate_and_free_one_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate and free");
    for bytes in (3..=13).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("lrmalloc-rs", bytes as u64),
            &bytes,
            |b, &size| {
                b.iter(|| {
                    let mem = do_malloc(size as usize);
                    unsafe {
                        do_free(mem);
                    }
                })
            },
        );
    }
    for bytes in (3..=13).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("native", bytes as u64),
            &bytes,
            |b, &size| {
                b.iter(|| {
                    let mut v = Vec::<u8>::with_capacity(size as usize);
                    v.push(1);
                })
            },
        );
    }
    group.finish()
}

criterion_group!(
    one_thread,
    allocate_one_thread,
    allocate_and_free_one_thread,
);

criterion_main!(one_thread);
