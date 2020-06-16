use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lrmalloc_rs::alloc::malloc_from_new_sb;
use lrmalloc_rs::allocation_data::{get_heaps, Descriptor};
use lrmalloc_rs::mem_info::{MAX_SZ_IDX, PAGE};
use lrmalloc_rs::pages::page_alloc;
use lrmalloc_rs::size_classes::SIZE_CLASSES;
use lrmalloc_rs::thread_cache::{fill_cache, ThreadCacheBin};
use lrmalloc_rs::{allocate_to_cache, do_malloc};

fn init_malloc(c: &mut Criterion) {
    c.bench_function("init malloc", |b| {
        b.iter(|| unsafe { lrmalloc_rs::init_malloc() });
    });
}

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

fn from_new_sb(c: &mut Criterion) {
    let mut tcache = [ThreadCacheBin::new(); MAX_SZ_IDX];
    unsafe {
        lrmalloc_rs::init_malloc();
    }
    c.bench_function("malloc from new super block", |b| {
        let cache = &mut tcache[1];
        b.iter(|| {
            malloc_from_new_sb(1, cache, &mut 0);
            cache.pop_list(cache.peek_block(), cache.get_block_num());
        });
    });
}

fn initialize_sb(c: &mut Criterion) {
    unsafe {
        lrmalloc_rs::init_malloc();
    }
    let size_class_index = 1;
    c.bench_function("initialize super block", |b| {
        b.iter(|| {
            let sc = unsafe { &SIZE_CLASSES[size_class_index] };

            // debug_assert!(!desc.is_null());

            let block_size = sc.block_size;
            let max_count = sc.get_block_num();

            let super_block =
                page_alloc(sc.sb_size as usize).expect("Couldn't create a superblock");

            for idx in 0..(max_count - 1) {
                unsafe {
                    let block = super_block.offset((idx * block_size as usize) as isize);
                    let next = super_block.offset(((idx + 1) * block_size as usize) as isize);
                    *(block as *mut *mut u8) = next;
                }
            }
        })
    });
}

fn desc_alloc(c: &mut Criterion) {
    c.bench_function("allocate descriptor", |b| {
        b.iter(|| unsafe { Descriptor::alloc() });
    });
}

criterion_group!(
    functions,
    init_malloc,
    page_alloc_bench,
    initialize_sb,
    from_new_sb,
    desc_alloc,
    thread_cache_fill
);
criterion_main!(functions);
