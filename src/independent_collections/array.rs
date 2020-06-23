use std::marker::PhantomData;
use std::ptr::null_mut;
use crate::pages::external_mem_reservation::{SEGMENT_ALLOCATOR, SegAllocator, Segment};
use std::ops::{Deref, Index, IndexMut};

pub struct RawArray<T> {
    segment: Option<Segment>,
    _phantom: PhantomData<T>
}

impl<T> RawArray<T> {
    pub const fn new() -> Self {
        Self {
            segment: None,
            _phantom: PhantomData
        }
    }

    pub fn reserve(&mut self, new_capacity: usize) {
        if self.segment.is_some() && new_capacity < self.segment.as_ref().unwrap().len() {
            return;
        }

        let new_ptr = SEGMENT_ALLOCATOR.allocate(new_capacity).unwrap();
        match &mut self.segment {
            None => {
                self.segment = Some(new_ptr);
            },
            Some(old_ptr) => {
                unsafe {
                    std::ptr::copy_nonoverlapping(old_ptr.get_ptr() as *mut T,
                                                  new_ptr.get_ptr() as *mut T,
                                                  old_ptr.len()
                    );
                }
                self.segment = Some(new_ptr);
            },
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut ret = Self::new();
        ret.reserve(capacity);
        ret
    }

    pub fn capacity(&self) -> usize {
        self.segment.as_ref().map_or(0, |s| s.len())
    }
}

impl<T> Drop for RawArray<T> {
    fn drop(&mut self) {
        match std::mem::replace(&mut self.segment, None) {
            None => {},
            Some(segment) => {
                SEGMENT_ALLOCATOR.deallocate(segment);
            },
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
    array: RawArray<T>
}

impl <T : Clone + Default> Array<T> {
    pub fn with_capacity(size: usize) -> Self {
        Self::with_capacity_using(Default::default(), size)
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

impl <T : Clone> Array<T> {
    pub fn with_capacity_using(default: T, size: usize) -> Self {
        let mut ret = Self {
            size,
            array: RawArray::with_capacity(size)
        };
        for i in 0..size {
            let ptr = &mut ret.array[i] as *mut T;
            unsafe {
                ptr.write(default.clone());
            }
        }
        ret
    }
}

impl<T> Array<T> {

    pub fn new() -> Self {
        Self {
            size: 0,
            array: RawArray::new()
        }
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
        if self.size == self.array.capacity() {
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
        if std::mem::needs_drop::<T>() {
            unsafe {
                for i in 0..self.size {
                    let ptr = self.get_ptr(i);
                    std::ptr::drop_in_place(ptr);
                }
            }
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

impl<T> Drop for Array<T> {
    fn drop(&mut self) {
        self.clear()
    }
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
}




