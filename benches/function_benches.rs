use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lrmalloc_rs::alloc::malloc_from_new_sb;
use lrmalloc_rs::allocation_data::{get_heaps, Descriptor};
use lrmalloc_rs::mem_info::{MAX_SZ_IDX, PAGE};
use lrmalloc_rs::pages::page_alloc;
use lrmalloc_rs::size_classes::SIZE_CLASSES;
use lrmalloc_rs::thread_cache::{fill_cache, ThreadCacheBin};
use lrmalloc_rs::{allocate_to_cache, do_malloc};

fn thread_cache_fill(c: &mut Criterion) {
    let mut tcache = [ThreadCacheBin::new(); MAX_SZ_IDX];
    c.bench_function("cache fill", |b| {
        let cache = &mut tcache[1];
        b.iter(|| {
            fill_cache(1, cache);
            cache.pop_list(cache.peek_block(), cache.get_block_num());
        });
    });
}

fn page_alloc_bench(c: &mut Criterion) {
    c.bench_function("page get", |b| {
        b.iter(|| page_alloc(PAGE));
    });
}

fn desc_alloc(c: &mut Criterion) {
    c.bench_function("allocate descriptor", |b| {
        b.iter(|| unsafe { Descriptor::alloc() });
    });
}

criterion_group!(functions, page_alloc_bench, desc_alloc, thread_cache_fill);
criterion_main!(functions);
