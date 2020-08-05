use criterion::{criterion_group, criterion_main, Criterion};
use apfmalloc_lib::alloc::malloc_from_new_sb;
use apfmalloc_lib::allocation_data::Descriptor;
use apfmalloc_lib::mem_info::{MAX_SZ_IDX, PAGE};
use apfmalloc_lib::pages::{page_alloc, page_free};
use apfmalloc_lib::ptr::auto_ptr::AutoPtr;
use apfmalloc_lib::thread_cache::{fill_cache, ThreadCacheBin};
use std::io::{stdout, Write};
use std::time::Duration;
use std::time::Instant;

fn thread_cache_fill(c: &mut Criterion) {
    let mut tcache = [ThreadCacheBin::new(); MAX_SZ_IDX];
    let _ptr = AutoPtr::new(8);
    c.bench_function("cache fill", |b| {
        let cache = &mut tcache[1];
        b.iter(|| {
            fill_cache(1, cache);
            cache.pop_list(cache.peek_block(), cache.get_block_num());
        });
    });
}

fn alloc_from_super_block(c: &mut Criterion) {
    let _ptr = AutoPtr::new(8usize);
    c.bench_function("alloc from super block", |b| {
        b.iter(|| {
            let mut cache = ThreadCacheBin::new();
            let mut block_num = 0;
            malloc_from_new_sb(3, &mut cache, &mut block_num);
            let ptr = cache.peek_block();
            cache.pop_list(ptr, cache.get_block_num());
            page_free(ptr);
        });
    });
}

fn alloc_from_super_block_no_free(c: &mut Criterion) {
    let _ptr = AutoPtr::new(8usize);
    let mut ptrs = vec![];
    c.bench_function("alloc from super block no free", |b| {
        b.iter(|| {
            let mut cache = ThreadCacheBin::new();
            let mut block_num = 0;
            malloc_from_new_sb(3, &mut cache, &mut block_num);
            let ptr = cache.peek_block();
            cache.pop_list(ptr, cache.get_block_num());
            ptrs.push(ptr);
        });
    });
    for ptr in ptrs {
        page_free(ptr);
    }
}

fn page_free_time(c: &mut Criterion) {
    let _ptr = AutoPtr::new(8usize);
    c.bench_function("page free", |b| {
        b.iter_custom(|iters| {
            let mut cache = ThreadCacheBin::new();
            let mut block_num = 0;
            let mut output = Duration::from_secs(0);
            let mut ptrs = vec![];
            for _ in 0..iters {
                malloc_from_new_sb(3, &mut cache, &mut block_num);
                let ptr = cache.peek_block();
                ptrs.push(ptr);
                cache.pop_list(ptr, cache.get_block_num());
            }
            for ptr in ptrs {
                let dur = Instant::now();

                page_free(ptr);
                output += dur.elapsed();
            }
            output
        });
    });
}

fn page_alloc_bench(c: &mut Criterion) {
    let mut ptrs = vec![];
    c.bench_function("page get", |b| {
        b.iter(|| ptrs.push(page_alloc(PAGE).unwrap()));
    });
    print!("Freeing pages ({}) from page get bench... ", ptrs.len());
    stdout().flush().unwrap();
    for ptr in ptrs {
        page_free(ptr);
    }
    println!("done")
}

fn desc_alloc(c: &mut Criterion) {
    c.bench_function("allocate descriptor", |b| {
        b.iter(|| unsafe { Descriptor::alloc() });
    });
}

criterion_group!(paging, page_alloc_bench, page_free_time);
criterion_group!(
    functions,
    desc_alloc,
    thread_cache_fill,
    alloc_from_super_block,
    alloc_from_super_block_no_free
);
criterion_main!(paging, functions);
