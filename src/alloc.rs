use crate::allocation_data::{
    get_heaps, Anchor, Descriptor, DescriptorNode, ProcHeap, SuperBlockState,
};
use crate::mem_info::{PAGE, PAGE_MASK};
use crate::page_map::{PageInfo, S_PAGE_MAP};
use crate::size_classes::SIZE_CLASSES;
use crate::thread_cache::ThreadCacheBin;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

use crate::pages::external_mem_reservation::{SegAllocator, Segment, SEGMENT_ALLOCATOR};

pub fn list_pop_partial(heap: &mut ProcHeap) -> Option<&mut Descriptor> {
    let list = &heap.partial_list;

    loop {
        let old_head = list.load(Ordering::Acquire);
        old_head?;
        let old_desc = old_head.unwrap().get_desc().unwrap();
        // Lets assume this descriptor exists

        let mut new_head: Option<DescriptorNode> = old_desc.next_partial.load(Ordering::Acquire);
        match &mut new_head {
            None => {
                if let Some(mut new_head_desc) = &mut new_head {
                    new_head_desc.set(None, 0);
                }
            }
            Some(descriptor_node) => {
                let desc = descriptor_node.get_desc().unwrap();
                let counter = descriptor_node.get_counter();
                if let Some(mut new_head_desc) = &mut new_head {
                    new_head_desc.set(Some(desc), counter);
                }
            }
        }

        if let Ok(_) =
        list.compare_exchange_weak(old_head, new_head, Ordering::Acquire, Ordering::Relaxed)
        {
            return Some(old_desc);
        }
    }
}

pub fn list_push_partial(desc: &'static mut Descriptor) {
    //info!("Pushing descriptor (Count = {}) to heap", desc.anchor.load(Ordering::Acquire).count());
    let heap = desc.proc_heap;
    let list = unsafe { &(*heap).partial_list };

    /*
    let mut replace: bool = false;
    let old_head = match list.load(Ordering::Acquire) {
        None => {
            let mut node = DescriptorNode::new();
            node.set(Some(desc), 0);
            replace = true;
            //info!("Heap is empty, starting descriptor list");
            node
        }
        Some(list) => {
            //info!("Pushing to start of descriptor list");
            list
        },
    };

     */

    // let old_head = list.load(Ordering::Acquire).unwrap();
    let mut new_head = DescriptorNode::default();

    loop {
        let old_head = list.load(Ordering::Acquire);
        /*
        new_head.set(
            Some(desc),
            old_head.get_counter().expect("Old heap should exist") + 1,
        );

         */
        new_head.set(
            Some(desc),
            if old_head.is_none() {
                0
            } else {
                old_head.unwrap().get_counter() + 1
            },
        );
        // debug_assert_ne!(old_head.get_desc(), new_head.get_desc());
        desc.next_partial.store(old_head, Ordering::Release);

        /*if replace {
            list.store(Some(new_head), Ordering::Acquire);
            break;
        }
         else
         */
        if list
            .compare_exchange_weak(
                old_head,
                Some(new_head),
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            /*info!("Heap partial list anchor updated to {:?}", unsafe { &(*heap).partial_list }
               .load(Ordering::Acquire)
               .unwrap()
               .get_desc()
               .anchor.load(Ordering::Acquire));

            */
            break;
        }
    }
}

pub fn heap_push_partial(desc: &'static mut Descriptor) {
    list_push_partial(desc)
}

pub fn heap_pop_partial(heap: &mut ProcHeap) -> Option<&mut Descriptor> {
    list_pop_partial(heap)
}

pub fn malloc_from_partial(
    size_class_index: usize,
    cache: &mut ThreadCacheBin,
    block_num: &mut usize,
) {
    let heap = get_heaps().get_heap_at_mut(size_class_index);
    let desc = heap_pop_partial(heap);

    match desc {
        None => {}
        Some(desc) => {
            //info!("Allocating blocks from a partial list...");
            let old_anchor = desc.anchor.load(Ordering::Acquire);
            let mut new_anchor: Anchor;

            let max_count = desc.max_count;
            let block_size = desc.block_size;

            let super_block = &desc.super_block;

            loop {
                if old_anchor.state() == SuperBlockState::EMPTY {
                    //info!("Retiring an empty descriptor");
                    desc.retire();
                    return malloc_from_partial(size_class_index, cache, block_num);
                }

                // debug_assert_eq!(old_anchor.state(), SuperBlockState::PARTIAL);
                //info!("Found old anchor {:?}", old_anchor);
                new_anchor = old_anchor;
                new_anchor.set_count(0);
                new_anchor.set_avail(max_count as u64);
                new_anchor.set_state(SuperBlockState::FULL);

                if desc
                    .anchor
                    .compare_exchange(old_anchor, new_anchor, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }

            let blocks_taken = old_anchor.count() as isize;
            let avail = old_anchor.avail() as isize;
            let super_block = super_block.as_ref().unwrap().get_ptr() as *mut u8;
            assert!(
                avail <= max_count as isize,
                "Avail: {}, Count {}",
                avail,
                max_count
            );
            let block = unsafe { super_block.offset(avail * block_size as isize) };
            assert_eq!(cache.get_block_num(), 0);
            cache.push_list(block, blocks_taken as u32);
            *block_num += 1;
        }
    }
}

pub fn malloc_from_new_sb(
    size_class_index: usize,
    cache: &mut ThreadCacheBin,
    block_num: &mut usize,
) {
    let heap = get_heaps().get_heap_at_mut(size_class_index);
    let sc = unsafe { &SIZE_CLASSES[size_class_index] };

    let desc = unsafe { &mut *Descriptor::alloc() };
    // debug_assert!(!desc.is_null());

    let block_size = sc.block_size;
    let max_count = sc.get_block_num();

    desc.proc_heap = heap;
    desc.block_size = block_size;
    desc.max_count = max_count as u32;
    desc.super_block = SEGMENT_ALLOCATOR.allocate(sc.sb_size as usize).ok();

    let super_block = desc.super_block.as_ref().unwrap().get_ptr() as *mut u8;

    #[cfg(not(feature = "no_met_stack"))]
        {
            unsafe {
                // Gets a pointer to the last block and sets it to 0xFF...F
                let ptr = super_block.add(block_size as usize * max_count - block_size as usize);
                *(ptr as *mut usize) = std::usize::MAX;
            }
        }
    #[cfg(feature = "no_met_stack")]
        {
            unsafe {
                // let mut ptr = super_block.add(block_size as usize * max_count - block_size as usize) as *mut *mut u8;
                let mut last: *mut u8 = null_mut();
                for block_num in (0..desc.max_count).rev() {
                    let current = super_block.add((block_size * block_num) as usize);
                    *(current as *mut *mut u8) = last;
                    last = current;
                    // ptr = current as *mut *mut u8;
                }
            }
        }

    let block = super_block;
    cache.push_list(block, max_count as u32);

    let mut anchor: Anchor = Anchor::default();
    anchor.set_avail(max_count as u64);
    anchor.set_count(0);
    anchor.set_state(SuperBlockState::FULL);

    desc.anchor.store(anchor, Ordering::SeqCst);

    register_desc(desc);
    *block_num += max_count;
}

/* WARNING -- ELIAS CODE -- WARNING */

pub fn malloc_count_from_partial(
    size_class_index: usize,
    cache: &mut ThreadCacheBin,
    block_num: &mut usize,
    count: usize,
) {
    let heap = get_heaps().get_heap_at_mut(size_class_index);
    let desc = heap_pop_partial(heap);

    match desc {
        None => {
            return;
        }
        Some(desc) => {
            //info!("Allocating blocks from a partial list...");
            let old_anchor = desc.anchor.load(Ordering::Acquire);
            let mut new_anchor: Anchor;

            let max_count = desc.max_count;
            let block_size = desc.block_size;

            let super_block = &desc.super_block;

            let avail = old_anchor.avail() as isize;
            let new_avail = match avail + (count as isize) < max_count as isize {
                true => avail + count as isize,
                false => max_count as isize,
            };

            loop {
                if old_anchor.state() == SuperBlockState::EMPTY {
                    //info!("Retiring an empty descriptor");
                    desc.retire();
                    return malloc_from_partial(size_class_index, cache, block_num);
                }

                // debug_assert_eq!(old_anchor.state(), SuperBlockState::PARTIAL);
                //info!("Found old anchor {:?}", old_anchor);
                new_anchor = old_anchor;
                new_anchor.set_count(0);
                new_anchor.set_avail(new_avail as u64);
                new_anchor.set_state(SuperBlockState::FULL);

                if desc
                    .anchor
                    .compare_exchange(old_anchor, new_anchor, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }

            assert!(
                avail <= max_count as isize,
                "Avail: {}, Count {}",
                avail,
                max_count
            );

            let c = new_avail - avail;
            let super_block = super_block.as_ref().unwrap().get_ptr() as *mut u8;
            for i in 0..c {
                let block =
                    unsafe { super_block.offset((avail + i as isize) * block_size as isize) };
                cache.push_block(block);
                *block_num += 1;
            }
        }
    }
}

pub fn malloc_count_from_new_sb(
    size_class_index: usize,
    cache: &mut ThreadCacheBin,
    block_num: &mut usize,
    count: usize,
) {
    let heap = get_heaps().get_heap_at_mut(size_class_index);
    let sc = unsafe { &SIZE_CLASSES[size_class_index] };

    let desc = unsafe { &mut *Descriptor::alloc() };
    // debug_assert!(!desc.is_null());

    let block_size = sc.block_size;
    let max_count = sc.get_block_num();

    let c = max_count.min(count);

    desc.proc_heap = heap;
    desc.block_size = block_size;
    desc.max_count = max_count as u32;
    desc.super_block = SEGMENT_ALLOCATOR.allocate(sc.block_size as usize * c).ok();

    let super_block = desc.super_block.as_ref().unwrap().get_ptr() as *mut u8;



    #[cfg(not(feature = "no_met_stack"))]
        {
            unsafe {
                // Gets a pointer to the last block and sets it to 0xFF...F
                let ptr = super_block.add(block_size as usize * c - block_size as usize);
                *(ptr as *mut usize) = std::usize::MAX;
            }
        }
    #[cfg(feature = "no_met_stack")]
        {
            unsafe {
                // let mut ptr = super_block.add(block_size as usize * max_count - block_size as usize) as *mut *mut u8;
                let mut last: *mut u8 = null_mut();
                for block_num in (0..c).rev() {
                    let current = super_block.add((block_size as usize * block_num) as usize);
                    *(current as *mut *mut u8) = last;
                    last = current;
                    // ptr = current as *mut *mut u8;
                }
            }
        }

    let block = super_block;
    cache.push_list(block, c as u32);

    let mut anchor: Anchor = Anchor::default();
    anchor.set_avail(c as u64);
    anchor.set_count(0);
    anchor.set_state(SuperBlockState::FULL);

    desc.anchor.store(anchor, Ordering::SeqCst);

    register_desc(desc);
    *block_num += max_count;
}

/* END ELIAS CODE */

pub fn update_page_map(
    heap: Option<&mut ProcHeap>,
    ptr: *mut u8,
    desc: Option<&mut Descriptor>,
    size_class_index: usize,
) {
    if ptr.is_null() {
        panic!("Pointer should not be null");
    }

    let mut info: PageInfo = PageInfo::default();
    info.set_ptr(
        desc.map_or(null_mut(), |d| d as *mut Descriptor),
        size_class_index,
    );
    if heap.is_none() {
        unsafe {
            S_PAGE_MAP.set_page_info(ptr, info);
            return;
        }
    }

    let heap = heap.unwrap();
    let sb_size = heap.get_size_class().sb_size;
    assert_eq!(
        sb_size & PAGE_MASK as u32,
        0,
        "sb_size must be a multiple of a page"
    );
    for index in 0..(sb_size / PAGE as u32) {
        unsafe {
            S_PAGE_MAP.set_page_info(ptr.offset((index * PAGE as u32) as isize), info.clone())
        }
    }
}

pub fn register_desc(desc: &mut Descriptor) {
    let heap = if desc.proc_heap.is_null() {
        None
    } else {
        Some(unsafe { &mut *desc.proc_heap })
    };
    let ptr = desc.super_block.as_ref().unwrap().get_ptr() as *mut u8;
    let size_class_index = 0;
    update_page_map(heap, ptr, Some(desc), size_class_index);
}

pub fn unregister_desc(heap: Option<&mut ProcHeap>, super_block: &Segment) {
    update_page_map(heap, super_block.get_ptr() as *mut u8, None, 0)
}

pub fn get_page_info_for_ptr<T: ?Sized>(ptr: *const T) -> PageInfo {
    unsafe { S_PAGE_MAP.get_page_info(ptr) }.clone()
}

macro_rules! size_classes_match {
/*
    ([$(SizeClassData { block_size: $block:expr, sb:size: $pages:expr, block_num: 0, cache_block_num: 0, }),+]) => {

    };

 */


    ($name:ident, $diff:ident, $sc_index:expr, sc($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:expr, $pgs:tt, $lg_delta_lookup:tt)) => {

        #[allow(irrefutible)]
        size_classes_match!(@ true, $name, $diff, $sc_index, found, (let mut found = false;), sc($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup))

    };
    ($name:ident, $diff:ident, $sc_index:expr, sc($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:expr, $pgs:tt, $lg_delta_lookup:tt) $(, sc($($args:tt),*))*) => {

        size_classes_match!(@ true, $name, $diff, $sc_index, found, (let mut found = false;), sc($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup) $(, sc($($args),*))*)

    };

    (@ true, $name:ident, $diff:ident, $sc_index:expr, $found:ident, ($($output:tt)*), sc ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:tt, $pgs:expr, $lg_delta_lookup:tt) $(, sc($($args:tt),*))*) => {
        size_classes_match!(@ false, $name, $diff, $sc_index, $found, (
             $($output)*
             {
                 let (index_g, block_size) = size_classes_match!(@ sc ($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup));
                 if $sc_index == index_g {
                        $name = $diff / block_size;
                        $found = true;
                 }
            }
        ) $(, sc($($args),*))* )
    };
    (@ false, $name:ident, $diff:ident, $sc_index:expr, $found:ident, ($($output:tt)*), sc ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:tt, $bin:tt, $pgs:expr, $lg_delta_lookup:tt) $(, sc($($args:tt),*))*) => {
        size_classes_match!(@ false, $name, $diff, $sc_index, $found, (
             $($output)*
             if !$found {
             let (index_g, block_size) = size_classes_match!(@ sc ($index, $lg_grp, $lg_delta, $ndelta, $psz, $bin, $pgs, $lg_delta_lookup));
                if $sc_index == index_g {
                    $name = $diff / block_size;
                    $found = true;
                }
             }
        ) $(, sc($($args),*))* )
    };
    (@ $val:expr, $name: ident, $diff: ident, $sc_index:expr, $found:ident, ($($arms:tt)*)) => {
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

pub fn compute_index(super_block: *mut u8, block: *mut u8, size_class_index: usize) -> u32 {
    let sc = unsafe { &mut SIZE_CLASSES[size_class_index] };
    let _sc_block_size = sc.block_size;
    debug_assert!(block >= super_block);
    debug_assert!(block < unsafe { super_block.offset(sc.sb_size as isize) });
    let diff = block as u32 - super_block as u32;
    let mut index = 0;
    let _found = size_classes_match![
        index,
        diff,
        size_class_index,
        sc(0, 3, 3, 0, no, yes, 1, 3),
        sc(1, 3, 3, 1, no, yes, 1, 3),
        sc(2, 3, 3, 2, no, yes, 3, 3),
        sc(3, 3, 3, 3, no, yes, 1, 3),
        sc(4, 5, 3, 1, no, yes, 5, 3),
        sc(5, 5, 3, 2, no, yes, 3, 3),
        sc(6, 5, 3, 3, no, yes, 7, 3),
        sc(7, 5, 3, 4, no, yes, 1, 3),
        sc(8, 6, 4, 1, no, yes, 5, 4),
        sc(9, 6, 4, 2, no, yes, 3, 4),
        sc(10, 6, 4, 3, no, yes, 7, 4),
        sc(11, 6, 4, 4, no, yes, 1, 4),
        sc(12, 7, 5, 1, no, yes, 5, 5),
        sc(13, 7, 5, 2, no, yes, 3, 5),
        sc(14, 7, 5, 3, no, yes, 7, 5),
        sc(15, 7, 5, 4, no, yes, 1, 5),
        sc(16, 8, 6, 1, no, yes, 5, 6),
        sc(17, 8, 6, 2, no, yes, 3, 6),
        sc(18, 8, 6, 3, no, yes, 7, 6),
        sc(19, 8, 6, 4, no, yes, 1, 6),
        sc(20, 9, 7, 1, no, yes, 5, 7),
        sc(21, 9, 7, 2, no, yes, 3, 7),
        sc(22, 9, 7, 3, no, yes, 7, 7),
        sc(23, 9, 7, 4, no, yes, 1, 7),
        sc(24, 10, 8, 1, no, yes, 5, 8),
        sc(25, 10, 8, 2, no, yes, 3, 8),
        sc(26, 10, 8, 3, no, yes, 7, 8),
        sc(27, 10, 8, 4, no, yes, 1, 8),
        sc(28, 11, 9, 1, no, yes, 5, 9),
        sc(29, 11, 9, 2, no, yes, 3, 9),
        sc(30, 11, 9, 3, no, yes, 7, 9),
        sc(31, 11, 9, 4, yes, yes, 1, 9),
        sc(32, 12, 10, 1, no, yes, 5, no),
        sc(33, 12, 10, 2, no, yes, 3, no),
        sc(34, 12, 10, 3, no, yes, 7, no),
        sc(35, 12, 10, 4, yes, yes, 2, no),
        sc(36, 13, 11, 1, no, yes, 5, no),
        sc(37, 13, 11, 2, yes, yes, 3, no),
        sc(38, 13, 11, 3, no, yes, 7, no)
    ];
    debug_assert_eq!(diff / _sc_block_size, index);
    index
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mem_info::MAX_SZ_IDX;
    use crate::MALLOC_INIT_S;

    #[test]
    fn from_new_sb() {
        let mut tcache = [ThreadCacheBin::new(); MAX_SZ_IDX];
        MALLOC_INIT_S.with(|| unsafe {
            crate::init_malloc();
        });

        let cache = &mut tcache[1];
        malloc_from_new_sb(1, cache, &mut 0);
        assert!(cache.block_num > 0);
    }
}
