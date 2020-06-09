use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId, black_box};
use std::iter::Iterator;
use lrmalloc_rs::{do_malloc, do_free};
use std::thread;

fn do_nothing(c: &mut Criterion) {
    c.bench_function(
        "do nothing",
        |b| {
            b.iter(|| {
                black_box(|| {
                    let x = 8usize;
                    let _y = x;
                });
            })
        }
    );
}

fn allocate_one_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate");
    for bytes in (4..=12).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::from_parameter(bytes as u64),
            &bytes,
            |b, &size| {
                let mut vec: Vec<* const u8> = vec![];
                b.iter(|| {
                    vec.push(do_malloc(size as usize));
                });
                black_box( ||
                for ptr in vec {
                    do_free(ptr);
                });
            }
        );

    }
    group.finish()
}

fn allocate_one_thread_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate comparison");
    for bytes in (4..=12).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::from_parameter(bytes as u64),
            &bytes,
            |b, &size| {
                let mut vec = vec![];
                b.iter(|| {
                    let mut v = Vec::with_capacity(size as usize);
                    v.push(1);
                    vec.push(v);
                });
            }
        );

    }
    group.finish()
}


fn allocate_and_free_one_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate and free");
    for bytes in (3..=13).map(|b| 1 << b) {
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
    for bytes in (3..=13).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::from_parameter(bytes as u64),
            &bytes,
            |b, &size| {
                b.iter(|| {
                    let mut v = Vec::<u8>::with_capacity(size as usize);
                    v.push(1);
                })
            }
        );

    }
    group.finish()
}





criterion_group!(one_thread, do_nothing, allocate_one_thread, allocate_one_thread_comparison, allocate_and_free_one_thread, allocate_and_free_one_thread_comparison);

criterion_main!(one_thread);