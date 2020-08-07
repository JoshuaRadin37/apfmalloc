use crate::independent_collections::{Array};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::ops::{Index, IndexMut, Deref};
use std::iter::FromIterator;
use std::fmt::{Debug, Formatter};
use crate::ptr::auto_ptr::{AutoPtr, AlignedAllocator};
use crate::mem_info::{align_size};
use crate::pages::external_mem_reservation::{SEGMENT_ALLOCATOR, SegAllocator, Segment};
use std::ffi::c_void;

pub struct IndependentAllocator;

impl AlignedAllocator for IndependentAllocator {
    fn aligned_alloc(align: usize, size: usize) -> *mut u8 {
        let actual_size = align_size(size, align);
        let new_ptr = SEGMENT_ALLOCATOR.allocate(actual_size).unwrap();
        new_ptr.get_ptr() as *mut u8
    }

    unsafe fn free<T>(ptr: *mut T) {
        let size = align_size(std::mem::size_of::<T>(), std::mem::align_of::<T>());
        let segment =
            Segment::new(
                ptr as *mut c_void,
                size
            );
        SEGMENT_ALLOCATOR.deallocate(segment);
    }
}

pub struct SyncArray<T> {
    array: Array<AutoPtr<T, IndependentAllocator>>,
    growing: AtomicBool,
    writers: AtomicU32
}

impl<T: Default> SyncArray<T> {
    pub fn of_size(size: usize) -> Self {
        Self::of_size_using(|| Default::default(), size)
    }

    pub unsafe fn from_ptr(ptr: *mut T, length: usize) -> Self {
        let array = Array::from_ptr(ptr, length);
        Self {
            array: array.into_iter().map(|s| AutoPtr::with_allocator(s)).collect(),
            growing: Default::default(),
            writers: Default::default()
        }
    }
}

macro_rules! with_write {
    ($write:expr, $blk:expr) => {
        {
            $write.fetch_add(1, Ordering::AcqRel);
            let out = $blk;
            $write.fetch_sub(1, Ordering::Acquire);
            out
        }
    };
}

#[allow(unused)]
impl<T> SyncArray<T> {



    fn with_grow<F : FnOnce()>(&self, func: F) {
        while self.growing.compare_and_swap(false, true, Ordering::AcqRel) {
            // get growing status
        }
        func();
        self.growing.store(false, Ordering::Release);
    }

    pub const fn new() -> Self {
        Self {
            array: Array::new(),
            growing: AtomicBool::new(false),
            writers: AtomicU32::new(0)
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            array: Array::with_capacity(capacity),
            growing: Default::default(),
            writers: Default::default()
        }
    }

    pub fn of_size_using<F>(default: F, size: usize) -> Self
        where
            F: Fn() -> T,
    {
        let mut ret = Self {
            array: Array::of_size_using(|| AutoPtr::with_allocator(default()), size),
            growing: Default::default(),
            writers: Default::default()
        };
        ret
    }

    pub fn len(&self) -> usize {
        self.array.size
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.wait_for_end_grow();
        if index >= self.array.size {
            None
        } else {
            Some(&self.array[index])
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.wait_for_end_grow();
        if index >= self.array.size {
            None
        } else {
            Some(&mut self.array[index])
        }
    }

    pub fn push(&mut self, val: T) {
        self.wait_for_end_grow();
        let capacity = self.array.capacity();
        if self.len() >= capacity {
            while self.growing.compare_and_swap(false, true, Ordering::Release) {
                // get growing status
            }
            while self.writers.load(Ordering::Acquire) > 0{
                // wait for writers to finish
            }

            let new_size = if self.array.size > 0 { self.array.size * 2 } else { 1 };
            self.array.reserve(new_size);
            self.growing.store(false, Ordering::Release);
        }
        with_write!(
            &self.writers,
            self.array.push(AutoPtr::with_allocator(val))
        )
    }

    pub fn pop(&mut self) -> Option<T> {
        self.wait_for_end_grow();
        with_write!(
            &self.writers,
            {
                let ret = self.array.pop();
                ret.map(|ptr| ptr.take())
            }
        )
    }

    pub fn clear(&mut self) {
        self.wait_for_end_grow();
        with_write!(&self.writers, self.array.clear());
    }

    /// If index is a valid position, replaces the current value at the index with `val` and returns
    /// the previous value
    pub fn swap(&mut self, index: usize, val: T) -> Option<T> {
        self.wait_for_end_grow();
        with_write!(&self.writers, {
            let ret = self.array.swap(index, AutoPtr::with_allocator(val));
            ret.map(|ptr| ptr.take())
        })
    }

    /// removes the element at the index and returns it
    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.wait_for_end_grow();
        with_write!(&self.writers,
                         {
                             let ret = self.array.remove(index);
                             ret.map(|ptr| ptr.take())
                         }
        )
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> ArrayIterator<&T> {
        self.wait_for_end_grow();
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> ArrayIterator<&mut T> {
        self.wait_for_end_grow();
        self.into_iter()
    }



    pub fn capacity(&self) -> usize {
        self.wait_for_end_grow();
        self.array.capacity()
    }

    pub fn reserve(&mut self, new_capacity: usize) {
        self.array.reserve(new_capacity);
    }

    fn wait_for_end_grow(&self) {
        while self.growing.load(Ordering::Relaxed) {}
    }

}


impl<T> Index<usize> for SyncArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .unwrap_or_else(|| panic!("Array index {} out of bounds", index))
    }
}

impl<T> IndexMut<usize> for SyncArray<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("Array index {} out of bounds", index))
    }
}


impl<T> Default for SyncArray<T> {
    fn default() -> Self {
        Self::new()
    }
}


pub struct ArrayIterator<T> {
    index: usize,
    array: SyncArray<T>,
}

impl<T> Iterator for ArrayIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.array.len() {
            None
        } else {
            unsafe {
                let ret = &self.array[self.index] as *const T;
                let ret = ret.read_unaligned();

                self.index += 1;
                Some(ret)
            }
        }
    }
}


impl<T> IntoIterator for SyncArray<T> {
    type Item = T;
    type IntoIter = ArrayIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        ArrayIterator {
            index: 0,
            array: self,
        }
    }
}

impl<'a, T> IntoIterator for &'a SyncArray<T> {
    type Item = &'a T;
    type IntoIter = ArrayIterator<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut out: SyncArray<&'a T> = SyncArray::with_capacity(self.len());

        for i in 0..self.len() {
            let item = self.get(i).unwrap();
            out.push(item);
        }

        ArrayIterator {
            index: 0,
            array: out
        }
    }
}

impl<'a, T> IntoIterator for &'a mut SyncArray<T> {
    type Item = &'a mut T;
    type IntoIter = ArrayIterator<&'a mut T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut out: SyncArray<&'a mut T> = SyncArray::with_capacity(self.len());

        for i in 0..self.len() {
            let item = unsafe {
                let auto = self.array.get_unchecked_mut(i);
                &mut *auto.get_ptr_mut()
            };
            out.push(item)
        }

        ArrayIterator {
            index: 0,
            array: out
        }
    }
}


impl<T> FromIterator<T> for SyncArray<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut output = SyncArray::new();
        for val in iter {
            output.push(val);
        }
        output
    }
}

impl<T: Clone> Clone for SyncArray<T> {
    fn clone(&self) -> Self {
        self.iter().map(|v| v.clone()).collect::<Self>()
    }
}

impl<T: PartialEq> PartialEq for SyncArray<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        loop {
            let self_val = self_iter.next();
            let other_val = other_iter.next();
            if self_val.is_none() || other_val.is_none() {
                return true;
            }

            if self_val != other_val {
                return false;
            }
        }
    }
}

impl<T: Debug> Debug for SyncArray<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let slice = self.deref();
        slice.fmt(f)
    }
}

