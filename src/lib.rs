#![allow(non_upper_case_globals)]

use crate::allocation_data::{get_heaps, Anchor, Descriptor, DescriptorNode, SuperBlockState};
use crate::mem_info::{MAX_SZ, MAX_SZ_IDX, align_val, PAGE, align_addr};
use lazy_static::lazy_static;
use std::ptr::null_mut;

use crate::size_classes::{get_size_class, init_size_class, SIZE_CLASSES};

use crate::page_map::S_PAGE_MAP;

use crate::alloc::{fill_cache, flush_cache, get_page_info_for_ptr, register_desc, unregister_desc, update_page_map};
use crate::pages::{page_alloc, page_free};
use atomic::{Atomic, Ordering};
use spin::Mutex;
use crate::bootstrap::{use_bootstrap, set_use_bootstrap, bootstrap_cache};
use std::cell::RefCell;

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

mod apf;

#[macro_use]
extern crate bitfield;

static AVAILABLE_DESC: Mutex<DescriptorNode> = Mutex::new(DescriptorNode::new());

pub static mut MALLOC_INIT: bool = false;

unsafe fn init_malloc() {
    MALLOC_INIT = true;
    init_size_class();

    S_PAGE_MAP.init();

    for idx in 0..MAX_SZ_IDX {
        let heap = get_heaps().get_heap_at_mut(idx);

        heap.partial_list.store(None, Ordering::Release);
        heap.size_class_index = idx;
    }
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
    static thread_localized: Mutex<bool> = Mutex::new(false);
    static do_thread_bootstrap: Mutex<bool> = Mutex::new(false);
    unsafe {
        if !MALLOC_INIT {
            init_malloc();
        }
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

        desc.anchor.store(anchor, Ordering::Acquire);

        register_desc(desc);
        let ptr = desc.super_block;
        return ptr;
    }

    let size_class_index = get_size_class(size);

    //thread_cache::thread_init.

    if !*thread_localized.lock() && use_bootstrap() {
        unsafe {
            let cache = &mut bootstrap_cache[size_class_index];
            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache);
            }

            cache.pop_block()
        }
    } else {
        set_use_bootstrap(true);
        thread_cache::thread_cache.with(|tcache| {
            *thread_localized.lock() = true;
            let cache = &mut tcache.borrow_mut()[size_class_index];
            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache);
            }


            if use_bootstrap() {
                set_use_bootstrap(false);
            }

            cache.pop_block()
        })
    }
}

fn is_power_of_two(x: usize) -> bool {
    // https://stackoverflow.com/questions/3638431/determine-if-an-int-is-a-power-of-2-or-not-in-a-single-line
    (if x != 0 { true } else { false }) && (if (!(x & (x - 1))) != 0 { true } else { false })
}

pub fn do_aligned_alloc(align: usize, size: usize) -> *mut u8 {
    static thread_localized: Mutex<bool> = Mutex::new(false);
    if !is_power_of_two(align) {
        return null_mut();
    }

    let mut size = align_val(size, align);

    unsafe {
        if !MALLOC_INIT {
            init_malloc();
        }
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

    //thread_cache::thread_init.

    if !*thread_localized.lock() && use_bootstrap() {
        unsafe {
            let cache = &mut bootstrap_cache[size_class_index];
            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache);
            }

            cache.pop_block()
        }
    } else {
        set_use_bootstrap(true);
        thread_cache::thread_cache.with(|tcache| {
            *thread_localized.lock() = true;
            let cache = &mut tcache.borrow_mut()[size_class_index];
            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache);
            }


            if use_bootstrap() {
                set_use_bootstrap(false);
            }

            cache.pop_block()
        })
    }
}

pub fn do_free<T>(ptr: *const T) {
    let info = get_page_info_for_ptr(ptr);
    let desc = unsafe { &mut *match info.get_desc() {
        Some(d) => { d},
        None => {
            #[cfg(debug_assertions)]
            println!("Free failed at {:?}", ptr);
            return; // todo: Band-aid fix
            // panic!("Descriptor not found for the pointer {:x?} with page info {:?}", ptr, info);
        }
    }};

    #[cfg(debug_assertions)]
    // println!("Free will succeed at {:?}", ptr);

    let size_class_index = info.get_size_class_index();
    match size_class_index {
        None => {
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
            if use_bootstrap() {
                unsafe {
                    let cache = &mut bootstrap_cache[size_class_index];
                    let sc = &SIZE_CLASSES[size_class_index];

                    if cache.get_block_num() >= sc.cache_block_num {
                        flush_cache(size_class_index, cache);
                    }

                    cache.push_block(ptr as *mut u8);
                }
            } else {
                thread_cache::thread_cache.with(|tcache| {
                    let cache = &mut tcache.borrow_mut()[size_class_index];
                    let sc = unsafe { &SIZE_CLASSES[size_class_index] };

                    if cache.get_block_num() >= sc.cache_block_num {
                        flush_cache(size_class_index, cache);
                    }

                    cache.push_block(ptr as *mut u8);
                })
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
}
