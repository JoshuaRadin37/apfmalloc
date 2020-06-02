use std::os::raw::c_void;
use crate::mem_info::PAGE_MASK;
use memmap::{MmapMut, MmapOptions};
use std::io::{ErrorKind, Error};
use bitfield::size_of;
use std::iter::Map;
use std::ops::Mul;
use std::sync::atomic::AtomicPtr;
use std::ptr::{slice_from_raw_parts_mut, null_mut, replace};
use atomic::Ordering;
use std::mem::MaybeUninit;

#[inline]
pub fn page_addr2base<T>(a : &T) -> * mut c_void {
    unsafe {
        (a as * const T as usize & !PAGE_MASK) as *mut c_void
    }
}

struct PageInfoHolder {
    internals: Option<MmapMut>,
    count: usize,
    capacity: usize,
    head: AtomicPtr<MapOrFreePointer>
}

#[derive(Debug)]
enum MapOrFreePointer {
    Map(MmapMut),
    Pointer(* mut MapOrFreePointer)
}

impl PartialEq for MapOrFreePointer {
    fn eq(&self, other: &Self) -> bool {
        use MapOrFreePointer::*;
        match (self, other) {
            (Map(map1), Map(map2)) => {
                map1.as_ptr() == map2.as_ptr()
            },
            (Pointer(ptr1), Pointer(ptr2)) => {
                ptr1 == ptr2
            },
            _ => false
        }
    }
}

impl PageInfoHolder {

    pub const fn new() -> Self {
        Self { internals: None, count: 0, capacity: 0, head: AtomicPtr::new(null_mut()) }
    }

    pub fn init(&mut self) {
        let capacity = 1024;
        let mem_size = Self::size_for_capacity(capacity);
        let mmap_mut = MmapMut::map_anon(mem_size).expect("Memory map must be created");
        *self = Self { internals: Some(mmap_mut), count: 0, capacity: 0, head: AtomicPtr::new(null_mut()) };
        let ptr = self.internals.as_mut().unwrap().as_mut_ptr();
        unsafe {
            let head = &mut *slice_from_raw_parts_mut(ptr, mem_size);
            self.initialize_slice(head);
        }
        self.capacity = capacity
    }

    fn size_for_capacity(capacity: usize) -> usize {
        capacity * size_of::<MapOrFreePointer>()
    }

    fn get_capacity(&self) -> &usize {
        &self.capacity
    }

    fn get_space_within(slice: &mut [u8]) -> usize {
        slice.len() / size_of::<MapOrFreePointer>()
    }

    fn get_maps(&mut self) -> &mut [MapOrFreePointer] {
        unsafe { slice_from_raw_parts_mut(self.internals.as_mut().unwrap().as_mut_ptr() as *mut MapOrFreePointer, *self.get_capacity()).as_mut().unwrap() }
    }

    unsafe fn initialize_slice(&mut self, slice: &mut [u8]) {
        let mut prev = if self.capacity == 0 {
            null_mut()
        } else {
            self.head.load(Ordering::Acquire)
        };
        let size = Self::get_space_within(slice);
        let mut slice = &mut *slice_from_raw_parts_mut(slice.as_mut_ptr() as *mut MaybeUninit<MapOrFreePointer>, size);
        for map_or_pointer in slice.into_iter().rev() {

            //std::mem::swap(map_or_pointer,&mut ;
            //*map_or_pointer =
            *map_or_pointer = MaybeUninit::new(MapOrFreePointer::Pointer(prev));
            let ptr = map_or_pointer as *mut MaybeUninit<MapOrFreePointer>;
            let ptr = ptr as *mut MapOrFreePointer;
            //map_or_pointer.write();
            prev = ptr;
        }
        let first = slice[0].as_mut_ptr();
        self.head.store(first, Ordering::SeqCst);
    }

    fn grow(&mut self) {
        let new_capacity = *self.get_capacity() * 2;
        let size = Self::size_for_capacity(new_capacity);
        let mut map = MmapMut::map_anon(size).expect("Should create");
        let mut slice = &mut map.as_mut()[..Self::size_for_capacity(*self.get_capacity())];
        slice.copy_from_slice(& *self.internals.as_mut().unwrap());
        let mut uninit = &mut map.as_mut()[Self::size_for_capacity(*self.get_capacity())..];
        unsafe {
            self.initialize_slice(uninit);
        }
        self.internals = Some(map);
        self.capacity = new_capacity;
    }

    pub fn alloc(&mut self, size: usize) -> Result<* mut u8, ()> {
        if self.count == self.capacity {
            self.grow();
        }

        let mut map = MmapOptions::new().len(size).map_anon().map_err(|_| ())?;
        let ptr = map.as_mut_ptr();
        let combo = MapOrFreePointer::Map(map);
        unsafe {
            let head = &mut *self.head.load(Ordering::Acquire);
            if let MapOrFreePointer::Pointer(prev_pointer) = head {
                self.head.store(*prev_pointer, Ordering::Release);
            } else {
                panic!("No more space in page container")
            }
            *head = combo;
            self.count += 1;
        }
        Ok(ptr)
    }

    pub fn dealloc(&mut self, page_ptr: *const u8) -> bool {
        let mut found_map = None;
        {
            for page in self.get_maps() {
                let mut is_map = false;
                match &page {
                    MapOrFreePointer::Map(map) => {
                        if map.as_ptr() == page_ptr {
                            is_map = true;
                            found_map = Some(page as *mut MapOrFreePointer);
                            break;
                        }
                    },
                    MapOrFreePointer::Pointer(_) => {},
                }
            }
        }
        match found_map {
            None => {
                false
            },
            Some(page) => {
                let prev = self.head.load(Ordering::Acquire);
                unsafe { *page = MapOrFreePointer::Pointer(prev) };
                self.head.store(page as *mut MapOrFreePointer, Ordering::Release);
                self.count -= 1;
                true

            },
        }
    }

}

static mut PAGE_HOLDER: PageInfoHolder = PageInfoHolder::new();

/// Returns a set of continuous pages, totaling to size bytes
pub fn page_alloc(size: usize) -> Result<*mut u8, ()> {
    if size & PAGE_MASK != 0 {
        return Err(())
    }

    unsafe {
        if PAGE_HOLDER.capacity == 0 {
            PAGE_HOLDER.init();
        }

        PAGE_HOLDER.alloc(size)
    }
}

/// Explicitly allow overcommitting
///
/// Used for array-based page map
///
/// TODO: Figure out how this is different
pub fn page_alloc_over_commit(size: usize) -> Result<*mut u8, ()> {
    page_alloc(size)
}

/// Altered version of the lralloc free, which uses the drop method
/// of MMapMut struct
pub fn page_free(ptr: *const u8) -> bool {
    unsafe {
        PAGE_HOLDER.dealloc(ptr)
    }
}

#[cfg(test)]
mod test {
    use crate::pages::{page_alloc, PAGE_HOLDER, MapOrFreePointer, PageInfoHolder};
    use atomic::Ordering;
    use crate::pages::MapOrFreePointer::Pointer;
    use std::ptr::null_mut;

    #[test]
    fn get_page() {
        let ptr = page_alloc(4096).expect("Couldn't get page");
        assert!(!ptr.is_null());
        unsafe {
            assert!(PAGE_HOLDER.capacity > 0);
            assert!(PAGE_HOLDER.count >= 1)
        }
    }

    #[test]
    fn can_write_to_page() {
        let ptr = page_alloc(4096).expect("Couldn't get page") as * mut usize;
        unsafe {
            *ptr = 0xdeadbeaf; // if this fails it means the test fails
        }
    }

    #[test]
    fn deallocate() {
        let ptr = page_alloc(4096).expect("Couldn't get page") as * mut usize;
        unsafe {
            *ptr = 0xdeadbeaf;
            assert!(PAGE_HOLDER.dealloc(ptr as *const u8));
            // uncommenting this causes a fault
            // *ptr = 0xdeadbeaf;
        }
    }

    #[test]
    fn grows() {
        let mut allocator = PageInfoHolder::new();
        allocator.init();
        for i in 0..2048 {
            allocator.alloc(8);
        }
        assert_eq!(allocator.count, 2048);
        assert!(allocator.capacity > 1028)
    }


}