use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use std::iter::Iterator;
use lralloc_rs::do_malloc;

fn allocate_one_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate");
    for bytes in (8..=18).map(|b| 1 << b) {
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

criterion_group!(benches, allocate_one_thread);
criterion_main!(benches);