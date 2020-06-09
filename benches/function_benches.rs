use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use lrmalloc_rs::{allocate_to_cache, do_malloc};
use lrmalloc_rs::mem_info::{MAX_SZ_IDX, PAGE};
use lrmalloc_rs::thread_cache::{
    ThreadCacheBin,
    fill_cache
};
use lrmalloc_rs::pages::page_alloc;
use lrmalloc_rs::alloc::malloc_from_new_sb;
use lrmalloc_rs::allocation_data::Descriptor;

fn init_malloc(c: &mut Criterion) {

    c.bench_function(
        "init malloc",
        |b| {
            b.iter(|| unsafe {
                lrmalloc_rs::init_malloc()
            }
            );
        }

    );
}

fn thread_cache_fill(c: &mut Criterion) {
    let mut tcache = [ThreadCacheBin::new(); MAX_SZ_IDX];
    c.bench_function(
        "cache fill",
        |b| {
            let cache = &mut tcache[1];
            b.iter(
                || {
                    fill_cache(
                        1,
                        cache
                    );
                    cache.pop_list(cache.peek_block(), cache.get_block_num());
                }

            );
        }
    );
}

fn page_alloc_bench(c: &mut Criterion) {
    c.bench_function(
        "page get",
        |b| {
            b.iter( || {
                page_alloc(PAGE)
            }
            );
        }
    );
}


fn from_new_sb(c: &mut Criterion) {
    let mut tcache = [ThreadCacheBin::new(); MAX_SZ_IDX];
    unsafe {
        lrmalloc_rs::init_malloc();
    }
    c.bench_function(
        "malloc from new super block",
        |b| {
            let cache = &mut tcache[1];
            b.iter(
                || {
                    malloc_from_new_sb(1, cache, &mut 0);
                    cache.pop_list(cache.peek_block(), cache.get_block_num());
                }
            );
        }
    );
}

fn desc_alloc(c: &mut Criterion) {
    c.bench_function(
        "allocate descriptor",
        |b| {
            b.iter(
                || unsafe {
                    Descriptor::alloc()
                }
            );
        }
    );
}

criterion_group!(functions, init_malloc, thread_cache_fill, page_alloc_bench, from_new_sb, desc_alloc);
criterion_main!(functions);