use std::cell::RefCell;
use crate::mem_info::MAX_SZ_IDX;
use crate::thread_cache::ThreadCacheBin;
use std::ptr::null_mut;
use spin::Mutex;

pub static mut bootstrap_cache: [ThreadCacheBin; MAX_SZ_IDX] = [ThreadCacheBin {
    block: null_mut(),
    block_num: 0
}; MAX_SZ_IDX];

static _use_bootstrap: Mutex<bool> = Mutex::new(false);

pub fn use_bootstrap() -> bool {
    *_use_bootstrap.lock()
}

pub fn set_use_bootstrap(val: bool) {
    *_use_bootstrap.lock() = val;
}