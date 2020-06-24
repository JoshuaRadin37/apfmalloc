use std::marker::PhantomData;
use std::ptr::{null_mut, slice_from_raw_parts_mut};
use crate::pages::external_mem_reservation::{SEGMENT_ALLOCATOR, SegAllocator, Segment};
use std::ops::{Deref, Index, IndexMut, RangeFrom, RangeTo};
use std::ptr::slice_from_raw_parts;
use std::ops::DerefMut;
use std::iter::FromIterator;
use std::fmt::Debug;
use std::fmt::Formatter;
use crate::mem_info::align_val;

struct RawArray<T> {
    segment: Option<Segment>,
    no_dealloc: bool,
    _phantom: PhantomData<T>
}

impl<T> RawArray<T> {
    pub const fn new() -> Self {
        Self {
            segment: None,
            no_dealloc: false,
            _phantom: PhantomData
        }
    }

    pub fn reserve(&mut self, new_capacity: usize) {
        if self.segment.is_some() && new_capacity < self.capacity() {
            return;
        }

        let initial_size = new_capacity * std::mem::size_of::<T>();
        let actual_size = align_val(initial_size, std::mem::align_of::<T>());
        let new_ptr = SEGMENT_ALLOCATOR.allocate(actual_size).unwrap();
        match &mut self.segment {
            None => {
                self.segment = Some(new_ptr);
            },
            Some(old_ptr) => {
                unsafe {
                    std::ptr::copy_nonoverlapping(old_ptr.get_ptr() as *mut u8,
                                                  new_ptr.get_ptr() as *mut u8,
                                                  old_ptr.len()
                    );
                }
                let old = std::mem::replace(&mut self.segment, Some(new_ptr));
                if let Some(segment) = old {
                    if segment.get_ptr() != self.segment.as_ref().unwrap().get_ptr() {
                        SEGMENT_ALLOCATOR.deallocate(segment);
                    }
                }
            },
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut ret = Self::new();
        ret.reserve(capacity);
        ret
    }

    pub fn get_ptr(&self) -> *mut T {
        match &self.segment {
            None => { null_mut() },
            Some(segment) => {
                segment.get_ptr() as *mut T
            },
        }
    }

    pub fn capacity(&self) -> usize {
        self.segment.as_ref().map_or(0, |s| s.len() / std::mem::size_of::<T>())
    }

    pub fn clear(&mut self) {
        if !self.no_dealloc {
            match std::mem::replace(&mut self.segment, None) {
                None => {},
                Some(_segment) => {
                    //SEGMENT_ALLOCATOR.deallocate(segment);
                },
            }
        }
    }

}


impl<T> Index<usize> for RawArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            &*self.segment.as_ref().map_or(null_mut(), |s| s.get_ptr() as *mut T).add(index)
        }
    }
}

impl<T> IndexMut<usize> for RawArray<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            &mut *self.segment.as_ref().map_or(null_mut(), |s| s.get_ptr() as *mut T).add(index)
        }
    }
}

pub struct Array<T> {
    size: usize,
    no_dealloc: bool,
    array: RawArray<T>
}

impl <T : Default> Array<T> {
    pub fn with_capacity(size: usize) -> Self {
        Self::with_capacity_using(|| Default::default(), size)
    }

    pub unsafe fn from_ptr(ptr: *mut T, length: usize) -> Self {
        use std::ffi::c_void;
        let mut ret = Self {
            size: length,
            no_dealloc: true,
            array: RawArray {
                segment: Some(Segment::new(ptr as *mut c_void, length)),
                no_dealloc: true,
                _phantom: PhantomData
            }
        };
        for element in ret.iter_mut() {
            (element as *mut T).write(T::default());
        }
        ret
    }

    pub fn grow(&mut self, new_size: usize) {
        if new_size > self.size {
            self.array.reserve(new_size);
            for i in self.size..new_size {
                let ptr = &mut self.array[i] as *mut T;
                unsafe {
                    ptr.write(T::default());
                }
            }
            self.size = new_size;
        }
    }
}

#[allow(unused)]
impl<T> Array<T> {

    pub const fn new() -> Self {
        Self {
            size: 0,
            no_dealloc: false,
            array: RawArray::new()
        }
    }



    pub fn with_capacity_using<F>(default: F, size: usize) -> Self where F : Fn() -> T {
        let mut ret = Self {
            size,
            no_dealloc: false,
            array: RawArray::with_capacity(size)
        };
        for i in 0..size {
            let ptr = &mut ret.array[i] as *mut T;
            unsafe {
                ptr.write(default());
            }
        }
        ret
    }

    pub fn len(&self) -> usize {
        self.size
    }


    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.size {
            None
        } else {
            Some(&self.array[index])
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.size {
            None
        } else {
            Some(&mut self.array[index])
        }
    }

    pub fn push(&mut self, val: T) {
        let capacity = self.array.capacity();
        if self.size >= capacity {
            let new_size = if self.size > 0 {
                self.size * 2
            } else {
                1
            };
            self.array.reserve(new_size);
        }
        unsafe {
            let index = self.size;
            (&mut self.array[index] as *mut T).write(val)
        }
        self.size += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }
        self.size -= 1;
        unsafe {
            let index = self.size;
            Some((&mut self.array[index] as *mut T).read())
        }
    }

    fn get_ptr(&self, offset: usize) -> *mut T {
        self.array.segment.as_ref().map_or(null_mut(), |s| unsafe { (s.get_ptr() as *mut T).add(offset) })
    }

    pub fn clear(&mut self) {
        unsafe {
            for element in self.deref_mut() {
                std::ptr::drop_in_place(element);
            }
        }
        self.array.clear();
        self.size = 0;
    }

    /// If index is a valid position, replaces the current value at the index with `val` and returns
    /// the previous value
    pub fn swap(&mut self, index: usize, val: T) -> Option<T> {
        if index >= self.size {
            return None;
        }
        let ptr = &mut self[index] as *mut T;
        unsafe {
            let ret = ptr.read();
            ptr.write(val);
            Some(ret)
        }
    }

    /// removes the element at the index and returns it
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.size {
            return None;
        }
        let pre_pops = self.len() - index - 1;
        let mut saved = Array::new();
        for _ in 0..pre_pops {
            saved.push(self.pop().unwrap());
        }
        let ret = self.pop();
        saved.reverse();
        for saved in saved {
            self.push(saved);
        }
        ret
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> ArrayIterator<&T> {
        let mut arr = Array::new();
        for i in 0..self.size {
            arr.push(&self[i])
        }
        ArrayIterator {
            index: 0,
            array: arr
        }
    }
}

impl Array<u8> {

    /// Receives a bit index and returns (the index of the byte, the index of the bit within the byte)
    fn byte_index(bit_index: usize) -> (usize, u8) {
        (bit_index % 8, bit_index as u8 / 8)
    }

    pub fn get_byte(&self, index: usize) -> Option<&u8> {
        self.get(index)
    }
    pub fn get_byte_mut(&mut self, index: usize) -> Option<&mut u8> {
        self.get_mut(index)
    }

    fn bit_mask(index: u8) -> u8 {
        let mut mask = 0b1u8;
        mask = mask << index;
        mask
    }

    pub fn get_bit(&self, bit_index: usize) -> Option<bool> {
        let (byte_index, bit_index) = Self::byte_index(bit_index);
        match self.get_byte(byte_index) {
            None => { None },
            Some(byte) => {
                let mask = Self::bit_mask(bit_index);
                Some((byte & mask) != 0)
            },
        }
    }

    pub fn set_bit(&mut self, bit_index: usize, bit: bool) {
        let (byte_index, bit_index) = Self::byte_index(bit_index);
        match self.get_byte_mut(byte_index) {
            None => {
                panic!("Index {} out of bounds", bit_index)
            },
            Some(byte) => {
                let mask = !Self::bit_mask(bit_index);
                let shifted = (bit as u8) << bit_index;
                *byte = (*byte & mask) | shifted;
            },
        }
    }
}



impl<T> Index<usize> for Array<T>{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap_or_else(|| panic!("Array index {} out of bounds", index))
    }
}

impl<T> IndexMut<usize> for Array<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap_or_else(|| panic!("Array index {} out of bounds", index))
    }
}




impl<T> Index<RangeFrom<usize>> for Array<T> {
    type Output = [T];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> IndexMut<RangeFrom<usize>> for Array<T> {
    fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl<T> Index<RangeTo<usize>> for Array<T> {
    type Output = [T];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> IndexMut<RangeTo<usize>> for Array<T> {
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}



impl<T> Drop for Array<T> {
    fn drop(&mut self) {
        if !self.no_dealloc {
            self.clear()
        }
    }
}

impl <T> Default for Array<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ArrayIterator<T> {
    index: usize,
    array: Array<T>
}

impl <T> Iterator for ArrayIterator<T>{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.array.len() {
            None
        } else {
            unsafe {
                let ret = &self.array[self.index] as *const T;
                let ret =
                    ret.read_unaligned();

                self.index += 1;
                Some(ret)
            }
        }
    }
}

impl <T> IntoIterator for Array<T> {
    type Item = T;
    type IntoIter = ArrayIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        ArrayIterator {
            index: 0,
            array: self
        }
    }
}

impl <T> Deref for Array<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let ptr = self.array.get_ptr();
        if ptr.is_null() {
            &[]
        } else {
            unsafe {
                &*slice_from_raw_parts(
                    ptr, self.size
                )
            }
        }
    }
}

impl <T> DerefMut for Array<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = self.array.get_ptr();
        if ptr.is_null() {
            &mut []
        } else {
            unsafe {
                &mut *slice_from_raw_parts_mut(
                    ptr, self.size
                )
            }
        }
    }
}


impl <T> FromIterator<T> for Array<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        let mut output = Array::new();
        for val in iter {
            output.push(val);
        }
        output
    }
}

impl <T : Clone> Clone for Array<T> {
    fn clone(&self) -> Self {
        self.iter().map(|v| v.clone()).collect::<Self>()
    }
}

impl <T: PartialEq> PartialEq for Array<T> {
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

impl <T : Debug> Debug for Array<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let slice = self.deref();
        slice.fmt(f)
    }
}

#[macro_export]
macro_rules! array {
    ($($element:expr),*) => {
        {
            let mut ret = Array::new();
            $(ret.push($element);
            )*
            ret
        }
    };
}

#[cfg(test)]
mod test {
    use crate::independent_collections::array::Array;

    #[test]
    fn can_use_array() {
        let mut arr: Array<usize> = Array::with_capacity(10);
        arr[5] = 7;
        arr[3] = 7;
        assert_eq!(arr[5], arr[3]);
        assert_eq!(arr[5], 7);
    }

    #[test]
    fn can_push() {
        let mut arr: Array<usize> = Array::new();
        arr.push(15);
        arr.push(5);
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[1], 5);
    }

    #[test]
    fn can_pop() {
        let mut arr: Array<usize> = Array::new();
        arr.push(15);
        arr.push(5);
        assert_eq!(arr.pop(), Some(5));
        assert_eq!(arr.pop(), Some(15));
        assert_eq!(arr.pop(), None);
    }

    #[test]
    #[should_panic]
    fn out_of_bounds() {
        let arr: Array<usize> = Array::new();
        let _i = arr[10];

    }

    #[test]
    fn with_capacity() {
        let arr: Array<usize> = Array::with_capacity(15);
        let _i = arr[14];
    }

    #[test]
    fn can_remove() {
        let mut arr = array![1, 2, 3, 4, 5, 6, 7];
        assert_eq!(arr.len(), 7);
        let received = arr.remove(3);
        assert!(received.is_some());
        let received = received.unwrap();
        assert_eq!(received, 4);
        assert_eq!(arr.len(), 6);
        assert_eq!(arr[3], 5);
        assert_eq!(arr, array![1, 2, 3, 5, 6, 7]);
        println!("{:?}", arr);
    }

}




