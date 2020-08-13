use apfmalloc_lib::mem_info::{PAGE, MAX_SZ_IDX};
use std::ptr::null_mut;
use apfmalloc_lib::{do_free, do_malloc, do_aligned_alloc, do_realloc};
use rand::Rng;
use apfmalloc_lib::page_map::PAGE_TABLE;
use crate::Mode::{Malloc, AlignedAlloc, Realloc};
use apfmalloc_lib::size_classes::SIZE_CLASSES;
use std::collections::{HashMap, HashSet};
use std::thread;

const NUMBER_BINS: usize = 400;
const NUMBER_ALLOCS: usize = 100_000;
const MAX_SIZE: usize = PAGE * 5;

#[derive(Debug, Copy, Clone)]
pub enum Mode {
    Malloc,
    AlignedAlloc,
    Realloc
}

impl From<u8> for Mode {
    fn from(i: u8) -> Self {
        match i {
            0 => Malloc,
            1 => AlignedAlloc,
            2 => Realloc,
            _ => {
                panic!("{} is an invalid Mode", i)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Bin {
    ptr: *mut u8,
    mode: Option<Mode>,
    size: usize,
    prev_sizes: Vec<(Option<Mode>, usize)>
}

fn malloc_test() {
    let mut bins = vec![Bin { ptr: null_mut(), mode: None, size: 0, prev_sizes: Vec::new() }; NUMBER_BINS];

    let mut created_pointers = HashSet::new();

    let mut frees = 0;

    let mut rand = rand::thread_rng();
    for _i in 0..NUMBER_ALLOCS {

        let mode: Mode = rand.gen_range(0, 3).into();
        let bin = &mut bins[rand.gen_range(0usize, NUMBER_BINS)];
        let index = rand.gen_range(1, 2/*MAX_SZ_IDX */);
        let size =
            if index > 0 {
                unsafe {
                    SIZE_CLASSES[index].block_size as usize
                }
            } else {
                MAX_SIZE
            };

        unsafe {
            match &mode {
                Malloc => {
                    if bin.size > 0 {
                        frees += 1;
                        if *bin.ptr != 1 {
                            let found = created_pointers.contains(&bin.ptr);
                            panic!("Invalid pointer used. Pointer was{} allocated", if found {""} else {"n't"});
                        }
                        do_free(bin.ptr);
                    }
                    bin.ptr = do_malloc(size);
                },
                AlignedAlloc => {
                    if bin.size > 0 {
                        frees += 1;
                        if *bin.ptr != 1 {
                            let found = created_pointers.contains(&bin.ptr);
                            panic!("Invalid pointer used. Pointer was{} allocated", if found {""} else {"n't"});

                        }
                        do_free(bin.ptr);
                    } else {

                    }
                    bin.ptr = do_aligned_alloc(8, size);
                },
                Realloc => {
                    if bin.size == 0 {
                        bin.ptr = null_mut();
                    } else {
                        frees += 1;
                        if *bin.ptr != 1 {
                            let found = created_pointers.contains(&bin.ptr);
                            panic!("Invalid pointer used. Pointer was{} allocated", if found {""} else {"n't"});

                        }

                    }
                    bin.ptr = do_realloc(bin.ptr, size);
                }
            }
        }
        unsafe {
            // check for valid
            *bin.ptr = 1;
        }
        if created_pointers.contains(&bin.ptr) {
            // println!("Reusing a pointer");
        }
        created_pointers.insert(bin.ptr);

        bin.prev_sizes.push((bin.mode, bin.size));
        bin.mode = Some(mode);
        bin.size = size;
        assert!(!bin.ptr.is_null(), "Didn't allocate {} bytes with mode {:?}", size, mode);
    }

    for bin in bins {
        if bin.size > 0 {
            unsafe {
                do_free(bin.ptr);
            }
        }
    }
}

#[test]
fn single_thread_random_bins() {
    malloc_test();
    unsafe {
        println!("Total space used for table = {} bytes", PAGE_TABLE.get_total_size());
    }
}

#[test]
fn many_threads_random_bins() {

    let handles =
        (0usize..2)
            .into_iter()
            .map(|_| thread::spawn(malloc_test))
            .collect::<Vec<_>>();

    for handle in handles {
        handle.join().unwrap();
    }

    unsafe {
        println!("Total space used for table = {} bytes", PAGE_TABLE.get_total_size());
    }
}