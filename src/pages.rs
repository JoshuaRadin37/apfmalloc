use crate::mem_info::PAGE_MASK;
use bitfield::size_of;
use memmap::MmapMut;
use std::io::ErrorKind;
use std::os::raw::c_void;

use atomic::Ordering;
use bitfield::fmt::{Debug, Display, Formatter};
use std::mem::MaybeUninit;
use std::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use std::sync::atomic::AtomicBool;
use std::{fmt, io};
use crate::pages::external_mem_reservation::{Segment, SEGMENT_ALLOCATOR, SegAllocator};
use crate::pages::MemoryOrFreePointer::Free;

pub mod external_mem_reservation;

#[inline]
#[allow(unused)]
pub fn page_addr2base<T>(a: &T) -> *mut c_void {
    (a as *const T as usize & !PAGE_MASK) as *mut c_void
}

struct PageInfoHolder {
    internals: Option<MmapMut>,
    count: usize,
    capacity: usize,
    head: Option<usize>,
    lock: AtomicBool,
}

#[derive(Debug)]
#[allow(dead_code)]
enum MemoryOrFreePointer {
    Map(MmapMut),
    Segment(Segment),
    Free { next: Option<usize> },
}

impl PartialEq for MemoryOrFreePointer {
    fn eq(&self, other: &Self) -> bool {
        use MemoryOrFreePointer::*;
        match (self, other) {
            (Map(map1), Map(map2)) => map1.as_ptr() == map2.as_ptr(),
            (Free { next: ptr1 }, Free { next: ptr2 }) => ptr1 == ptr2,
            _ => false,
        }
    }
}

pub const INITIAL_PAGES: usize = 128;
const MIN_MAP_ALLOCATION_SIZE: usize = 1 << 14;

impl PageInfoHolder {
    pub const fn new() -> Self {
        Self {
            internals: None,
            count: 0,
            capacity: 0,
            head: None,
            lock: AtomicBool::new(false),
        }
    }

    fn grab(&mut self) {
        while self.lock.compare_and_swap(false, true, Ordering::Acquire) {

        }
    }

    fn release(&mut self) {
        self.lock.compare_and_swap(true, false, Ordering::Release);
    }

    pub fn init(&mut self) {
        self.grab();
        let capacity = INITIAL_PAGES;
        let mem_size = Self::size_for_capacity(capacity);
        let mmap_mut = MmapMut::map_anon(mem_size).expect("Memory map must be created");
        *self = Self {
            internals: Some(mmap_mut),
            count: 0,
            capacity: 0,
            head: None,
            lock: AtomicBool::new(false),
        };
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
        capacity * size_of::<MemoryOrFreePointer>()
    }

    fn get_capacity(&self) -> &usize {
        &self.capacity
    }

    fn get_space_within(slice: &mut [u8]) -> usize {
        slice.len() / size_of::<MemoryOrFreePointer>()
    }

    fn get_maps(&mut self) -> &mut [MemoryOrFreePointer] {
        unsafe {
            slice_from_raw_parts_mut(
                self.internals.as_mut().unwrap().as_mut_ptr() as *mut MemoryOrFreePointer,
                *self.get_capacity(),
            )
                .as_mut()
                .unwrap()
        }
    }

    unsafe fn initialize_slice(&mut self, slice: &mut [u8]) {
        let mut prev = self.head;
        let size = Self::get_space_within(slice);
        let slice = &mut *slice_from_raw_parts_mut(
            slice.as_mut_ptr() as *mut MaybeUninit<MemoryOrFreePointer>,
            size,
        );
        let mut first = true;
        // let mut first_index = None;
        for (index, map_or_pointer) in slice.into_iter().enumerate().rev() {
            //std::mem::swap(map_or_pointer,&mut ;
            //*map_or_pointer =
            *map_or_pointer = MaybeUninit::new(MemoryOrFreePointer::Free { next: prev });
            /*
            let ptr = map_or_pointer as *mut MaybeUninit<MemoryOrFreePointer>;
            let ptr = ptr as *mut MemoryOrFreePointer;

             */
            //map_or_pointer.write();
            if first {
                prev = Some(index + self.capacity);
                first = false;
                // first_index = Some(index + self.capacity);
            } else {
                prev = Some(prev.unwrap() - 1);
            }
        }
        let slice =
            &mut *slice_from_raw_parts_mut(slice.as_mut_ptr() as *mut MemoryOrFreePointer, size);
        // println!("{:?}", slice);

        {
            if let Free { next: next_ptr } = slice.last_mut().unwrap() {
                let old_head = self.head;
                *next_ptr = old_head;
            }
        }
        // let first = &mut slice[0];

        self.head = Some(self.capacity);
    }

    pub fn get_index_from_pointer(&self, ptr: * const MemoryOrFreePointer) -> Option<usize> {
        match &self.internals {
            None => { None },
            Some(map) => {
                let base_ptr = & map[0] as *const u8 as usize;
                if (ptr as usize) < base_ptr {
                    None
                } else {
                    let i = (ptr as usize - base_ptr) / std::mem::size_of::<MemoryOrFreePointer>();
                    if i < self.capacity {
                        Some(i)
                    } else {
                        None
                    }
                }
            },
        }
    }

    fn grow(&mut self) {
        // self.grab();
        let new_capacity = *self.get_capacity() * 2;
        let size = Self::size_for_capacity(new_capacity);
        let mut map = MmapMut::map_anon(size).expect("Should create");
        let prev_index = self.head.unwrap();
        let slice = &mut map.as_mut()[..Self::size_for_capacity(*self.get_capacity())];
        slice.copy_from_slice(&*self.internals.as_mut().unwrap());
        self.head = Some(prev_index); //self.get_index_from_pointer(unsafe { (&mut map[0] as *mut u8 as * mut MemoryOrFreePointer).offset(prev_index as isize)}) ;
        let uninit = &mut map.as_mut()[Self::size_for_capacity(*self.get_capacity())..];
        unsafe {
            self.initialize_slice(uninit);
        }
        self.internals = Some(map);
        self.capacity = new_capacity;
        // self.release();
    }

    pub fn alloc(&mut self, size: usize) -> Result<*mut u8, io::Error> {
        self.grab();
        if self.count == self.capacity - 1 {
            // println!("Growing Page Holder");
            self.grow();
        }
        // println!("Beginning Alloc Page");
        // println!("Before: {:?}", self);
        if self.head.is_none() {
            // panic!("Head is none when it shouldn't be");
        }

        let (memory, ptr) = {

            /*if size > MIN_MAP_ALLOCATION_SIZE {
                let mut map = MmapOptions::new().len(size).map_anon()?;
                let ptr = map.as_mut_ptr();
                let combo = MemoryOrFreePointer::Map(map);

                (combo, ptr)
            } else
            {
             */
                let segment = SEGMENT_ALLOCATOR.allocate(size).expect("Should be able to allocate a space");
                let ptr = segment.get_ptr() as *mut u8;
                let combo = MemoryOrFreePointer::Segment(segment);
                //self.release();
                //return Ok(ptr)
                (combo, ptr)
            // }
        };
        unsafe {
            let head = self.head;
            if let MemoryOrFreePointer::Free { next: prev_pointer } = self.get_at_index(head.unwrap()).unwrap() {
                if prev_pointer.is_none() {
                    panic!("Previous pointer should not be null");
                }
                // println!("Previous pointer: {:x?}", *prev_pointer);
                self.head = *prev_pointer;
                // self.head.store(*prev_pointer, Ordering::SeqCst);
            } else {
                // eprintln!("Head is {:?}", head);
                panic!("No more space in page container")
            }
            *self.get_at_index(head.unwrap()).unwrap() = memory;
            assert_ne!(self.head, head, "Head should not be the same");
            self.count += 1;
        }

        // println!("After: {:?}", self);
        // println!("Finished Alloc Page");
        self.release();
        Ok(ptr)
    }

    fn get_at_index(&self, index: usize) -> Option<& mut MemoryOrFreePointer> {
        match &self.internals {
            None => { None },
            Some(map) => {
                let ptr = &map[0] as * const u8 as * mut MemoryOrFreePointer;
                if index >= self.capacity {
                    None
                } else {
                    unsafe {
                        Some(&mut *ptr.offset(index as isize))
                    }
                }
            },
        }
    }

    pub fn alloc_overcommit(&mut self, size: usize) -> Result<*mut u8, io::Error> {
        self.grab();
        if self.count == self.capacity - 1 {
            // println!("Growing Page Holder");
            self.grow();
        }

        // println!("Beginning Alloc Page");
        // println!("Before: {:?}", self);
        if self.head.is_none() {
            panic!("Head is null when it shouldn't be");
        }

        let (memory, ptr) = {
            let segment = SEGMENT_ALLOCATOR.allocate_massive(size).expect("Should be able to allocate a space");
            let ptr = segment.get_ptr() as *mut u8;
            let combo = MemoryOrFreePointer::Segment(segment);

            (combo, ptr)
        };
        unsafe {
            let head = self.head.unwrap();
            if let MemoryOrFreePointer::Free { next: prev_pointer } = self.get_at_index(head).unwrap() {
                if prev_pointer.is_none() {
                    panic!("Previous pointer should not be null");
                }
                self.head = *prev_pointer;
                // self.head.store(*prev_pointer, Ordering::SeqCst);
            } else {
                panic!("No more space in page container")
            }
            *self.get_at_index(head).unwrap() = memory;
            //*head = memory;
            self.count += 1;
        }

        // println!("After: {:?}", self);
        // println!("Finished Alloc Page");
        self.release();
        Ok(ptr)
    }

    pub fn dealloc(&mut self, page_ptr: *const u8) -> bool {
        self.grab();
        let prev = {
            self.head.clone()
        };
        let mut found_mem = None;
        let new_head =
            {
                for page in self.get_maps() {
                    match page {
                        MemoryOrFreePointer::Map(map) => {
                            if map.as_ptr() == page_ptr {
                                found_mem = Some(page);
                                break;
                            }
                        },
                        MemoryOrFreePointer::Segment(segment) => {
                            if segment.get_ptr() as *const u8 == page_ptr {
                                let x = page;
                                found_mem = Some(x);
                                break;
                            }
                        },
                        MemoryOrFreePointer::Free { next: _ } => {

                        }
                    }
                }

                let output = match found_mem {
                    None => return false,
                    Some(page) => {
                        // println!("De-allocating a page");

                        unsafe {
                            // Now will definitely drop the map
                            let mem = std::ptr::replace(page, MemoryOrFreePointer::Free { next: prev });
                            if let MemoryOrFreePointer::Segment(segment) = mem {
                                // deallocate the segment
                                SEGMENT_ALLOCATOR.deallocate(segment);
                            }
                        };
                        page as * mut MemoryOrFreePointer
                        //self.head
                        //    .store(page as *mut MemoryOrFreePointer, Ordering::Release);
                    }
                };
                output
            };
        self.head = self.get_index_from_pointer(new_head);
        self.count -= 1;
        // println!("{:?}", self);
        self.release();
        true
    }

    #[cfg(test)]
    #[allow(unused)]
    pub fn show_free_list(&self) {
        let head = self.head;
        unsafe {
            // println!("Head: {:?}", head);
            let mut ptr = head;
            loop {


                if ptr.is_none() {
                    // println!("done");
                    break;
                } else {
                    // println!("{:?} ->", ptr);
                }

                if let Some(Free { next }) = self.get_at_index(ptr.unwrap()) {
                    ptr = *next;
                } /*else {
                    panic!("Free list inconsistent")
                } */

            }
        }
    }

    #[allow(unused)]
    pub fn get_free_list(&self) -> Vec<Option<usize>>{
        let mut output = vec![];
        let head = self.head;

        unsafe {
            let mut ptr = head;
            loop {
                output.push(ptr);
                if ptr.is_none() {
                    break;
                }

                if let Some(Free { next }) = self.get_at_index(ptr.unwrap()) {
                    ptr = *next;
                } else {
                    panic!("Free list inconsistent")
                }
            }
        }

        output
    }
}

impl Debug for PageInfoHolder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.internals.is_some() {
            write!(
                f,
                "Head: {:?}, Use: {}/{} Pages: {:?}",
                self.head,
                self.count,
                self.capacity,
                unsafe {
                    (&*slice_from_raw_parts(
                        self.internals.as_ref().unwrap().as_ptr() as *const MemoryOrFreePointer,
                        self.capacity,
                    )).iter()
                        .map(|mem| format!("{:?}: {:?}", self.get_index_from_pointer(mem as *const MemoryOrFreePointer), mem))
                        .collect::<Vec<String>>()
                        .join(", ")
                }
            )
        } else {
            write!(f, "UNINITIALIZED")
        }
    }
}

pub static mut PAGE_HOLDER_INIT: AtomicBool = AtomicBool::new(false);
static mut PAGE_HOLDER: PageInfoHolder = PageInfoHolder::new();

#[derive(Debug)]
pub struct PageMaskError;

impl std::error::Error for PageMaskError {}

impl Display for PageMaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Returns a set of continuous pages, totaling to size bytes
pub fn page_alloc(size: usize) -> Result<*mut u8, io::Error> {
    if size & PAGE_MASK != 0 {
        return Err(io::Error::new(ErrorKind::InvalidData, PageMaskError));
    }

    unsafe {
        //println!("PAGE_HOLDER_INIT: {:?}", PAGE_HOLDER_INIT);
        if PAGE_HOLDER_INIT.compare_and_swap(false, true, Ordering::AcqRel) == false {
            // println!("PAGE_HOLDER_INIT: {:?}", PAGE_HOLDER_INIT);
            //print!("page alloc initializing the page holder...");
            PAGE_HOLDER.init();
            //println!(" done");
        }

        while PAGE_HOLDER.capacity == 0 {
            //println!("Waiting for PAGE_HOLDER...")
        }
        PAGE_HOLDER.alloc(size)
    }
}

/// Explicitly allow overcommitting
///
/// Used for array-based page map
pub fn page_alloc_over_commit(size: usize) -> Result<*mut u8, io::Error> {
    if size & PAGE_MASK != 0 {
        return Err(io::Error::new(ErrorKind::InvalidData, PageMaskError));
    }

    unsafe {
        // println!("PAGE_HOLDER_INIT: {:?}", PAGE_HOLDER_INIT);
        if PAGE_HOLDER_INIT.compare_and_swap(false, true, Ordering::AcqRel) == false {
            // println!("PAGE_HOLDER_INIT: {:?}", PAGE_HOLDER_INIT);
            // print!("page alloc initializing the page holder...");
            PAGE_HOLDER.init();
            // println!(" done");
        }

        while PAGE_HOLDER.capacity == 0 {
            // println!("Waiting for PAGE_HOLDER...")
        }
        PAGE_HOLDER.alloc_overcommit(size)
    }
}

/// Altered version of the lralloc free, which uses the drop method
/// of MMapMut struct
pub fn page_free(ptr: *const u8) -> bool {
    unsafe { PAGE_HOLDER.dealloc(ptr) }
}

#[cfg(test)]
mod test {
    use crate::pages::{page_alloc, PAGE_HOLDER, INITIAL_PAGES};
    use crate::mem_info::PAGE;
    use crate::size_classes::{SIZE_CLASSES, SizeClassData};

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
        let ptr = page_alloc(4096).expect("Couldn't get page") as *mut usize;
        unsafe {
            *ptr = 0xdeadbeaf; // if this fails it means the test fails
        }
    }

    #[test]
    fn mass_allocate() {

        unsafe {
            crate::init_malloc();
        }
        let sc: &mut SizeClassData = unsafe { &mut SIZE_CLASSES[1] };
        let size = sc.sb_size;

        for _ in 0..10000 {
            page_alloc(size as usize).unwrap();
        }
    }

    mod safety {
        use super::*;

        #[test]
        fn deallocate() {
            for _ in 0..8 {
                let ptr = page_alloc(4096).expect("Couldn't get page") as *mut usize;
                page_alloc(4096).expect("Couldn't get page") as *mut usize; // double it up
                unsafe {
                    *ptr = 0xdeadbeaf;
                    PAGE_HOLDER.show_free_list();
                    assert!(PAGE_HOLDER.dealloc(ptr as *const u8));
                    // uncommenting this causes a fault
                    // *ptr = 0xdeadbeaf;
                    PAGE_HOLDER.show_free_list();
                }
            }
        }

        #[test]
        //#[ignore]
        fn grows() {
            // get_page();

            unsafe {
                for _i in 0..(INITIAL_PAGES * 2).max(128) {
                    //println!("{:?}", PAGE_HOLDER);
                    // println!("Allocating page {:?}", _i);
                    page_alloc(4096).unwrap();
                    PAGE_HOLDER.show_free_list();
                }

                assert!(PAGE_HOLDER.count >= INITIAL_PAGES * 2);
                assert!(unsafe { PAGE_HOLDER.capacity } > INITIAL_PAGES * 2)
            }
        }
    }
}
