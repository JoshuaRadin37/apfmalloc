use crate::allocation_data::{ProcHeap, Descriptor, DescriptorNode, get_heaps, Anchor, SuperBlockState};
use crate::thread_cache::ThreadCacheBin;
use crate::page_map::{PageInfo, S_PAGE_MAP};
use crate::mem_info::PAGE_MASK;
use std::ptr::null_mut;
use crate::size_classes::SIZE_CLASSES;
use std::sync::atomic::Ordering;



pub fn list_pop_partial(heap: &mut ProcHeap) -> Option<&mut Descriptor> {
    let list = &heap.partial_list;
    let ptr = list.load(Ordering::Acquire);
    let old_head = unsafe {

        & *ptr
    };
    let mut new_head: *mut DescriptorNode;
    loop {
        let old_desc = old_head.get_desc();
        if old_desc.is_none() {
            return None;
        }
        let old_desc = old_desc.unwrap();

        new_head = old_desc.next_partial.load(Ordering::Acquire);
        let desc = old_head.get_desc();
        let counter = old_head.get_counter().expect("Counter doesn't exist");
        unsafe { (*new_head).set(desc.unwrap(), counter); }


        if list.compare_exchange_weak(ptr, new_head, Ordering::Acquire, Ordering::Release).is_ok() {
            break;
        }
    }

    old_head.get_desc()
}

pub fn list_push_partial(desc: &'static mut Descriptor) {
    let heap = desc.proc_heap;
    let list = unsafe { & (*heap).partial_list };

    let old_head = unsafe {&mut *list.load(Ordering::Acquire) };
    let mut new_head = DescriptorNode::default();

    loop {
        new_head.set(desc, old_head.get_counter().expect("Old heap should exist") + 1);
        // debug_assert_ne!(old_head.get_desc(), new_head.get_desc());
        match new_head.get_desc() {
            None => { panic!("A descriptor should exist")},
            Some(desc) => {
                desc.next_partial.store(old_head as *mut DescriptorNode, Ordering::SeqCst)
            },
        }

        if list.compare_exchange_weak(old_head as *mut DescriptorNode, &mut new_head as *mut DescriptorNode, Ordering::Acquire, Ordering::Release).is_ok() {
            break;
        }
    }
}

pub fn heap_push_partial(desc: &'static mut Descriptor) {
    list_push_partial(desc)
}

pub fn heap_pop_partial(heap: &mut ProcHeap) -> Option<& mut Descriptor> {
    list_pop_partial(heap)
}

pub fn malloc_from_partial(size_class_index: usize, cache: &mut ThreadCacheBin, block_num: &usize) {
    let heap = get_heaps().get_heap_at_mut(size_class_index);
    let desc = heap_pop_partial(heap);

    match desc {
        None => { return; },
        Some(desc) => {

            let old_anchor = desc.anchor;
            let new_anchor: Anchor;

            let max_count = desc.max_count;
            let block_size = desc.block_size;

            let super_block = desc.super_block;

            loop {

                if old_anchor.get_stat() == SuperBlockState::EMPTY {
                    todo!("retire_desc()");
                    return malloc_from_partial(size_class_index, cache, block_num);
                }

                new_anchor = old_anchor;
                new_anchor.set_count(0);
                new_anchor.set_avail(max_count);
                new_anchor.set_state(SuperBlockState::FULL);

                if desc.anchor
            }
        },
    }
}

pub fn malloc_from_new_sb(size_class_index: usize, cache: &mut ThreadCacheBin, block_num: &usize) {

}

pub fn desc_alloc() -> * mut Descriptor {

}

pub fn fill_cache(size_class_index: usize, cache: &mut ThreadCacheBin) {


}

pub fn flush_cache(size_class_index: usize, cache: &mut ThreadCacheBin) {


}

pub fn update_page_map(heap: Option<&mut ProcHeap>, ptr: * mut u8, desc: Option<&mut Descriptor>, size_class_index: usize) {
    if ptr.is_null() {
        panic!("Pointer should not be null");
    }

    let mut info: PageInfo = PageInfo::default();
    info.set_ptr(desc.map_or(null_mut(), |d| d as *mut Descriptor), size_class_index);
    if heap.is_none() {
        unsafe {
            S_PAGE_MAP.set_page_info(ptr, info);
            return;
        }
    }

    let heap = heap.unwrap();
    let sb_size = heap.get_size_class().sb_size;
    assert_eq!(sb_size & PAGE_MASK as u32, 0, "sb_size must be a multiple of a page");
    for index in 0..sb_size {
        unsafe {
            S_PAGE_MAP.set_page_info(ptr.offset(index as isize), info.clone())
        }
    }
}

pub fn register_desc(desc: &mut Descriptor) {
    let heap = if desc.proc_heap.is_null() {
        None
    } else {
        Some(unsafe {&mut *desc.proc_heap})
    };
    let ptr = desc.super_block;
    let size_class_index = 0;
    update_page_map(heap, ptr, Some(desc), size_class_index);
}

pub fn unregister_desc(heap: &mut ProcHeap, super_block: * mut u8) {
    update_page_map(Some(heap), super_block, None, 0)
}

pub fn get_page_info_for_ptr<T>(ptr: * const T) -> PageInfo {
    unsafe { S_PAGE_MAP.get_page_info(ptr) }.clone()
}

macro_rules! sc {
    ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, no, yes, $pgs:expr, $lg_delta_lookup:expr) => {
       {
        let index = $index + 1;
        let block_size = (1 << $lg_grp) + ($ndelta << $lg_delta);
        (index, block_size)
       }
    };
    ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, yes, yes, $pgs:expr, $lg_delta_lookup:expr) => {
       {
        let index = $index + 1;
        let block_size = (1 << $lg_grp) + ($ndelta << $lg_delta);
        (index, block_size)
       }
    };
    ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, yes, yes, $bin:expr, $pgs:expr, no) => {
       {
        let index = $index + 1;
        let block_size = (1 << $lg_grp) + ($ndelta << $lg_delta);
        (index, block_size)
       }
    };
    ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, no, yes, $bin:expr, $pgs:expr, no) => {
       {
        let index = $index + 1;
        let block_size = (1 << $lg_grp) + ($ndelta << $lg_delta);
        (index, block_size)
       }
    };
}

macro_rules! size_classes_match {
/*
    ([$(SizeClassData { block_size: $block:expr, sb:size: $pages:expr, block_num: 0, cache_block_num: 0, }),+]) => {

    };

 */


    ($name:ident, $diff:ident, sc($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:expr, $pgs:tt, $lg_delta_lookup:tt)) => {

        size_classes_match!(@ true, $name, $diff, found, (let mut found = false;), sc($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup))

    };
    ($name:ident, $diff:ident, sc($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:expr, $pgs:tt, $lg_delta_lookup:tt) $(, sc($($args:tt),*))*) => {

        size_classes_match!(@ true, $name, $diff, found, (let mut found = false;), sc($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup) $(, sc($($args),*))*)

    };

    (@ true, $name:ident, $diff:ident, $found:ident, ($($output:tt)*), sc ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:tt, $pgs:expr, $lg_delta_lookup:tt) $(, sc($($args:tt),*))*) => {
        size_classes_match!(@ false, $name, $diff, $found, (
             $($output)*
             if let (index_g, block_size) = size_classes_match!(@ sc ($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup)){
                if $name == index_g {
                    $name = $diff / block_size;
                    $found = true;
                }
             }
        ) $(, sc($($args),*))* )
    };
    (@ false, $name:ident, $diff:ident, $found:ident, ($($output:tt)*), sc ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:tt, $pgs:expr, $lg_delta_lookup:tt) $(, sc($($args:tt),*))*) => {
        size_classes_match!(@ false, $name, $diff, $found, (
             $($output)*
             else if let (index_g, block_size) = size_classes_match!(@ sc ($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup)){
                if $name == index_g {
                    $name = $diff / block_size;
                    $found = true;
                }
             }
        ) $(, sc($($args),*))* )
    };
    (@ $val:expr, $name: ident, $diff: ident, $found:ident, ($($arms:tt)*)) => {
        {
            $($arms)*
            if !$found {
                panic!("No size class found")
            }
            $found
        }
    };
    (@sc($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:tt, $pgs:expr, $lg_delta_lookup:tt)) => {
       {
        let index = $index + 1;
        let block_size = (1 << $lg_grp) + ($ndelta << $lg_delta);
        (index, block_size)
       }
    };
}

pub fn compute_index(super_block: * mut u8, block: * mut u8, size_class_index: usize) -> u32 {
    let sc = unsafe { &mut SIZE_CLASSES[size_class_index] };
    let _sc_block_size = sc.block_size;
    debug_assert!(block >= super_block);
    debug_assert!(block < unsafe { super_block.offset(sc.sb_size as isize )});
    let diff = block as u32 - super_block as u32;
    let index = 0;
    let found = size_classes_match![index, diff,
        sc(  0,      3,        3,      0,  no, yes,   1,  3),
        sc(  1,      3,        3,      1,  no, yes,   1,  3),
        sc(  2,      3,        3,      2,  no, yes,   3,  3),
        sc(  3,      3,        3,      3,  no, yes,   1,  3),
        sc(  4,      5,        3,      1,  no, yes,   5,  3),
        sc(  5,      5,        3,      2,  no, yes,   3,  3),
        sc(  6,      5,        3,      3,  no, yes,   7,  3),
        sc(  7,      5,        3,      4,  no, yes,   1,  3),
        sc(  8,      6,        4,      1,  no, yes,   5,  4),
        sc(  9,      6,        4,      2,  no, yes,   3,  4),
        sc( 10,      6,        4,      3,  no, yes,   7,  4),
        sc( 11,      6,        4,      4,  no, yes,   1,  4),
        sc( 12,      7,        5,      1,  no, yes,   5,  5),
        sc( 13,      7,        5,      2,  no, yes,   3,  5),
        sc( 14,      7,        5,      3,  no, yes,   7,  5),
        sc( 15,      7,        5,      4,  no, yes,   1,  5),
        sc( 16,      8,        6,      1,  no, yes,   5,  6),
        sc( 17,      8,        6,      2,  no, yes,   3,  6),
        sc( 18,      8,        6,      3,  no, yes,   7,  6),
        sc( 19,      8,        6,      4,  no, yes,   1,  6),
        sc( 20,      9,        7,      1,  no, yes,   5,  7),
        sc( 21,      9,        7,      2,  no, yes,   3,  7),
        sc( 22,      9,        7,      3,  no, yes,   7,  7),
        sc( 23,      9,        7,      4,  no, yes,   1,  7),
        sc( 24,     10,        8,      1,  no, yes,   5,  8),
        sc( 25,     10,        8,      2,  no, yes,   3,  8),
        sc( 26,     10,        8,      3,  no, yes,   7,  8),
        sc( 27,     10,        8,      4,  no, yes,   1,  8),
        sc( 28,     11,        9,      1,  no, yes,   5,  9),
        sc( 29,     11,        9,      2,  no, yes,   3,  9),
        sc( 30,     11,        9,      3,  no, yes,   7,  9),
        sc( 31,     11,        9,      4, yes, yes,   1,  9),
        sc( 32,     12,       10,      1,  no, yes,   5, no),
        sc( 33,     12,       10,      2,  no, yes,   3, no),
        sc( 34,     12,       10,      3,  no, yes,   7, no),
        sc( 35,     12,       10,      4, yes, yes,   2, no),
        sc( 36,     13,       11,      1,  no, yes,   5, no),
        sc( 37,     13,       11,      2, yes, yes,   3, no),
        sc( 38,     13,       11,      3,  no, yes,   7, no)
    ];
    debug_assert_eq!(diff / _sc_block_size, index);
    index

}



