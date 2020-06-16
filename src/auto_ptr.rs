use crate::{do_aligned_alloc, do_free};
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::drop_in_place;

/// Similar to Box, but directly calls the do_aligned_alloc and free for dropping
pub struct AutoPtr<T : ?Sized> {
    data: *mut T,
}

impl<T> AutoPtr<T> {

    /// Creates a new AutoPtr
    pub fn new(data: T) -> Self {
        let ptr = do_aligned_alloc(std::mem::align_of::<T>(), std::mem::size_of::<T>())
            as *mut T;
        unsafe {
            ptr.write(data);
        }
        Self {
            data: ptr
        }
    }

    /// Takes the data in the pointer, and de-allocates the space in the heap re
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
        let data = self.data;
        std::mem::forget(self);
        data
    }

    /// Replaces the data stored in the pointer, and returns the old data
    pub fn replace(&mut self, new: T) -> T {
        unsafe {
            let ptr = self.data;
            ptr.replace(new)
        }
    }

    /// Replaces the data stored in the pointer, and drops the old value
    pub fn write(&mut self, new: T) {
        let _ = self.replace(new);
    }

    /// Turns an un-managed pointer into a managed one
    ///
    /// # Safety
    /// This function is unsafe because we can't guarantee that the pointer goes to a valid object
    /// of type T, or that the pointer doesn't point to de-allocated space.
    ///
    /// For this function to have safe behavior, it must be assured that the un-managed pointer is never used
    /// after this function is called on it, or at least the un-managed pointer isn't used after the scope of
    /// the AutoPtr is finished. This causes data pointed to by the un-managed pointer to be freed, and potentially
    /// double freed as a result
    pub unsafe fn from_ptr(ptr: *mut T) -> Self {
        Self {
            data: ptr
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
    use crate::{do_free, allocate_type};

    #[test]
    fn normal_ptr() {
        let mut auto_ptr = AutoPtr::new(Some(0xdeadbeafusize));
        assert_eq!(*auto_ptr, Some(0xdeadbeafusize));
        *auto_ptr = None;
        assert_eq!(*auto_ptr, None);
    }

    #[test]
    fn can_take() {
        let val = 0xdeadbeafusize;
        let auto_ptr = AutoPtr::new(val);
        let gotten_val = auto_ptr.take();
        assert_eq!(gotten_val, val);
        // note: compilation fails if try to use the auto_ptr again
    }

    #[test]
    fn can_turn_into_un_managed_ptr() {
        let val = 0xdeadbeafusize;
        let _unused1 = AutoPtr::new(val); // used to ensure that the cache bin doesn't empty
        let auto_ptr = AutoPtr::new(val);
        let un_managed_ptr = auto_ptr.into_ptr();
        let potential_fail_pointer = AutoPtr::new(0usize); // If the original pointer is dropped for some reason, this will overwrite the original auto ptr
        assert_eq!(unsafe {*un_managed_ptr }, val, "value should not have changed");
        let un_managed_ptr2 = potential_fail_pointer.into_ptr();
        assert_ne!(un_managed_ptr2, un_managed_ptr, "Should not point to the same location");
        do_free(un_managed_ptr);
        do_free(un_managed_ptr2);
    }

    #[test]
    fn create_managed_from_ptr() {
        // test by creating a pointer to a heap allocated object, then create a managed pointer for it.
        // Then cause the managed pointer to drop. Then, allocate another object of the same type. If
        // the managed pointer and allocator is working properly, it will be allocated at the same place.
        let _unused1 = AutoPtr::new(0usize); // used to ensure that the cache bin doesn't empty
        let ptr = allocate_type::<usize>();
        unsafe
        {
            ptr.write(0xdeadbeaf);
            let managed = AutoPtr::from_ptr(ptr);
            assert_eq!(*managed, 0xdeadbeaf);
        }
        // managed pointer should now be dropped
        let new_managed = AutoPtr::new(0usize);
        let new_ptr = new_managed.into_ptr();
        assert_eq!(ptr, new_ptr, "The managed ptr should have caused the old pointer to be freed, enabling the next allocation to be at the same location");
        do_free(new_ptr); // must deallocate manually now

    }

}
