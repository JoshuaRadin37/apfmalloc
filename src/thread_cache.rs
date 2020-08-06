use crate::allocation_data::Anchor;

use crate::alloc::{
    compute_index, get_page_info_for_ptr, heap_push_partial, malloc_count_from_new_sb,
    malloc_count_from_partial, malloc_from_new_sb, malloc_from_partial, unregister_desc,
};
use crate::allocation_data::{get_heaps, SuperBlockState};
use crate::mem_info::{CACHE_LINE, MAX_SZ_IDX};
use crate::size_classes::{get_size_class, SIZE_CLASSES};
use core::ops::{Deref, DerefMut};
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

static RECORDED_SC: usize = 41; // Size class to record and display graph of -- 41 if none

/// This structure contains the stack of blocks of a certain size class that a thread has access to.
/// It has two public fields:
/// - `block`
/// - `block_num`
///
/// These are used to keep easy track of the condition of the thread cache.
///
/// This struct implements Copy so that it can be created without using the heap when creating thread statics
#[derive(Debug, Copy, Clone)]
pub struct ThreadCacheBin {
    pub(crate) block: *mut u8,
    pub(crate) block_num: u32,
    block_size: Option<u32>,
}

impl ThreadCacheBin {
    /// Creates a new bin of undetermined class
    pub const fn new() -> Self {
        Self {
            block: null_mut(),
            block_num: 0,
            block_size: None,
        }
    }

    /// Common and Fast. Pushes a block to the top of the stack so it can be used again later. This function should be unsafe,
    /// but because it is only ever called from an unsafe context, it's unnecessary.
    #[inline]
    pub fn push_block(&mut self, block: *mut u8) {
        match self.block_size {
            // If the block size is recorded and it's less than the CACHE_LINE, it may be slightly faster to attempt to push it back as
            // a contiguous block
            Some(block_size) if block_size < CACHE_LINE as u32 && !cfg!(feature = "no_met_stack") => {
                let old_loc = self.block as usize as isize;
                let diff = old_loc - block as usize as isize;
                if diff == block_size as isize {
                    unsafe {
                        *(block as *mut *mut u8) = null_mut();
                    }
                } else {
                    unsafe {
                        *(block as *mut *mut u8) = self.block;
                    }
                }
                self.block = block;
                self.block_num += 1;
            }
            None | Some(_) => {
                unsafe {
                    *(block as *mut *mut u8) = self.block;
                }
                self.block = block;
                self.block_num += 1;
            }
        }
        /*
        unsafe {
            *(block as *mut *mut u8) = self.block;
        }
        self.block = block;
        self.block_num += 1;

         */
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
            match self.block_size {
                None => {
                    if (self.block as *mut u8).is_null() {
                        return null_mut();
                    }
                    self.block = unsafe { *(self.block as *mut *mut u8) };
                    //self.block = unsafe { self.block.offset(-1) };

                }
                Some(block_size) => if cfg!(feature = "no_met_stack") {
                    if (self.block as *mut u8).is_null() {
                        return null_mut();
                    }
                    self.block = unsafe { *(self.block as *mut *mut u8) };
                } else {
                    unsafe {
                        let block_read = *(self.block as *mut *mut usize);
                        if block_read.is_null() {
                            self.block = self.block.add(block_size as usize);
                        } else if block_read as usize == std::usize::MAX {
                            self.block = null_mut();
                        } else {
                            self.block = *(self.block as *mut *mut u8);
                        }
                    }
                },
            };
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

    /// Gives the pointer to the first block in the stack
    #[inline]
    pub fn peek_block(&self) -> *mut u8 {
        self.block
    }

    /// Gets the number of blocks in the stack
    #[inline]
    pub fn get_block_num(&self) -> u32 {
        self.block_num
    }
}

/// Fills a cache with blocks of the `size_class_index`.
///
/// This either fills the cache using a partial list in the central reserve, or by creating a new super block.
pub fn fill_cache(size_class_index: usize, cache: &mut ThreadCacheBin) {
    let mut block_num = 0;
    let mut used_partial = true;

    // Uses a partial list from the central reserve
    malloc_from_partial(size_class_index, cache, &mut block_num);
    if block_num == 0 {
        // Creates a new super block. Depending on the load on the kernel, the tail latency on this operation is high.
        malloc_from_new_sb(size_class_index, cache, &mut block_num);
        used_partial = false;
    }
    if block_num == 0 || cache.block_num == 0 {
        panic!(
            "Didn't allocate any blocks to the cache. USED PARTIAL={}, block_num={}, cache.block_num={}",
            used_partial,
            block_num,
            cache.block_num
        );
    }

    let sc = unsafe { &SIZE_CLASSES[size_class_index] };
    cache.block_size = Some(sc.block_size);

    #[cfg(debug_assertions)]
        {
            debug_assert!(block_num > 0);
            debug_assert!(block_num <= sc.cache_block_num as usize);
        }
}

/// Flushes the contents of a thread cache bin back to the central reserve.
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
        let desc = unsafe {
            match info.get_desc() {
                None => {
                    return;
                }
                Some(desc) => &mut *desc,
            }
        };
        //info!("Descriptor: {:?}", desc);
        //info!("Cache anchor info: {:?}", desc.anchor.load(Ordering::Acquire));

        let super_block = desc.super_block.as_ref().unwrap().get_ptr() as *mut u8;

        let mut block_count = 1;
        while cache.get_block_num() > block_count {
            if cfg!(feature = "no_met_stack") || cache.block_size.is_none() || cache.block_size == Some(0)
            {
                let ptr = unsafe { *(tail as *mut *mut u8) };
                if ptr < super_block || ptr as usize >= super_block as usize + sb_size as usize {
                    break;
                }

                block_count += 1;
                tail = ptr;
            } else {
                // let ptr = tail as *mut usize;
                unsafe {
                    if (*(tail as *mut *mut u8)).is_null() {
                        tail = tail.add(cache.block_size.unwrap() as usize);
                        block_count += 1;
                    } else if *(tail as *const usize) == std::usize::MAX {
                        block_count += 1;
                        break;
                    } else {
                        let ptr = *(tail as *mut *mut u8);
                        if ptr < super_block || ptr as usize >= super_block as usize + sb_size as usize {
                            break;
                        }

                        block_count += 1;
                        tail = ptr;
                    }
                }
            }
        }
        //info!("Reclaiming {} blocks", block_count);
        cache.pop_list(unsafe { *(tail as *mut *mut u8) }, block_count);

        let index = compute_index(super_block, head, size_class_index);


        let mut new_anchor: Anchor;
        let old_anchor = loop {
            let old_anchor = desc.anchor.load(Ordering::Acquire);
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
                break old_anchor;
            }
        };

        if new_anchor.state() == SuperBlockState::EMPTY {
            unregister_desc(Some(heap), desc.super_block.as_ref().unwrap());
            if let Some(segment) = std::mem::replace(&mut desc.super_block, None) {
                unsafe {
                    SEGMENT_ALLOCATOR.deallocate(segment);
                }
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

    cache.block_size = None;
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
            if let Some(sz) = bin.block_size {
                let sz_idx = get_size_class(sz as usize);
                flush_cache(sz_idx, bin);
            }
        }
    }
}

/// This is a zero sized structure. When a thread ends, all thread-static variables that implement drop are dropped. [ThreadCacheBins](struct.ThreadCacheBin.html)
/// implement Copy, and therefore can not be dropped. By having a thread-static variable of this type initialized, we are able to
/// simulate having a "de-constructor" for threads.
pub struct ThreadEmpty;

#[cfg(unix)]
impl Drop for ThreadEmpty {
    /// Flushes all of the [ThreadCacheBins](struct.ThreadCacheBin.html)
    fn drop(&mut self) {
        //info!("Flushing entire thread cache");

        thread_cache.with(|tcache| {
            let tcache = unsafe { &mut *tcache.get() };
            for bin_index in 0..tcache.len() {
                let cache = tcache.get_mut(bin_index).unwrap();
                if cache.block_num > 0 {
                    if let Some(sz) = cache.block_size {
                        let sz_idx = get_size_class(sz as usize);
                        flush_cache(sz_idx, cache);
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
                    (&mut *tuners.get()).push(ApfTuner::new(
                        i,
                        check,
                        fetch,
                        ret,
                        i == RECORDED_SC,
                    ));
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

    let mut block_num = 0;

    malloc_count_from_partial(size_class_index, cache, &mut block_num, count);

    // Handles no partial block and insufficient partial block cases
    // Shouldn't need to loop more than once unless fetching *really* large count
    if block_num == 0 {
        malloc_count_from_new_sb(size_class_index, cache, &mut block_num, count);
    }

    return block_num > 0;
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
use crate::pages::external_mem_reservation::{SegAllocator, SEGMENT_ALLOCATOR};

thread_local! {
    // pub static thread_cache: UnsafeCell<ThreadCache> = UnsafeCell::new(ThreadCache::new());
    /// The actual thread cache
    pub static thread_cache: UnsafeCell<[ThreadCacheBin; MAX_SZ_IDX]> = UnsafeCell::new([ThreadCacheBin::new(); MAX_SZ_IDX]);
    /// Enables dropping of the thread cache bins (see [ThreadEmpty](struct.ThreadEmpty.html))
    pub static thread_init: ThreadEmpty = ThreadEmpty;

    // #[cfg(unix)]
    pub static skip: UnsafeCell<bool> = UnsafeCell::new(false);

    //#[cfg(unix)]
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
