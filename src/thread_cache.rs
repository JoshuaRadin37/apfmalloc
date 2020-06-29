use crate::allocation_data::Anchor;

use crate::alloc::{
    compute_index, get_page_info_for_ptr, heap_push_partial, malloc_count_from_new_sb,
    malloc_count_from_partial, malloc_from_new_sb, malloc_from_partial, unregister_desc,
};
use crate::allocation_data::{get_heaps, SuperBlockState};
use crate::mem_info::MAX_SZ_IDX;
use crate::size_classes::SIZE_CLASSES;
use core::ops::{Deref, DerefMut};
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

#[derive(Debug, Copy, Clone)]
pub struct ThreadCacheBin {
    pub(crate) block: *mut u8,
    pub(crate) block_num: u32,
    block_size: Option<usize>,
}

impl ThreadCacheBin {
    pub const fn new() -> Self {
        Self {
            block: null_mut(),
            block_num: 0,
            block_size: None,
        }
    }

    /// Common and Fast
    #[inline]
    pub fn push_block(&mut self, block: *mut u8) {
        unsafe {
            *(block as *mut *mut u8) = self.block;
        }
        self.block = block;
        self.block_num += 1;
    }

    /// Pushes a block list
    ///
    /// # Panic
    /// Panics if cache isn't empty
    #[inline]
    pub fn push_list(&mut self, block: *mut u8, length: u32) {
        if self.block_num > 0 {
            panic!("Attempting to push a block list while cache is not empty");
        } else {
            //info!("Pushing {} blocks to cache", length);
            self.block = block;
            self.block_num = length;
        }
    }

    /// Pops a block from the cache
    ///
    /// # Panic
    /// Panics if the cache is empty
    #[inline]
    pub fn pop_block(&mut self) -> *mut u8 {
        if self.block_num == 0 {
            panic!("Attempting to pop a block from cache while cache is empty")
        } else {
            let ret = self.block;
            self.block = unsafe { *(self.block as *mut *mut u8) };
            //self.block = unsafe { self.block.offset(-1) };
            self.block_num -= 1;

            ret
        }
    }

    /// Manually popped the list and now needs to update cache
    ///
    /// the `block` parameter is the new block
    ///
    /// the `length` is the length of the popped list
    ///
    /// # Panic
    /// Panics if the `self.block_num < length`
    #[inline]
    pub fn pop_list(&mut self, block: *mut u8, length: u32) {
        if self.block_num < length {
            panic!("The block_num must be greater than or equal to the provided length");
        } else {
            self.block = block;
            self.block_num -= length;
        }
    }

    #[inline]
    pub fn peek_block(&self) -> *mut u8 {
        self.block
    }

    #[inline]
    pub fn get_block_num(&self) -> u32 {
        self.block_num
    }
}

pub fn fill_cache(size_class_index: usize, cache: &mut ThreadCacheBin) {
    let mut block_num = 0;
    let mut used_partial = true;

    malloc_from_partial(size_class_index, cache, &mut block_num);
    if block_num == 0 {
        malloc_from_new_sb(size_class_index, cache, &mut block_num);
        used_partial = false;
    }
    if block_num == 0 || cache.block_num == 0 {
        /*
        error!("Didn't allocate any blocks to the cache. USED PARTIAL={}, block_num={}, cache.block_num={}",
               used_partial,
               block_num,
               cache.block_num);

         */
        panic!(
            "Didn't allocate any blocks to the cache. USED PARTIAL={}, block_num={}, cache.block_num={}",
            used_partial,
            block_num,
            cache.block_num
        );
    }

    cache.block_size = Some(size_class_index);

    #[cfg(debug_assertions)]
        {
            let sc = unsafe { &SIZE_CLASSES[size_class_index] };
            debug_assert!(block_num > 0);
            debug_assert!(block_num <= sc.cache_block_num as usize);
        }
}

pub fn flush_cache(size_class_index: usize, cache: &mut ThreadCacheBin) {
    // println!("Flushing Cache");
    //info!("Flushing size class {} cache...", size_class_index);
    let heap = get_heaps().get_heap_at_mut(size_class_index);
    let sc = unsafe { &SIZE_CLASSES[size_class_index] };

    let sb_size = sc.sb_size;
    let block_size = sc.block_size;

    let _max_count = sc.get_block_num();

    // There's a to do here in the original program to optimize, which is amusing
    while cache.get_block_num() > 0 {
        let head = cache.peek_block();
        let mut tail = head;
        let info = get_page_info_for_ptr(head);
        let desc = unsafe { &mut *info.get_desc().expect("Could not find descriptor") };
        //info!("Descriptor: {:?}", desc);
        //info!("Cache anchor info: {:?}", desc.anchor.load(Ordering::Acquire));

        let super_block = desc.super_block.as_ref().unwrap().get_ptr() as *mut u8;

        let mut block_count = 1;
        while cache.get_block_num() > block_count {
            let ptr = unsafe { *(tail as *mut *mut u8) };
            if ptr < super_block || ptr as usize >= super_block as usize + sb_size as usize {
                break;
            }

            block_count += 1;
            tail = ptr;
        }
        //info!("Reclaiming {} blocks", block_count);
        cache.pop_list(unsafe { *(tail as *mut *mut u8) }, block_count);

        let index = compute_index(super_block, head, size_class_index);

        let old_anchor = desc.anchor.load(Ordering::Acquire);
        let mut new_anchor: Anchor;
        loop {
            unsafe {
                // update avail
                let next = super_block.offset((old_anchor.avail() * block_size as u64) as isize);

                *(tail as *mut *mut u8) = next;
            }

            new_anchor = old_anchor;
            new_anchor.set_avail(index as u64);

            // state updates
            // dont set to partial if state is active
            if old_anchor.state() == SuperBlockState::FULL {
                new_anchor.set_state(SuperBlockState::PARTIAL);
            }

            if old_anchor.count() + block_count as u64 == desc.max_count as u64 {
                //info!("Setting new anchor to EMPTY");
                new_anchor.set_count(desc.max_count as u64 - 1);
                new_anchor.set_state(SuperBlockState::EMPTY);
            } else {
                new_anchor.set_count(block_count as u64);
            }

            if desc
                .anchor
                .compare_exchange_weak(old_anchor, new_anchor, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }

        if new_anchor.state() == SuperBlockState::EMPTY {
            unregister_desc(Some(heap), desc.super_block.as_ref().unwrap());
            if let Some(segment) = std::mem::replace(&mut desc.super_block, None) {
                SEGMENT_ALLOCATOR.deallocate(segment);
            }
        } else if old_anchor.state() == SuperBlockState::FULL {
            /*info!("Pushing a partially used list to the heap (Size Class Index = {}, available = {}, count = {})",
                  size_class_index,
                  desc.anchor.load(Ordering::Acquire).avail(),
                  desc.anchor.load(Ordering::Acquire).count()
            );

             */
            heap_push_partial(desc)
        }
    }
}

pub struct ThreadCache([ThreadCacheBin; MAX_SZ_IDX]);

impl ThreadCache {
    pub const fn new() -> Self {
        ThreadCache([ThreadCacheBin::new(); MAX_SZ_IDX])
    }
}

impl Deref for ThreadCache {
    type Target = [ThreadCacheBin; MAX_SZ_IDX];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ThreadCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for ThreadCache {
    fn drop(&mut self) {

        //let thread_cache_bins  = &mut self.get_mut();
        //info!("Flushing a thread cache");
        for bin_index in 0..self.len() {
            let bin = self.get_mut(bin_index).unwrap();
            if let Some(sz_idx) = bin.block_size {
                flush_cache(sz_idx, bin);
            }
        }

    }
}

pub struct ThreadEmpty;

#[cfg(unix)]
impl Drop for ThreadEmpty {
    fn drop(&mut self) {
        //info!("Flushing entire thread cache");
        thread_cache.with(|tcache| {
            let tcache = unsafe { &mut *tcache.get() };
            for bin_index in 0..tcache.len() {
                let cache = tcache.get_mut(bin_index).unwrap();
                if cache.block_num > 0 {
                    if let Some(size_class_index) = cache.block_size {
                        flush_cache(size_class_index, cache);
                    }
                }
            }
        });
    }
}
// APF Functions

pub fn init_tuners() {
    no_tuning(|| {
        apf_tuners.with(|tuners| {
            for i in 0..MAX_SZ_IDX {
                unsafe {
                    (&mut *tuners.get()).push(ApfTuner::new(i, check, fetch, ret));
                }
            }
        });
        apf_init.with(|b| {
            *b.borrow_mut() = true;
        });
        skip_tuners.with(|s| unsafe {
            *s.get() = 0;
        })
    });
}

fn check(size_class_index: usize) -> u32 {
    return thread_cache.with(|tcache| {
        unsafe {
            return (*tcache.get())
                .get(size_class_index)
                .unwrap()
                .get_block_num();
        };
    });
}

fn fetch(size_class_index: usize, count: usize) -> bool {
    let cache = &mut thread_cache
        .with(|tcache| unsafe { (*tcache.get()).get_mut(size_class_index).unwrap() });

    let mut block_num = 100.max(count);


    malloc_count_from_partial(size_class_index, cache, &mut block_num, count);

    // Handles no partial block and insufficient partial block cases
    // Shouldn't need to loop more than once unless fetching *really* large count
    while block_num < count {
        malloc_count_from_new_sb(size_class_index, cache, &mut block_num, count);
    }

    return false;
}

fn ret(size_class_index: usize, count: u32) -> bool {
    let cache =
        thread_cache.with(|tcache| unsafe { (*tcache.get()).get_mut(size_class_index).unwrap() });

    assert!(
        count <= cache.get_block_num(),
        "Trying to pop return more blocks than in cache"
    );

    for _i in 0..count {
        cache.pop_block();
    }

    return true;
}

use crate::apf::ApfTuner;
use crate::pages::external_mem_reservation::{SEGMENT_ALLOCATOR, SegAllocator};

#[cfg(not(unix))]
impl Clone for ThreadBool {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

#[cfg(not(unix))]
impl Copy for ThreadBool {}

thread_local! {
    // pub static thread_cache: UnsafeCell<ThreadCache> = UnsafeCell::new(ThreadCache::new());
    pub static thread_cache: UnsafeCell<[ThreadCacheBin; MAX_SZ_IDX]> = UnsafeCell::new([ThreadCacheBin::new(); MAX_SZ_IDX]);

    pub static thread_init: ThreadEmpty = ThreadEmpty;

    #[cfg(unix)]
    pub static skip: UnsafeCell<bool> = UnsafeCell::new(false);

    #[cfg(unix)]
    pub static skip_tuners: UnsafeCell<usize> = UnsafeCell::new(1);

    // Probably don't want a static lifetime here
    pub static apf_tuners: UnsafeCell<Vec<ApfTuner<'static>>> = UnsafeCell::new(Vec::<ApfTuner>::new());
    pub static apf_init: RefCell<bool> = RefCell::new(false);

    pub static thread_use_bootstrap: UnsafeCell<bool> = UnsafeCell::new(false);
}

#[inline]
pub fn no_tuning<R, F: FnOnce() -> R>(func: F) -> R {
    crate::thread_cache::skip_tuners.with(|b| unsafe {
        *b.get() += 1;
    });
    let ret = func();
    crate::thread_cache::skip_tuners.with(|b| unsafe {
        if *b.get() > 0 {
            *b.get() -= 1;
        }
    });
    ret
}

#[cfg(test)]
mod test {
    use crate::thread_cache::ThreadCacheBin;


    #[test]
    fn check_bin_consistency() {
        let _bin = ThreadCacheBin::new();
    }
}
