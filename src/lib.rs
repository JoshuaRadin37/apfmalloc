#![allow(non_upper_case_globals)]

use crate::allocation_data::{get_heaps, Anchor, Descriptor, DescriptorNode, SuperBlockState};
use crate::mem_info::{MAX_SZ, MAX_SZ_IDX, align_val, PAGE, align_addr};
use std::ptr::null_mut;

use crate::size_classes::{get_size_class, init_size_class, SIZE_CLASSES};

use crate::page_map::S_PAGE_MAP;

use crate::alloc::{get_page_info_for_ptr, register_desc, unregister_desc, update_page_map};
use crate::pages::{page_alloc, page_free};
use atomic::{Atomic, Ordering};
use spin::Mutex;
use crate::bootstrap::{use_bootstrap, set_use_bootstrap, bootstrap_cache};
use std::cell::RefCell;
use core::mem::MaybeUninit;
use thread_local::ThreadLocal;
use crate::thread_cache::{ThreadCacheBin, fill_cache, flush_cache};
use std::thread::AccessError;
use std::sync::atomic::AtomicBool;

#[macro_use]
pub mod macros;
mod alloc;
mod allocation_data;
mod mem_info;
mod page_map;
mod pages;
mod size_classes;
mod thread_cache;
mod no_heap_mutex;
mod bootstrap;
pub mod auto_ptr;


#[macro_use]
extern crate bitfield;



static AVAILABLE_DESC: Mutex<DescriptorNode> = Mutex::new(DescriptorNode::new());

pub static mut MALLOC_INIT: AtomicBool = AtomicBool::new(false);
pub static mut MALLOC_FINISH_INIT: AtomicBool = AtomicBool::new(false);


unsafe fn init_malloc() {
    init_size_class();

    S_PAGE_MAP.init();

    for idx in 0..MAX_SZ_IDX {
        let heap = get_heaps().get_heap_at_mut(idx);

        heap.partial_list.store(None, Ordering::Release);
        heap.size_class_index = idx;
    }

    MALLOC_FINISH_INIT.store(true, Ordering::Release);
}

unsafe fn thread_local_init_malloc() {
    /*
    if thread_cache::thread_init.with(|f| *f.borrow()) {

    }

     */
    thread_cache::thread_init.with(|f| {
        let mut ref_mut = f.borrow_mut();
        *ref_mut = true;
    });

    thread_cache::thread_cache.with(|_f| {});
}



pub fn do_malloc(size: usize) -> *mut u8 {

    unsafe {
        if !MALLOC_INIT.compare_and_swap(false, true, Ordering::AcqRel) {
            init_malloc();
        }
        while !MALLOC_FINISH_INIT.load(Ordering::Relaxed) { }
    }

    if size > MAX_SZ {
        let pages = page_ceiling!(size);
        let desc = unsafe { &mut *Descriptor::alloc() };

        desc.proc_heap = null_mut();
        desc.block_size = pages as u32;
        desc.max_count = 1;
        desc.super_block = page_alloc(pages).expect("Should create");

        let mut anchor = Anchor::default();
        anchor.set_state(SuperBlockState::FULL);

        desc.anchor.store(anchor, Ordering::Release);

        register_desc(desc);
        let ptr = desc.super_block;
        return ptr;
    }

    let size_class_index = get_size_class(size);

    //thread_cache::thread_init.


    allocate_to_cache(size_class_index)
}

fn is_power_of_two(x: usize) -> bool {
    // https://stackoverflow.com/questions/3638431/determine-if-an-int-is-a-power-of-2-or-not-in-a-single-line
    (if x != 0 { true } else { false }) && (if (!(x & (x - 1))) != 0 { true } else { false })
}

pub fn do_aligned_alloc(align: usize, size: usize) -> *mut u8 {

    if !is_power_of_two(align) {
        return null_mut();
    }

    let mut size = align_val(size, align);

    unsafe {
        if !MALLOC_INIT.compare_and_swap(false, true, Ordering::AcqRel) {
            init_malloc();
        }
        while !MALLOC_FINISH_INIT.load(Ordering::Relaxed) { }
    }

    if size > PAGE {

        size = size.max(MAX_SZ + 1);

        let need_more_pages = align > PAGE;
        if need_more_pages {
            size += align;
        }

        let pages = page_ceiling!(size);

        let desc = unsafe {
            &mut *Descriptor::alloc()
        };

        let mut ptr = match page_alloc(pages) {
            Ok(ptr) => {ptr},
            Err(_) => null_mut(),
        };

        desc.proc_heap = null_mut();
        desc.block_size = pages as u32;
        desc.max_count = 1;
        desc.super_block = match page_alloc(pages) {
            Ok(ptr) => {ptr},
            Err(_) => null_mut(),
        };

        let mut anchor = Anchor::default();
        anchor.set_state(SuperBlockState::FULL);

        desc.anchor.store(anchor, Ordering::Release);


        register_desc(desc);


        if need_more_pages {
            ptr = align_addr(ptr as usize, align) as *mut u8;

            update_page_map(None, ptr, Some(desc), 0);
        }

        return ptr;
    }


    let size_class_index = get_size_class(size);

     /*

     The way this works pretty wild
     There is a global state of use_bootstrap

      */

    allocate_to_cache(size_class_index)
}

fn allocate_to_cache(size_class_index: usize) -> *mut u8 {
// Because of how rust creates thread locals, we have to assume the thread local does not exist yet
    // We also can't tell if a thread local exists without causing it to initialize, and when using
    // This as a global allocator, it ends up calling this function again. If not careful, we will create an
    // infinite recursion. As such, we must have a "bootstrap" bin that threads can use to initalize it's
    // own local bin

    // todo: remove the true
    if use_bootstrap() { // This is a global state, and tells to allocate from the bootstrap cache
        unsafe {
            // Gets and fills the correct cache from the bootstrap
            let mut bootstrap_cache_guard = bootstrap_cache.lock();
            let cache = bootstrap_cache_guard.get_mut(size_class_index).unwrap();
            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache);
            }

            cache.pop_block()
        }
    } else {
        set_use_bootstrap(true); // Sets the next allocation to use the bootstrap cache
        thread_cache::thread_init.with(|val| { // if not initalized, it goes back
            if !*val.borrow() { // the default value of the val is false, which means that the thread cache has not been created yet
                thread_cache::thread_cache.with(|tcache| { // This causes another allocation, hopefully with bootstrap
                    let _tcache = tcache;                                                    // There is a theoretical bootstrap data race here, but because
                });                                                                          // it repeatedly sets it false, eventually, it will allocate
                *val.borrow_mut() = true; // Never has to repeat this code after this
            }
            set_use_bootstrap(false) // Turns off the bootstrap
        });

        // If we are able to reach this piece of code, we know that the thread local cache is initalized
        thread_cache::thread_cache.with(|tcache| {
            let cache = unsafe {
                (*tcache.get()).get_mut(size_class_index).unwrap() // Gets the correct bin based on size class index
            };


            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache); // Fills the cache if necessary
                if cache.block_num == 0 {
                    panic!("Cache didn't fill");
                }
            }
            cache.pop_block() // Pops the block from the thread cache bin
        })
    }
}

fn do_malloc_aligned_from_bootstrap(align: usize, size: usize) -> *mut u8 {
    if !is_power_of_two(align) {
        return null_mut();
    }

    let mut size = align_val(size, align);

    unsafe {
        if !MALLOC_INIT.compare_and_swap(false, true, Ordering::AcqRel) {
            init_malloc();
        }
        while !MALLOC_FINISH_INIT.load(Ordering::Relaxed) { }
    }

    if size > PAGE {

        size = size.max(MAX_SZ + 1);

        let need_more_pages = align > PAGE;
        if need_more_pages {
            size += align;
        }

        let pages = page_ceiling!(size);

        let desc = unsafe {
            &mut *Descriptor::alloc()
        };

        let mut ptr = page_alloc(pages).expect("Error getting pages for aligned allocation");

        desc.proc_heap = null_mut();
        desc.block_size = pages as u32;
        desc.max_count = 1;
        desc.super_block = page_alloc(pages).expect("Should create");

        let mut anchor = Anchor::default();
        anchor.set_state(SuperBlockState::FULL);

        desc.anchor.store(anchor, Ordering::Release);


        register_desc(desc);


        if need_more_pages {
            ptr = align_addr(ptr as usize, align) as *mut u8;

            update_page_map(None, ptr, Some(desc), 0);
        }

        return ptr;
    }


    let size_class_index = get_size_class(size);

    unsafe {
        let mut bootstrap_cache_guard = bootstrap_cache.lock();
        let cache = bootstrap_cache_guard.get_mut(size_class_index).unwrap();
        if cache.get_block_num() == 0 {
            fill_cache(size_class_index, cache);
        }

        cache.pop_block()
    }

}

pub fn do_free<T>(ptr: *const T) {
    let info = get_page_info_for_ptr(ptr);
    let desc = unsafe { &mut *match info.get_desc() {
        Some(d) => { d},
        None => {
            // #[cfg(debug_assertions)]
                // println!("Free failed at {:?}", ptr);
                return; // todo: Band-aid fix
            // panic!("Descriptor not found for the pointer {:x?} with page info {:?}", ptr, info);
        }
    }};

   // #[cfg(debug_assertions)]
        // println!("Free will succeed at {:?}", ptr);

    let size_class_index = info.get_size_class_index();
    match size_class_index {
        None | Some(0) => {
            let super_block = desc.super_block;
            // unregister
            unregister_desc(None, super_block);

            // if large allocation
            if ptr as *const u8 != super_block as *const u8 {
                unregister_desc(None, ptr as *mut u8)
            }

            // free the super block
            page_free(super_block);

            // retire the descriptor
            desc.retire();
        }
        Some(size_class_index) => {
            let force_bootstrap = use_bootstrap() || match thread_cache::thread_init.try_with(|_| {}) {
                Ok(_) => {
                    false
                },
                Err(_) => {
                    true
                },
            };
            // todo: remove true
            if force_bootstrap {
                unsafe {
                    let mut bootstrap_cache_guard = bootstrap_cache.lock();
                    let cache = bootstrap_cache_guard.get_mut(size_class_index).unwrap();
                    let sc = &SIZE_CLASSES[size_class_index];

                    if cache.get_block_num() >= sc.cache_block_num {
                        flush_cache(size_class_index, cache);
                    }

                    cache.push_block(ptr as *mut u8);
                }
            } else {
                set_use_bootstrap(true);
                thread_cache::thread_init.with(|val| {
                    if !*val.borrow() {
                        thread_cache::thread_cache.with(|tcache| {
                            let _tcache = tcache;
                        });
                        *val.borrow_mut() = true;
                    }
                    set_use_bootstrap(false)
                });

                thread_cache::thread_cache.try_with(|tcache| {
                    let cache = unsafe {
                        (*tcache.get()).get_mut(size_class_index).unwrap()
                    };
                    let sc = unsafe { &SIZE_CLASSES[size_class_index] };
                    /*
                    if sc.block_num == 0 {
                        unsafe {
                            let mut guard = bootstrap_cache.lock();
                            let cache = guard.get_mut(size_class_index).unwrap();


                            if cache.get_block_num() >= sc.cache_block_num {
                                flush_cache(size_class_index, cache);
                            }

                            return cache.push_block(ptr as *mut u8);
                        }
                    }

                     */

                    if cache.get_block_num() >= sc.cache_block_num {
                        flush_cache(size_class_index, cache);
                    }

                    return cache.push_block(ptr as *mut u8);
                });
            }
        },
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::allocation_data::get_heaps;
    use bitfield::size_of;
    use core::mem::MaybeUninit;

    #[test]
    fn heaps_valid() {
        let heap = get_heaps();
        let _p_heap = heap.get_heap_at_mut(0);
    }

    #[test]
    fn malloc_and_free() {
        let ptr = unsafe { &mut *(super::do_malloc(size_of::<usize>()) as *mut MaybeUninit<usize>) };
        *ptr = MaybeUninit::new(8);
        assert_eq!(& unsafe { *(ptr as * const MaybeUninit<usize> as * const u8) }, &8); // should be trivial
        do_free(ptr as *mut MaybeUninit<usize>);
    }

    #[test]
    fn malloc_and_free_large() {
        let ptr = super::do_malloc(MAX_SZ * 2);
        do_free(ptr);
    }
}
