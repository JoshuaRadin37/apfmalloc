use std::os::raw::c_void;
use crate::mem_info::PAGE_MASK;
use memmap::{MmapMut, MmapOptions};
use std::io::{ErrorKind, Error};
use bitfield::size_of;
use std::iter::Map;
use std::ops::Mul;
use std::sync::atomic::{AtomicPtr, AtomicBool};
use std::ptr::{slice_from_raw_parts_mut, null_mut, replace, slice_from_raw_parts, null};
use atomic::{Ordering, Atomic};
use std::mem::MaybeUninit;
use bitfield::fmt::{Debug, Formatter, Display};
use std::{fmt, io};

mod external_mem_reservation;

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
    head: AtomicPtr<MapOrFreePointer>,
    lock: AtomicBool
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

pub const INITIAL_PAGES: usize = 256;

impl PageInfoHolder {

    pub const fn new() -> Self {
        Self { internals: None, count: 0, capacity: 0, head: AtomicPtr::new(null_mut()), lock: AtomicBool::new(false) }
    }

    fn grab(&mut self) {
        while !self.lock.compare_and_swap(false, true, Ordering::Acquire) {}
    }

    fn release(&mut self) {
        self.lock.compare_and_swap(true, false, Ordering::Release);
    }

    pub fn init(&mut self) {
        self.grab();
        let capacity = INITIAL_PAGES;
        let mem_size = Self::size_for_capacity(capacity);
        let mmap_mut = MmapMut::map_anon(mem_size).expect("Memory map must be created");
        *self = Self { internals: Some(mmap_mut), count: 0, capacity: 0, head: AtomicPtr::new(null_mut()), lock: AtomicBool::new(false) };
        let ptr = self.internals.as_mut().unwrap().as_mut_ptr();
        unsafe {
            let head = &mut *slice_from_raw_parts_mut(ptr, mem_size);
            self.initialize_slice(head);
        }
        self.capacity = capacity;
        //println!("PageInfoHolder initialized to {:?}", self);
        self.release();

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
        let slice = &mut *slice_from_raw_parts_mut(slice.as_mut_ptr() as *mut MapOrFreePointer, size);
        // println!("{:?}", slice);
        let first = &mut slice[0];
        self.head.store(first, Ordering::SeqCst);
    }

    fn grow(&mut self) {
        self.grab();
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
        self.release();
    }

    pub fn alloc(&mut self, size: usize) -> Result<* mut u8, io::Error> {

        if self.count == self.capacity - 1 {
            println!("Growing Page Holder");
            self.grow();
        }
        self.grab();
        println!("Beginning Alloc Page");
        println!("Before: {:?}", self);
        if self.head.load(Ordering::SeqCst).is_null() {
            panic!("Head is null when it shouldn't be");
        }

        let mut map = MmapOptions::new().len(size).map_anon()?;
        let ptr = map.as_mut_ptr();
        let combo = MapOrFreePointer::Map(map);
        unsafe {
            let head = &mut *self.head.load(Ordering::SeqCst);
            if let MapOrFreePointer::Pointer(prev_pointer) = head {
                if prev_pointer.is_null() {
                    panic!("Previous pointer should not be null");
                }
                self.head.store(*prev_pointer, Ordering::SeqCst);
            } else {
                panic!("No more space in page container")
            }
            *head = combo;
            self.count += 1;
        }

        println!("After: {:?}", self);
        println!("Finished Alloc Page");
        self.release();
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
                self.grab();
                println!("De-allocating a page");
                let prev = self.head.load(Ordering::Acquire);
                unsafe { *page = MapOrFreePointer::Pointer(prev) };
                self.head.store(page as *mut MapOrFreePointer, Ordering::Release);
                self.count -= 1;
                self.release();
                true

            },
        }
    }

}

impl Debug for PageInfoHolder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Head: {:?}, Use: {}/{} Pages: {:?}", self.head, self.count, self.capacity, unsafe {& *slice_from_raw_parts(self.internals.as_ref().unwrap().as_ptr() as *const MapOrFreePointer, self.capacity) })
    }
}

pub static mut PAGE_HOLDER_INIT: AtomicBool = AtomicBool::new(false);
static mut PAGE_HOLDER: PageInfoHolder = PageInfoHolder::new();

#[derive(Debug)]
pub struct PageMaskError;

impl std::error::Error for PageMaskError {

}

impl Display for PageMaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Returns a set of continuous pages, totaling to size bytes
pub fn page_alloc(size: usize) -> Result<*mut u8, io::Error> {
    if size & PAGE_MASK != 0 {
        return Err(io::Error::new(ErrorKind::InvalidData, PageMaskError))
    }

    unsafe {
        println!("PAGE_HOLDER_INIT: {:?}", PAGE_HOLDER_INIT);
        if PAGE_HOLDER_INIT.compare_and_swap(false, true, Ordering::AcqRel) == false {
            println!("PAGE_HOLDER_INIT: {:?}", PAGE_HOLDER_INIT);
            print!("page alloc initializing the page holder...");
            PAGE_HOLDER.init();
            println!(" done");
        }

        while PAGE_HOLDER.capacity == 0 {
            println!("Waiting for PAGE_HOLDER...")
        }
        PAGE_HOLDER.alloc(size)
    }
}

/// Explicitly allow overcommitting
///
/// Used for array-based page map
///
/// TODO: Figure out how this is different
pub fn page_alloc_over_commit(size: usize) -> Result<*mut u8, io::Error> {
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
    use super::*;
    use crate::mem_info::PAGE;

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
        unsafe {
            for i in 0..256 {
                page_alloc(4096).unwrap();
            };


            assert_eq!(PAGE_HOLDER.count, 256);
            assert!(PAGE_HOLDER.capacity > 256)
        }

    }


}