use core::sync::atomic::AtomicBool;
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering;

/// A single access struct is an atomic, spin-locked barrier that ensures that only a single thread is ever able to
/// to execute some function with it. Single Access structs can not preset its function, as this type is only usefull
/// when used in a static context.
pub struct SingleAccess {
    internal: UnsafeCell<SingleAccessInternal>,
}

struct SingleAccessInternal {
    access: AtomicBool,
    wait: bool,
    skip: bool,
}

impl SingleAccessInternal {
    fn with<F>(&mut self, func: F)
    where
        F: FnOnce(),
    {
        if !self.skip {
            if !self.access.compare_and_swap(false, true, Ordering::Acquire) {
                func();
                self.wait = false;
                self.skip = true;
            }

            while self.wait {}
        }
    }

    fn with_then<F1, F2>(&mut self, func: F1, after: F2)
    where
        F1: FnOnce(),
        F2: FnOnce(),
    {
        if !self.skip {
            if !self.access.compare_and_swap(false, true, Ordering::Release) {
                func();
                self.wait = false;
                self.skip = true;
                after();
            }

            while self.wait {}
        }
    }
}

impl SingleAccess {
    /// Creates a new `SingleAccess` struct. Any struct can only be used with [`with()`](#method.with) or [`with_then()`](#method.with_then)
    /// a single time.
    pub const fn new() -> Self {
        Self {
            internal: UnsafeCell::new(SingleAccessInternal {
                access: AtomicBool::new(false),
                wait: true,
                skip: false,
            }),
        }
    }

    /// When multiple threads have access to the same same `SingleAccess` struct, only one thread will ever execute the
    /// the `func`tion. This ensured initially atomically, then un-atomically after the function has completed.
    ///
    /// While multiple threads are within this function, they are spin locked until the executing thread completes the function
    /// # Panic
    /// If the execuitng thread panics while other threads are within this function, they will never be released.
    pub fn with<F>(&self, func: F)
    where
        F: FnOnce(),
    {
        unsafe { (*self.internal.get()).with(func) }
    }

    /// When multiple threads have access to the same same `SingleAccess` struct, only one thread will ever execute the
    /// the `func`tion. This ensured initially atomically, then un-atomically after the function has completed.
    ///
    /// While multiple threads are within this function, they are spin locked until the executing thread completes the function
    ///
    /// Once the executing thread finishes `func`, all other locked threads are released, then the same executing thread will then
    /// execute `after`.
    /// # Panic
    /// If the execuitng thread panics while other threads are within this function, they will never be released.
    pub fn with_then<F1, F2>(&self, func: F1, after: F2)
    where
        F1: FnOnce(),
        F2: FnOnce(),
    {
        unsafe { (*self.internal.get()).with_then(func, after) }
    }
}

unsafe impl Send for SingleAccess {}

unsafe impl Sync for SingleAccess {}

#[cfg(test)]
mod test {
    use crate::single_access::SingleAccess;
    use spin::Mutex;
    use std::sync::{Arc, Barrier};
    use std::thread;

    static mut counter: i16 = 0;
    static LOCK_TEST: Mutex<()> = Mutex::new(());

    fn increase_counter() {
        unsafe {
            counter += 1;
        }
    }

    fn get_counter() -> i16 {
        unsafe { counter }
    }

    #[test]
    fn only_once() {
        let _m = LOCK_TEST.lock();
        let access = SingleAccess::new();
        let start = get_counter();
        access.with(increase_counter);
        assert_eq!(get_counter(), start + 1);
        access.with(increase_counter);
        assert_eq!(get_counter(), start + 1);
    }

    #[test]
    fn multiple_at_once() {
        let _m = LOCK_TEST.lock();
        let access = Arc::new(SingleAccess::new());
        let barrier = Arc::new(Barrier::new(4));

        let start = get_counter();
        let mut handles = vec![];
        for _ in 0..4 {
            let ac = access.clone();
            let br = barrier.clone();
            handles.push(thread::spawn(move || {
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
