use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// As it is not clone or copy
pub struct NoHeapMutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct MutexLocked;

#[allow(unused)]
impl<T> NoHeapMutex<T> {
    pub const fn new(t: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }

    pub fn lock(&self) -> NoHeapMutexGuard<'_, T> {
        while self.lock.compare_and_swap(false, true, Ordering::AcqRel) {}
        NoHeapMutexGuard { mutex: self }
    }

    pub fn try_lock(&self) -> Result<NoHeapMutexGuard<'_, T>, MutexLocked> {
        match self.lock.compare_and_swap(false, true, Ordering::AcqRel) {
            false => Ok(NoHeapMutexGuard { mutex: self }),
            true => Err(MutexLocked),
        }
    }
}

unsafe impl<T: Send> Send for NoHeapMutex<T> {}
unsafe impl<T: Sync> Sync for NoHeapMutex<T> {}

pub struct NoHeapMutexGuard<'a, T> {
    mutex: &'a NoHeapMutex<T>,
}

impl<T> Drop for NoHeapMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.lock.store(false, Ordering::Release);
    }
}

impl<T> Deref for NoHeapMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for NoHeapMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

#[cfg(test)]
mod test {
    use crate::no_heap_mutex::NoHeapMutex;
    use std::thread;

    #[test]
    fn use_mutex() {
        static mutex: NoHeapMutex<usize> = NoHeapMutex::new(0usize);
        for _ in 0..100 {
            {
                *mutex.lock() = 0;
            }

            let mut children = vec![];
            let target = 5usize;
            for _ in 0..target {
                children.push(thread::spawn(|| {
                    let mut lock = mutex.lock();
                    *lock += 1;
                }))
            }

            for child in children {
                child.join().unwrap();
            }

            {
                let m = mutex.lock();
                assert_eq!(*m, target);
            }
        }
    }

    #[test]
    fn mutex_locked() {
        static mutex: NoHeapMutex<usize> = NoHeapMutex::new(0usize);
        {
            let _lock = mutex.lock();

            match mutex.try_lock() {
                Ok(_) => panic!("Should not be able to acquire the lock, as it already out"),
                Err(_) => {}
            }
        }

        match mutex.try_lock() {
            Ok(_) => {}
            Err(_) => panic!("Lock should have been freed after previous block"),
        }
    }
}
