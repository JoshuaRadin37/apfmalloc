use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use lrmalloc_rs::allocate_to_cache;
use lrmalloc_rs::size_classes::get_size_class;

fn allocate_to_cache_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate to cache");
    for bytes in (8..=13).map(|b| 1 << b) {
        group.throughput(Throughput::Bytes(bytes));
        let size_class_index = get_size_class(bytes as usize);
        group.bench_with_input(
            BenchmarkId::from_parameter(size_class_index as u64),
            &bytes,
            |b, &size_class| {
                b.iter(|| {
                    allocate_to_cache(0, size_class as usize);
                })
            }
        );

    }
    group.finish()
}

criterion_group!(functions, allocate_to_cache_bench);
criterion_main!(functions, multo);