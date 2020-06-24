use crate::ptr::auto_ptr::AutoPtr;
use core::ptr::drop_in_place;
use core::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/*
pub struct Rc<T> {
    ptr_to_inner: NonNull<RcInner<T>>
}

impl<T> Rc<T> {

    pub fn new(val: T) -> Self {
        Self {
            ptr_to_inner: NonNull::new(AutoPtr::new(RcInner::new(val)).into_ptr()).unwrap()
        }
    }

}

struct RcInner<T> {
    ptr: AutoPtr<T>,
    strong_count: usize,
    weak_count: usize
}

impl<T> RcInner<T> {
    pub fn new(data: T) -> Self {
        Self {
            ptr: AutoPtr::new(data),
            strong_count: 1,
            weak_count: 0
        }

    }

    fn increment(&mut self) {
        self.strong_count += 1;
    }

    fn decrement(&mut self) {
        self.strong_count -= 1;
        if self.strong_count == 0 {
            unsafe {
                drop_in_place(self.ptr.into_ptr())
            }
        }
    }
}


impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.ptr_to_inner.as_mut().increment()
        }
        Self {
            ptr_to_inner: self.ptr_to_inner
        }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe {
            (*self.ptr_to_inner).decrement()
        }
    }
}


 */
