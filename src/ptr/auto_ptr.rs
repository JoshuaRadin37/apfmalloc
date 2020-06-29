use crate::{do_aligned_alloc, do_free};
use std::fmt::Formatter;
use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::drop_in_place;

/// Similar to Box, but directly calls the [`do_aligned_alloc`](fn.do_aligned_alloc.html) and free for dropping
pub struct AutoPtr<T> {
    data: *mut T,
}

impl<T> AutoPtr<T> {
    /// Creates a new AutoPtr
    pub fn new(data: T) -> Self {
        let ptr = do_aligned_alloc(std::mem::align_of::<T>(), std::mem::size_of::<T>()) as *mut T;
        unsafe {
            ptr.write(data);
        }
        Self { data: ptr }
    }

    /// Takes the data in the pointer, and de-allocates the space in the heap.
    ///
    /// # Example
    /// ```
    /// use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
    /// let ptr = AutoPtr::new(100usize);
    /// assert_eq!(*ptr, 100usize);
    /// let value = ptr.take();
    /// assert_eq!(value, 100usize);
    /// ```
    pub fn take(self) -> T {
        unsafe {
            let Self { data } = &self;
            let deref = *data;
            let output = std::ptr::read(deref);
            do_free(*data);
            std::mem::forget(self);
            output
        }
    }

    /// Converts the AutoPtr into a normal pointer, and removes the ability to auto deallocate the pointer
    ///
    /// # Example
    /// ```
    /// use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
    /// use std::ptr::null_mut;
    /// use lrmalloc_rs::do_free;
    /// let mut unsafe_ptr = null_mut();
    /// {
    ///     let ptr = AutoPtr::new(100usize);
    ///     unsafe_ptr = ptr.into_ptr(); // Normally, `ptr` would deallocate after this, but because we have taken the pointer, it longer does
    /// }
    /// unsafe {
    ///     assert_eq!(*unsafe_ptr, 100usize); // `unsafe_ptr` is still valid
    ///     do_free(unsafe_ptr); // must be manually dropped now
    /// }
    ///
    /// ```
    pub fn into_ptr(self) -> *mut T {
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
        Self { data: ptr }
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
        unsafe {
            drop_in_place(self.data);
            do_free(self.data);
        }
    }
}

unsafe impl<T: Send> Send for AutoPtr<T> {}
unsafe impl<T: Sync> Sync for AutoPtr<T> {}

impl<T: Debug> Debug for AutoPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Display> Display for AutoPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Clone> Clone for AutoPtr<T> {
    fn clone(&self) -> Self {
        let data = self.deref();
        let clone = data.clone();
        AutoPtr::new(clone)
    }
}

#[cfg(test)]
mod test {
    use crate::ptr::auto_ptr::AutoPtr;
    use crate::{allocate_type, do_free};


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
    fn take_de_allocates() {
        let _unused1 = AutoPtr::new(0usize); // used to ensure that the cache bin doesn't empty
        let ptr = AutoPtr::new(0usize);
        let location = ptr.data;
        let _ = ptr.take();
        let new_ptr = AutoPtr::new(0usize);
        let new_location = new_ptr.data;
        assert_eq!(new_location, location);
    }

    #[test]
    fn tree_properly_de_allocates() {
        struct Tree(AutoPtr<usize>, AutoPtr<usize>);

        let _unused1 = AutoPtr::new(0usize); // used to ensure that the cache bin doesn't empty

        let (loc1, loc2) = {
            let tree = Tree(AutoPtr::new(0), AutoPtr::new(0));
            (tree.0.data, tree.1.data)
        };

        let tree = Tree(AutoPtr::new(0), AutoPtr::new(0));
        assert!(tree.0.data == loc1 || tree.1.data == loc1);
        assert!(tree.0.data == loc2 || tree.1.data == loc2);
    }

    #[test]
    fn nested_auto_ptr_de_allocates() {
        let _unused1 = AutoPtr::new(0usize); // used to ensure that the cache bin doesn't empty

        let loc = {
            let dptr = AutoPtr::new(AutoPtr::new(0usize));
            (*dptr).data
        };

        let _unused = AutoPtr::new(0usize);
        let new_managed = AutoPtr::new(0usize);
        assert_eq!(new_managed.data, loc, "Should be in the same place, as the nested pointer should have been de-allocated as well");
    }

    #[test]
    fn recursive_struct_test_ptr_de_allocates() {
        let _unused1 = AutoPtr::new(Some(AutoPtr::new(0usize)));

        enum Value {
            Ptr(AutoPtr<Test>),
            Val(usize),
        }
        use Value::*;
        struct Test(Value);

        impl Test {
            fn take(self) -> usize {
                match self.0 {
                    Ptr(ptr) => {
                        let t = ptr.take();
                        t.take()
                    }
                    Val(v) => v,
                }
            }
        }

        let val = {
            let t = Test(Ptr(AutoPtr::new(Test(Val(3799)))));
            t.take()
        };

        assert_eq!(val, 3799);
    }

    #[test]
    fn can_turn_into_un_managed_ptr() {
        let val = 0xdeadbeafusize;
        let _unused1 = AutoPtr::new(val); // used to ensure that the cache bin doesn't empty
        let auto_ptr = AutoPtr::new(val);
        let un_managed_ptr = auto_ptr.into_ptr();
        let potential_fail_pointer = AutoPtr::new(0usize); // If the original pointer is dropped for some reason, this will overwrite the original auto ptr
        assert_eq!(
            unsafe { *un_managed_ptr },
            val,
            "value should not have changed"
        );
        let un_managed_ptr2 = potential_fail_pointer.into_ptr();
        assert_ne!(
            un_managed_ptr2, un_managed_ptr,
            "Should not point to the same location"
        );
        unsafe {
            do_free(un_managed_ptr);
            do_free(un_managed_ptr2);
        }
    }

    #[test]
    fn create_managed_from_ptr() {
        // test by creating a pointer to a heap allocated object, then create a managed pointer for it.
        // Then cause the managed pointer to drop. Then, allocate another object of the same type. If
        // the managed pointer and allocator is working properly, it will be allocated at the same place.
        let _unused1 = AutoPtr::new(0usize); // used to ensure that the cache bin doesn't empty
        let ptr = allocate_type::<usize>();
        unsafe {
            ptr.write(0xdeadbeaf);
            let managed = AutoPtr::from_ptr(ptr);
            assert_eq!(*managed, 0xdeadbeaf);
        }
        // managed pointer should now be dropped
        let new_managed = AutoPtr::new(0usize);
        let new_ptr = new_managed.into_ptr();
        assert_eq!(ptr, new_ptr, "The managed ptr should have caused the old pointer to be freed, enabling the next allocation to be at the same location");
        unsafe {
            do_free(new_ptr); // must deallocate manually now
        }
    }
}
