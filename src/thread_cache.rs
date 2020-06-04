#[derive(Debug, Copy, Clone)]
pub struct ThreadCacheBin {
    pub(crate) block: *mut u8,
    pub(crate) block_num: u32,
}

impl ThreadCacheBin {
    /// Common and Fast
    #[inline]
    pub fn push_block(&mut self, block: *mut u8) {
        unsafe {
            *(block as *mut *mut u8) = self.block;
        };
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

use crate::mem_info::MAX_SZ_IDX;
use std::cell::RefCell;
use std::ptr::null_mut;
thread_local! {
    pub static thread_cache: RefCell<[ThreadCacheBin; MAX_SZ_IDX]> = RefCell::new([ThreadCacheBin {
        block: null_mut(),
        block_num: 0
    }; MAX_SZ_IDX]);

    pub static thread_init: RefCell<bool> = RefCell::new(false);
}
