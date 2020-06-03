extern crate libc;


use std::io;
use std::ffi::{c_void, NulError};

#[cfg(windows)] use winapi::{
    shared::{
        ntdef::HANDLE,
        winerror::S_OK
    },
    um::{
        heapapi::{GetProcessHeap, HeapAlloc, HeapFree, HeapSummary, HEAP_SUMMARY, LPHEAP_SUMMARY},
        winnt::HEAP_ZERO_MEMORY
    }
};
use std::io::ErrorKind;
use std::fmt::Error;


pub struct Segment {
    ptr: * mut c_void,
    length: usize
}

impl Segment {
    pub fn new(ptr: *mut c_void, length: usize) -> Self {
        Segment { ptr, length }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn get_ptr(&self) -> *mut c_void {
        self.ptr
    }
}


pub struct SegmentAllocator;

pub static SEGMENT_ALLOCATOR: SegmentAllocator = SegmentAllocator;

/// This trait allows for multiple implementations for the SegmentAllocator, instead of needing different structs and statics for different
/// platforms
///
pub trait SegAllocator {

    /// Must guarantee that a segment is returned safetly, or results in an error.
    /// It must no panic when called
    fn allocate(&self, size: usize) -> Result<Segment, io::Error>;
    /// De-allocates a segment. Depending on the platform, this may not do anything
    fn deallocate(&self, segment: Segment) -> bool;

}

#[cfg(windows)]
impl SegAllocator for SegmentAllocator {


    fn allocate(&self, size: usize) -> Result<Segment, io::Error> {
        unsafe {

            let heap: HANDLE = GetProcessHeap();


            let alloc = HeapAlloc(heap, HEAP_ZERO_MEMORY, size);
            #[cfg(debug_assertions)]
                #[allow(non_snake_case)]
                {
                    let mut heap_summary: HEAP_SUMMARY = HEAP_SUMMARY {
                        cb: 0,
                        cbAllocated: 0,
                        cbCommitted: 0,
                        cbReserved: 0,
                        cbMaxReserve: 0
                    };
                    match HeapSummary(heap, 0, &mut heap_summary as LPHEAP_SUMMARY) {
                        S_OK => {
                            let HEAP_SUMMARY { cb: _, cbAllocated, cbCommitted, cbReserved, cbMaxReserve } = heap_summary;
                            println!("HEAP SUMMARY");
                            println!("\tAllocated: {:?}", cbAllocated);
                            println!("\tCommitted: {:?}", cbCommitted);
                            println!("\tReserved: {:?}", cbReserved);
                            println!("\tMax Reserve: {:?}", cbMaxReserve);
                        }
                        _ => {
                            panic!("Unable to get the heap summary")
                        }
                    }
                }
            if alloc.is_null() {
                Err(io::Error::new(ErrorKind::AddrNotAvailable, Error))
            } else {
                let seg = Segment::new(
                    alloc,
                    size
                );
                Ok(seg)
            }
        }
    }

    fn deallocate(&self, segment: Segment) -> bool {
        unsafe {
            let heap: HANDLE = GetProcessHeap();
            if HeapFree(heap, 0, segment.ptr) != 0 {
                true
            } else {
                false
            }
        }
    }
}

#[cfg(unix)]
impl SegAllocator for SegmentAllocator {
    fn allocate(&self, size: usize) -> Result<Segment, io::Error> {
        unimplemented!()
    }

    fn deallocate(&self, segment: Segment) -> bool {
        unimplemented!()
    }
}


#[cfg(test)]
mod test {
    use crate::pages::external_mem_reservation::{SEGMENT_ALLOCATOR, SegAllocator, Segment};
    use crate::mem_info::PAGE;
    use std::ffi::c_void;

    #[test]
    pub fn get_segment() {
        unsafe { SEGMENT_ALLOCATOR.allocate(PAGE) }.expect("Test must fail is this fails");
    }

    #[test]
    pub fn free_segment() {
        unsafe {
            let segment = SEGMENT_ALLOCATOR.allocate(PAGE).expect("Test must fail is this fails");
            assert!(SEGMENT_ALLOCATOR.deallocate(segment));
        }
    }

    #[test]
    pub fn write_to_segment() {
        unsafe {
            let segment = SEGMENT_ALLOCATOR.allocate(PAGE).expect("Test must fail is this fails");
            let ptr1 = segment.get_ptr() as *mut usize;
            (*ptr1) = 0xdeadbeaf;

            let segment = SEGMENT_ALLOCATOR.allocate(PAGE).expect("Test must fail is this fails");
            let ptr2 = segment.get_ptr() as *mut usize;
            (*ptr2) = 0xdeadbeaf;

            assert_eq!(*ptr1, *ptr2)
        }
    }



}
