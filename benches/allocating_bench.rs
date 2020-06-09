use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use std::iter::Iterator;
use lrmalloc_rs::{do_malloc, do_free};
use std::thread;

fn allocate_one_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate");
    for bytes in (8..=13).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::from_parameter(bytes as u64),
            &bytes,
            |b, &size| {
                b.iter(|| do_malloc(size as usize))
            }
        );

    }
    group.finish()
}

fn allocate_and_free_one_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate and free");
    for bytes in (8..=13).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::from_parameter(bytes as u64),
            &bytes,
            |b, &size| {
                b.iter(|| {
                    let mem = do_malloc(size as usize);
                    do_free(mem);
                })
            }
        );

    }
    group.finish()
}

fn allocate_and_free_one_thread_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate and free comparison");
    for bytes in (8..=13).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::from_parameter(bytes as u64),
            &bytes,
            |b, &size| {
                b.iter(|| {
                    Vec::<u8>::with_capacity(size as usize);
                })
            }
        );

    }
    group.finish()
}




criterion_group!(one_thread, allocate_one_thread, allocate_and_free_one_thread, allocate_and_free_one_thread_comparison);

criterion_main!(one_thread);