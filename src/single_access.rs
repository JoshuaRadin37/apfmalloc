use core::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct SingleAccess {
    access: AtomicBool,
    wait: bool,
    skip: bool,
}

impl SingleAccess {

    pub const fn new() -> Self {
        Self {
            access: AtomicBool::new(false),
            wait: true,
            skip: false,
        }
    }

    pub fn with<F>(&mut self, func: F) where F : FnOnce() {
        if !self.skip {
            if !self.access.compare_and_swap(false, true, Ordering::Acquire) {
                func();
                self.wait = false;
                self.skip = true;
            }

            while self.wait { }
        }
    }
}

