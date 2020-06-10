use crate::{do_aligned_alloc, do_free};
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

pub struct AutoPtr<T> {
    data: *mut T,
}

impl<T> AutoPtr<T> {
    pub fn new(data: T) -> Self {
        let ptr = do_aligned_alloc(std::mem::align_of::<T>(), std::mem::size_of::<T>())
            as *mut MaybeUninit<T>;
        unsafe {
            *ptr = MaybeUninit::new(data);
        }
        let ret = Self {
            data: ptr as *mut T,
        };
        ret
    }

    pub fn take(self) -> T
    where
        T: Copy,
    {
        unsafe {
            let Self { data } = self;
            let output = *data;
            do_free(data);
            output
        }
    }
}

impl<T> Deref for AutoPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T> DerefMut for AutoPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<T> Drop for AutoPtr<T> {
    fn drop(&mut self) {
        do_free(self.data);
    }
}

unsafe impl<T: Send> Send for AutoPtr<T> {}
