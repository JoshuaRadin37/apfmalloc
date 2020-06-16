use core::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::cell::UnsafeCell;

pub struct SingleAccess {
    internal: UnsafeCell<SingleAccessInternal>
}

struct SingleAccessInternal {
    access: AtomicBool,
    wait: bool,
    skip: bool,
}

impl SingleAccessInternal {
    fn with<F>(&mut self, func: F) where F : FnOnce() {
        if !self.skip {
            if !self.access.compare_and_swap(false, true, Ordering::Acquire) {
                func();
                self.wait = false;
                self.skip = true;
            }

            while self.wait { }
        }
    }

    fn with_then<F1, F2>(&mut self, func: F1, after: F2) where F1 : FnOnce(), F2 : FnOnce() {
        if !self.skip {
            if !self.access.compare_and_swap(false, true, Ordering::Acquire) {
                func();
                self.wait = false;
                self.skip = true;
                after();
            }

            while self.wait { }
        }
    }
}

impl SingleAccess {

    pub const fn new() -> Self {
        Self {
            internal: UnsafeCell::new(SingleAccessInternal {
                access: AtomicBool::new(false),
                wait: true,
                skip: false,
            }),
        }
    }

    pub fn with<F>(&self, func: F) where F : FnOnce() {
        unsafe {
            (*self.internal.get()).with(func)
        }
    }

    pub fn with_then<F1, F2>(&self, func: F1, after: F2) where F1 : FnOnce(), F2 : FnOnce() {
        unsafe {
            (*self.internal.get()).with_then(func, after)
        }
    }
}

unsafe impl Send for SingleAccess {

}

unsafe impl Sync for SingleAccess {

}

#[cfg(test)]
mod test {
    use crate::single_access::SingleAccess;
    use std::sync::{Arc, mpsc, Barrier};
    use std::thread;
    use spin::Mutex;

    static mut counter: i16 = 0;
    static LOCK_TEST: Mutex<()> = Mutex::new(());

    fn increase_counter() {
        unsafe {
            counter += 1;
        }
    }

    fn get_counter() -> i16 {
        unsafe {
            counter
        }
    }


    #[test]
    fn only_once() {
        let _ = LOCK_TEST.lock();
        let access = SingleAccess::new();
        let start = get_counter();
        access.with(increase_counter);
        assert_eq!(get_counter(), start + 1);
        access.with(increase_counter);
        assert_eq!(get_counter(), start + 1);


    }

    #[test]
    fn multiple_at_once() {
        let _ = LOCK_TEST.lock();
        let access = Arc::new(SingleAccess::new());
        let barrier = Arc::new(Barrier::new(4));

        let start = get_counter();
        let mut handles = vec![];
        for _ in 0..4 {
            let ac = access.clone();
            let br = barrier.clone();
            handles.push(thread::spawn( move || {
                br.wait();
                ac.with(increase_counter)
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(get_counter(), start + 1);

    }
}

