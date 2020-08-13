#![allow(non_upper_case_globals)]
#![deny(unused_imports)]

#[macro_use]
extern crate bitfield;

use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::AtomicUsize;

use atomic::Ordering;
use spin::Mutex;

use crate::alloc::{get_page_info_for_ptr, register_desc, unregister_desc, update_page_map};
use crate::allocation_data::{Anchor, Descriptor, DescriptorNode, get_heaps, SuperBlockState};
use crate::bootstrap::{bootstrap_reserve, use_bootstrap, set_use_bootstrap};
use crate::mem_info::{align_addr, align_size, MAX_SZ, MAX_SZ_IDX, PAGE};
use crate::pages::{SegAllocator, SEGMENT_ALLOCATOR};
use crate::single_access::SingleAccess;
use crate::size_classes::{get_size_class, init_size_class, SIZE_CLASSES};
use crate::thread_cache::{fill_cache, flush_cache};

#[macro_export]
macro_rules! dump_info {
    () => {
        #[cfg(feature = "track_allocation")]
        $crate::info_dump::print_info_dump()
    };
}

#[macro_use]
pub mod macros;

pub mod alloc;
pub mod allocation_data;
pub mod independent_collections;
#[cfg(feature = "track_allocation")]
pub mod info_dump;
#[allow(unused)]
pub mod mem_info;
pub mod page_map;
pub mod pages;
pub mod single_access;
pub mod size_classes;
pub mod thread_cache;

mod bootstrap;

pub mod ptr {
    pub mod auto_ptr;
    pub mod rc;
}

pub mod apf;

static AVAILABLE_DESC: Mutex<DescriptorNode> = Mutex::new(DescriptorNode::new());

pub static IN_CACHE: AtomicUsize = AtomicUsize::new(0);
pub static IN_BOOTSTRAP: AtomicUsize = AtomicUsize::new(0);

static MALLOC_INIT_S: SingleAccess = SingleAccess::new();

static mut USE_APF: bool = true;

/// Tells the allocator to remember the total amount allocated to a thread cache or the bootstrap. Only available in builds
/// with debug_assertions
#[cfg(debug_assertions)]
pub const TRACK_ALLOCATION_LOCATION: bool = false;

/// Initializes malloc. Only needs to ran once for the entire program, and manually running it again will cause all of the memory saved
/// in the central reserve to be lost
unsafe fn init_malloc() {
    bootstrap_reserve.lock().init();

    init_size_class();

    // S_PAGE_MAP.init();
    // HASH_PAGE_MAP.init();


    let option = std::env::var("USE_APF");
    if option == Ok("false".to_string()) {
        USE_APF = false;
    }

    for idx in 0..MAX_SZ_IDX {
        let heap = get_heaps().get_heap_at_mut(idx);

        heap.partial_list.store(None, Ordering::Release);
        heap.size_class_index = idx;
    }

    set_use_bootstrap(false);
    //info!("Malloc Initialized")
}

/// Performs an aligned allocation for type `T`. Type `T` must be `Sized`
///
/// # Example
/// ```
/// use apfmalloc_lib::allocate_type;
/// let ptr = allocate_type::<usize>();
/// unsafe {
///     ptr.write(100);
/// }
/// ```
///
/// # Safety
/// Although this function can not cause undefined behavior, the pointer created by this method should be deallocated using
/// [`do_free()`](fn.do_free.html)
pub fn allocate_type<T>() -> *mut T {
    let size = std::mem::size_of::<T>();
    let align = std::mem::align_of::<T>();
    do_aligned_alloc(align, size) as *mut T
}

/// Allocates a space in memory with correct alignment for type `T`, then sets the value at the pointed location to `val`
///
/// # Safety
/// If for whatever reason, the pointer created is a NULL pointer, this function will not attempt to write to the pointer,
/// so no SEGFAULTS should occur
pub fn allocate_val<T>(val: T) -> *mut T {
    let ret = allocate_type::<T>();
    if !ret.is_null() {
        unsafe {
            ret.write(val);
        }
    }
    ret
}

/// Allocates a space in memory of length `size`.
///
/// If the allocation fails, a NULL pointer is returned.
pub fn do_malloc(size: usize) -> *mut u8 {

    MALLOC_INIT_S.with(
        || unsafe { init_malloc() },
        // || unsafe { ret = bootstrap_reserve.lock().allocate(size) }
    );

    /*
    unsafe {
        if !MALLOC_SKIP {
            if !MALLOC_INIT.compare_and_swap(false, true, Ordering::AcqRel) {
                init_malloc();
            }
            while !MALLOC_FINISH_INIT.load(Ordering::Relaxed) {}
        }
    }

     */

    if size > MAX_SZ {
        let pages = page_ceiling!(size);
        let desc = unsafe { &mut *Descriptor::alloc() };

        desc.proc_heap = null_mut();
        desc.block_size = pages as u32;
        desc.max_count = 1;
        desc.super_block = SEGMENT_ALLOCATOR.allocate(pages).ok();

        let mut anchor = Anchor::default();
        anchor.set_state(SuperBlockState::FULL);

        desc.anchor.store(anchor, Ordering::Release);

        register_desc(desc);
        let ptr = desc.super_block.as_ref().unwrap().get_ptr() as *mut u8;
        // Log malloc with tuner
        return ptr;
    }

    let size_class_index = get_size_class(size);

    allocate_to_cache(size, size_class_index)
}

fn is_power_of_two(x: usize) -> bool {
    // https://stackoverflow.com/questions/3638431/determine-if-an-int-is-a-power-of-2-or-not-in-a-single-line
    (if x != 0 { true } else { false }) && (if (!(x & (x - 1))) != 0 { true } else { false })
}



/// Allocates a space in memory of length `size`, that is aligned to `align`. `align` must be a power of 2.
///
/// If the allocation fails, a NULL pointer is returned.
pub fn do_aligned_alloc(align: usize, size: usize) -> *mut u8 {
    if !is_power_of_two(align) {
        return null_mut();
    }

    let mut size = align_size(size, align);
    MALLOC_INIT_S.with(
        || unsafe { init_malloc() },
        // || unsafe { ret = bootstrap_reserve.lock().allocate(size) }
    );

    if size > PAGE {
        size = size.max(MAX_SZ + 1);

        let need_more_pages = align > PAGE;
        if need_more_pages {
            size += align;
        }

        let pages = page_ceiling!(size);

        let seg = match SEGMENT_ALLOCATOR.allocate(pages) {
            Ok(seg) => seg,
            Err(_) => {
                return null_mut();
            }
        };

        let desc = unsafe { &mut *Descriptor::alloc() };

        desc.proc_heap = null_mut();
        desc.block_size = pages as u32;
        desc.max_count = 1;
        desc.super_block = Some(seg);

        let mut ptr = desc.super_block.as_ref().unwrap().get_ptr() as *mut u8;

        let mut anchor = Anchor::default();
        anchor.set_state(SuperBlockState::FULL);

        desc.anchor.store(anchor, Ordering::Release);

        register_desc(desc);

        if need_more_pages {
            ptr = align_addr(ptr as usize, align) as *mut u8;

            unsafe {
                /*
                let mut guard = RANGE_PAGE_MAP.write().unwrap();
                guard.update_page_map(None, ptr, Some(desc), 0);

                 */
                update_page_map(None, ptr, Some(desc), 0);
            }
        }

        return ptr;
    }

    let size_class_index = get_size_class(size);

    allocate_to_cache(size, size_class_index)
}

/// Requests either a memory of length `size` from the bootstrap or a block from a reserve of the
/// `size_class_index` size class. The memory is only allocated from bootstrap if a global variable is
/// set.
///
/// By default, memory is reserved from a thread cache. To reserve from the bootstrap, first the user must
/// pass true to [`set_use_bootstrap()`](fn.set_use_bootstrap.html)
/// # Safety
/// This function is safe, as if for whatever reason memory can not be reserved, a NULL pointer is returned
pub fn allocate_to_cache(size: usize, size_class_index: usize) -> *mut u8 {
    // dbg!(size_class_index);
    // Because of how rust creates thread locals, we have to assume the thread local does not exist yet
    // We also can't tell if a thread local exists without causing it to initialize, and when using
    // This as a global allocator, it ends up calling this function again. If not careful, we will create an
    // infinite recursion. As such, we must have a "bootstrap" bin that threads can use to initalize it's
    // own local bin

    //let id = thread::current();
    let panic_status = std::thread::panicking();

    if panic_status || use_bootstrap() {
        // This is a global state, and tells to allocate from the bootstrap cache
        /*
        unsafe {
            // Gets and fills the correct cache from the bootstrap
            let mut bootstrap_cache_guard = bootstrap_cache.lock();
            let cache = bootstrap_cache_guard.get_mut(size_class_index).unwrap();
            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache);
            }

            cache.pop_block()
        }
         */
        #[cfg(debug_assertions)]
        if TRACK_ALLOCATION_LOCATION {
            IN_BOOTSTRAP.fetch_add(size, Ordering::AcqRel);
        }

        unsafe { bootstrap_reserve.lock().allocate(size) }
    } else {
        /*

        This has been commented out because the bootstrapping problem has been solved by way of
        the Copy/Drop thread stack difference.

        Thus, initialization of thread caches can be done without using any heap allocated memory,
        which would cause infinite recursion. Although this was originally solved using a "bootstrap" cache,
        this would cause the program to be very slow and all threads to get memory from the bootstrap at
        the same time.


        #[cfg(not(unix))]
        {
            set_use_bootstrap(true); // Sets the next allocation to use the bootstrap cache
                                     //WAIT_FOR_THREAD_INIT.store(Some(thread::current().id()));
            thread_cache::thread_init.with(|val| {
                // if not initalized, it goes back
                if !*val.borrow() {
                    // the default value of the val is false, which means that the thread cache has not been created yet
                    thread_cache::thread_cache.with(|tcache| {
                        // This causes another allocation, hopefully with bootstrap
                        let _tcache = tcache; // There is a theoretical bootstrap data race here, but because
                    }); // it repeatedly sets it false, eventually, it will allocate
                    *val.borrow_mut() = true; // Never has to repeat this code after this
                }
                set_use_bootstrap(false) // Turns off the bootstrap
            });
        }

         */

        #[cfg(debug_assertions)]
        if TRACK_ALLOCATION_LOCATION {
            IN_CACHE.fetch_add(size, Ordering::AcqRel);
        }

        // If we are able to reach this piece of code, we know that the thread local cache is initalized
        let ret = thread_cache::thread_cache.with(|tcache| {
            let cache = unsafe {
                (*tcache.get()).get_mut(size_class_index).unwrap() // Gets the correct bin based on size class index
            };

            if cache.get_block_num() == 0 {
                fill_cache(size_class_index, cache); // Fills the cache if necessary
                if cache.block_num == 0 {
                    panic!("Cache didn't fill");
                }
            }
            #[cfg(feature = "track_allocation")]
                {
                    let ret = cache.pop_block();
                    let size = get_allocation_size(ret as *const c_void).unwrap() as usize;
                    crate::info_dump::log_malloc(size);
                    #[cfg(feature = "show_all_allocations")]
                    dump_info!();
                    ret
                }
            #[cfg(not(feature = "track_allocation"))]
                let ptr = cache.pop_block(); // Pops the block from the thread cache bin

            /* WARNING -- ELIAS CODE -- WARNING */

            // #[cfg(unix)]
            {
                if unsafe { USE_APF } {
                    thread_cache::skip.with(|b| unsafe {
                        if !*b.get() {
                            let skip = b.get();
                            *skip = true;
                            thread_cache::apf_init.with(|init| {
                                if !*init.borrow() {
                                    thread_cache::init_tuners();
                                    *init.borrow_mut() = true;
                                }
                                assert_eq!(
                                    thread_cache::apf_init.with(|init| { *init.borrow() }),
                                    true
                                );
                                // set_use_bootstrap(false);
                            });
                            assert_eq!(
                                thread_cache::apf_init.with(|init| { *init.borrow() }),
                                true
                            );
                            let _ = thread_cache::thread_init.with(|_| ());
                        }
                    });
                    thread_cache::skip_tuners.with(|b| unsafe {
                        if *b.get() == 0 {
                            thread_cache::apf_tuners.with(|tuners| {
                                (&mut *tuners.get())
                                    .get_mut(size_class_index)
                                    .unwrap()
                                    .malloc(ptr);
                            });
                        }
                    });
                }
            }

            //set_use_bootstrap(true);

            ptr
        });

        ret
    }
}

/// Creates a new space in memory of length `size`, and moves the memory in `ptr` to the newly created memory. A pointer to the new memory is
/// returned.
///
/// The original `ptr` is free'd within this function, and should not be called manually.
///
/// If the size class of the new `size` is the same as the old size class, then no new memory is allocated and nothing performed. The exception
/// to this is if both the old size class and the new size class are part of the size class 0, which does not have a specific size. If this is
/// the case, then the re-alloc operation is performed only if the new size greater than the old size.
///
/// If a NULL pointer is passed through, it's equivalent to calling `malloc(size)`
///
/// # Safety
/// If an invalid pointer is passed to this function, then a SEGFAULT will occur. As such, this function is marked as unsafe.
pub unsafe fn do_realloc<T>(ptr: *mut T, size: usize) -> *mut T {
    /*
    if ptr.is_null() {
        let ptr = do_malloc(size) as *mut T;
        if ptr.is_null() {
            println!("woops")
        }
        return ptr;
    }

    let new_size_class = get_size_class(size);
    let old_size = match get_allocation_size(ptr as *mut c_void) {
        Ok(size) => size as usize,
        Err(_) => {
            /*
            eprintln!("Given a pointer that isn't in the heap");
            exit(libc::EINVAL);

             */
            // just give up and return a malloc
            let ptr = do_malloc(size) as *mut T;
            if ptr.is_null() {
                println!("woops")
            }
            return ptr;
            //return null_mut();
        }
    };
    let old_size_class = get_size_class(old_size);
    if old_size_class != 0 && old_size_class == new_size_class
        || old_size_class == 0 && new_size_class == 0 && size < old_size
    {
        if ptr.is_null() {
            println!("woops")
        }
        return ptr;
    }

     */




    let ret = do_malloc(size) as *mut T;

    if !ptr.is_null() {

        let old_size = get_allocation_size(ptr as *const c_void).unwrap();
        let copy_size = old_size.min(size as u32) as usize;
        if !ret.is_null() && ret != ptr {
            libc::memcpy(ret as *mut c_void, ptr as *mut c_void, copy_size as usize);
            /*
            for i in 0..(copy_size) {
                (ret as *mut u8).add(i).write((ptr as *mut u8).add(i).read());
            }

             */
        }


        do_free(ptr);
    }

    ret
}

/// Determines the size of the allocation for a pointer. If no allocation data is available for the pointer, `Err(())` is returned. Otherwise,
/// `Ok(block size)` is returned
pub fn get_allocation_size(ptr: *const c_void) -> Result<u32, ()> {
    if let Some(info) = get_page_info_for_ptr(ptr) {
        let desc = unsafe { &*info.get_desc().ok_or(())? };

        Ok(desc.block_size)
    } else {
        Err(())
    }
}

/// Frees a location in memory so that it can be reused at a later time. A free to a `NULL` pointer has no effect.
///
/// # Safety
///
/// If a pointer has already been freed, a second free to that pointer will cause undefined behavior, and likely a SEGFAULT
pub unsafe fn do_free<T: ?Sized>(ptr: *const T) {
    /*
    {
        let bootstrap = bootstrap_reserve.lock();
        if ptr.is_null() || bootstrap.ptr_in_bootstrap(ptr) {
            return;
        }
    }

     */
    if ptr.is_null() {
        return;
    }
    let info = match get_page_info_for_ptr(ptr) {
        None => { return },
        Some(info) => info,
    };
    let desc = &mut *match info.get_desc() {
        Some(d) => d,
        None => {
            // #[cfg(debug_assertions)]
            // println!("Free failed at {:?}", ptr);
            return; // todo: Band-aid fix
            // panic!("Descriptor not found for the pointer {:x?} with page info {:?}", ptr, info);
        }
    };

    // #[cfg(debug_assertions)]
    // println!("Free will succeed at {:?}", ptr);

    let size_class_index = info.get_size_class_index();
    match size_class_index {
        None | Some(0) => {
            let super_block = desc.super_block.as_ref().unwrap();
            // unregister

            unregister_desc(None, super_block);

            // if large allocation
            if ptr as *const u8 != super_block.get_ptr() as *const u8 {
                unregister_desc(None, super_block)
            }

            // free the super block
            if let Some(segment) = std::mem::replace(&mut desc.super_block, None) {
                SEGMENT_ALLOCATOR.deallocate(segment);
            }

            // retire the descriptor
            desc.retire();
        }
        Some(size_class_index) => {
            /*
            let force_bootstrap = bootstrap_reserve.lock().ptr_in_bootstrap(ptr)
                || use_bootstrap()
                || std::thread::panicking()
                || (!cfg!(unix) && thread_cache::thread_init.try_with(|_| {}).is_err());
             */
            let force_bootstrap = false;
            #[cfg(feature = "track_allocation")]
                crate::info_dump::log_free(get_allocation_size(ptr as *const c_void).unwrap() as usize);
            #[cfg(feature = "show_all_allocations")]
            dump_info!();

            if force_bootstrap {
            } else {
                /*
                #[cfg(not(unix))]
                {
                    set_use_bootstrap(true);
                    thread_cache::thread_init.with(|mut val| {
                        if !*val.borrow() {
                            thread_cache::thread_cache.with(|tcache| {
                                let _tcache = tcache;
                            });
                            *val.borrow_mut() = true;
                        }
                        set_use_bootstrap(false)
                    });
                }

                 */

                /* WARNING -- ELIAS CODE -- WARNING */

                // Should always be initialized at this point
                if USE_APF
                    && thread_cache::apf_init
                    .try_with(|init| *init.borrow())
                    .unwrap_or(false)
                {
                    let _r1 = thread_cache::skip_tuners
                        .try_with(|b| {
                            if *b.get() == 0 {
                                let _r2 = thread_cache::apf_tuners.try_with(|tuners| {
                                    (*tuners.get())
                                        .get_mut(size_class_index)
                                        .unwrap()
                                        .free(ptr as *mut u8);
                                });
                            }
                        })
                        .unwrap();
                }

                /* END ELIAS CODE */
                thread_cache::thread_cache
                    .try_with(|tcache| {
                        let cache = (*tcache.get()).get_mut(size_class_index).unwrap();
                        if size_class_index != 0 && cache.get_block_size().is_none() {
                            *cache.get_block_size_mut() = Some(SIZE_CLASSES[size_class_index].block_size);
                        }
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
                        if !USE_APF {
                            let sc = &SIZE_CLASSES[size_class_index];
                            if cache.get_block_num() >= sc.cache_block_num {
                                flush_cache(size_class_index, cache);
                            }
                        }

                        if cache.block_num > 0 {
                            cache.push_block(ptr as *mut u8)
                        } else {
                            cache.push_list(ptr as *mut u8, 1);
                        }
                    })
                    .expect("Freeing to cache failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;
    use std::thread;

    use bitfield::size_of;

    use crate::allocation_data::get_heaps;
    use crate::ptr::auto_ptr::AutoPtr;
    use crate::size_classes::SIZE_CLASSES;

    use super::*;

    #[test]
    fn heaps_valid() {
        let heap = get_heaps();
        let _p_heap = heap.get_heap_at_mut(0);
    }

    #[test]
    fn malloc_and_free() {
        let ptr =
            unsafe { &mut *(super::do_malloc(size_of::<usize>()) as *mut MaybeUninit<usize>) };
        *ptr = MaybeUninit::new(8);
        assert_eq!(
            &unsafe { *(ptr as *const MaybeUninit<usize> as *const usize) },
            &8
        ); // should be trivial
        unsafe {
            do_free(ptr as *mut MaybeUninit<usize>);
        }
    }

    #[test]
    fn malloc_and_free_large() {
        let ptr = super::do_malloc(MAX_SZ * 2);
        unsafe {
            do_free(ptr);
        }
    }

    #[test]
    fn cache_pop_no_fail() {
        const size_class: usize = 16;
        MALLOC_INIT_S.with(|| unsafe { init_malloc() });

        let sc = unsafe { &SIZE_CLASSES[size_class] };
        let total_blocks = sc.block_num;
        let block_size = sc.block_size;

        let test_blocks = total_blocks * 3 / 2;
        let null: *mut u8 = null_mut();
        let mut ptrs = vec![];
        for _ in 0..test_blocks {
            let ptr = do_malloc(block_size as usize);
            unsafe {
                *ptr = b'1';
            }
            assert_ne!(
                ptr, null,
                "Did not successfully get a pointer from the cache"
            );
            ptrs.push(ptr);
        }
        unsafe {
            for ptr in ptrs {
                do_free(ptr);
            }
        }
    }

    #[test]
    fn zero_size_malloc() {
        let v = do_malloc(0);
        assert_ne!(v, null_mut());
        assert_eq!(
            get_allocation_size(v as *const c_void)
                .expect("Zero Sized Allocation should act as an 8 byte allocation"),
            8
        );
        unsafe {
            do_free(v);
        }
    }

    // O(n)
    fn fast_fib(n: usize) -> usize {
        let mut saved = vec![0usize, 1];

        for i in 2..=n {
            saved.push(saved[i - 1] + saved[i - 2]);
        }

        saved[n]
    }

    #[test]
    fn fib_intractable() {
        enum FibTree {
            Val(usize),
            Sum(AutoPtr<FibTree>, AutoPtr<FibTree>),
        }

        impl FibTree {
            fn to_val(&self) -> usize {
                match self {
                    FibTree::Val(v) => *v,
                    FibTree::Sum(l, r) => l.to_val() + r.to_val(),
                }
            }

            fn into_val(self) -> usize {
                dump_info!();
                self.to_val()
            }
        }

        fn fib(n: usize) -> FibTree {
            match n {
                0 => FibTree::Val(0),
                1 => FibTree::Val(1),
                n => FibTree::Sum(AutoPtr::new(fib(n - 1)), AutoPtr::new(fib(n - 2))),
            }
        }

        for n in 0..15 {
            assert_eq!(
                fast_fib(n),
                fib(n).into_val(),
                "fast_fib({}) gave the wrong result",
                n
            );
        }
        dump_info!();
    }

    #[test]
    fn fib_intractable_multi_thread() {
        enum FibTree {
            Val(usize),
            Sum(AutoPtr<FibTree>, AutoPtr<FibTree>),
        }

        impl FibTree {
            fn into_val(self) -> usize {
                dump_info!();
                match self {
                    FibTree::Val(v) => v,
                    FibTree::Sum(l, r) => {
                        let l = l.take();
                        let r = r.take();
                        let l_t = thread::spawn(move || l.into_val()).join().unwrap();
                        let r_t = thread::spawn(move || r.into_val()).join().unwrap();
                        l_t + r_t
                    }
                }
            }
        }

        fn fib(n: usize) -> FibTree {
            match n {
                0 => FibTree::Val(0),
                1 => FibTree::Val(1),
                n => FibTree::Sum(AutoPtr::new(fib(n - 1)), AutoPtr::new(fib(n - 2))),
            }
        }

        for n in 0..15 {
            assert_eq!(
                fast_fib(n),
                fib(n).into_val(),
                "fast_fib({}) gave the wrong result",
                n
            );
        }
        dump_info!();
    }

    #[test]
    fn fib_allocation() {
        fn slow_fib(n: usize) -> AutoPtr<usize> {
            match n {
                0 => AutoPtr::new(0),
                1 => AutoPtr::new(1),
                n => {
                    let ret = AutoPtr::new(*slow_fib(n - 1) + *slow_fib(n - 2));
                    ret
                }
            }
        }

        for n in 0..15 {
            assert_eq!(
                fast_fib(n),
                *slow_fib(n),
                "fast_fib({}) gave the wrong result",
                n
            );
        }

        dump_info!();
    }

    #[test]
    fn big_allocs_on_aligned() {
        let size = 4096 * 16;
        let mut allocs = vec![];

        for i in 0..10_000 {
            let ptr = do_aligned_alloc(4096, size);
            assert!(!ptr.is_null(), "Allocation of large size failed failed after {} attempts", i);
            allocs.push(
                ptr
            );
        }

        for ptr in allocs {
            unsafe {
                do_free(ptr);
            }
        }
    }
}


#[cfg(test)]
mod track_allocation_tests {

    #[cfg(feature = "track_allocation")]
    #[test]
    fn info_dump_one_thread() {
        {
            use crate::ptr::auto_ptr::AutoPtr;

            dump_info!();
            let first_ptrs = (0..10)
                .into_iter()
                .map(|_| AutoPtr::new(0usize))
                .collect::<Vec<_>>();

            dump_info!();

            {
                let first_ptrs = (0..10)
                    .into_iter()
                    .map(|_| AutoPtr::new([0usize; 16]))
                    .collect::<Vec<_>>();
                dump_info!();
            }
            dump_info!();
        }
        dump_info!();
    }


}
