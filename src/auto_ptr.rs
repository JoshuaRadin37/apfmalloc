use crate::{do_aligned_alloc, do_free};
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::drop_in_place;

pub struct AutoPtr<T : ?Sized> {
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
    {
        unsafe {
            let Self { data } = &self;
            let output = std::ptr::read(*data);
            //do_free(data);
            output
        }
    }

    /// Converts the AutoPtr into a normal pointer, and removes the ability to auto deallocate the pointer
    pub fn into_ptr(self) -> * mut T {
        let AutoPtr { data } = self;
        data
    }

    /// Replaces the data stored in the pointer, and returns the old data
    pub fn replace(&mut self, new: T) -> T {
        unsafe {
            let ptr = self.data;
            ptr.replace(new)
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

impl<T : ?Sized> Drop for AutoPtr<T> {
    fn drop(&mut self) {
        unsafe {
            // drop_in_place::<T>(self.data);
        }
        do_free(self.data);
    }
}

unsafe impl <T: Send> Send for AutoPtr<T> {}
unsafe impl <T: Sync> Sync for AutoPtr<T> {}

#[cfg(test)]
mod test {
    use crate::auto_ptr::AutoPtr;
    use std::fmt::{Display, Debug};

    #[test]
    fn normal_ptr() {
        let mut auto_ptr = AutoPtr::new(Some(0xdeadbeafusize));
        assert_eq!(*auto_ptr, Some(0xdeadbeafusize));
        *auto_ptr = None;
        assert_eq!(*auto_ptr, None);
    }

}
