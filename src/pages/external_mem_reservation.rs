extern crate libc;

use std::ffi::c_void;
use std::fmt;

#[cfg(windows)]
use winapi::{
    shared::{ntdef::HANDLE, winerror::S_OK},
    um::{
        heapapi::{
            GetProcessHeap, HeapAlloc, HeapCreate, HeapDestroy, HeapFree, HeapSummary,
            HEAP_SUMMARY, LPHEAP_SUMMARY,
        },
        memoryapi::{VirtualAlloc, VirtualFree},
        winnt::{HEAP_ZERO_MEMORY, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE},
    },
};

use crate::pages::external_mem_reservation::AllocationError::AllocationFailed;
use errno::Errno;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use winapi::shared::minwindef::LPVOID;

/// Represents a "segment" in memory- a contiguous section of memory.
#[derive(Debug)]
pub struct Segment {
    ptr: *mut c_void,
    #[cfg(windows)]
    heap: HANDLE,
    length: usize,
}

unsafe impl Send for Segment {}

impl Segment {
    #[cfg(windows)]
    pub fn new(ptr: *mut c_void, heap: HANDLE, length: usize) -> Self {
        Segment { ptr, heap, length }
    }

    #[cfg(unix)]
    pub fn new(ptr: *mut c_void, length: usize) -> Self {
        Segment { ptr, length }
    }

    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn get_ptr(&self) -> *mut c_void {
        self.ptr
    }
}

/// The struct that implements [SegAllocator](trait.SegAllocator.html)
pub struct SegmentAllocator;

#[derive(Debug)]
pub enum AllocationError {
    #[cfg(windows)]
    NoHeap,
    #[cfg(windows)]
    HeapNotCreated(usize),
    AllocationFailed(usize, Errno),
}

impl Display for AllocationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for AllocationError {}

/// The struct that contains the API to request memory from the kernel. It can also deallocate requested memory.
///
pub static SEGMENT_ALLOCATOR: SegmentAllocator = SegmentAllocator;
/// If necessary, this can be used to lock the SEGMENT_ALLOCATOR so only one thread can access it at a time
pub static LOCK: AtomicBool = AtomicBool::new(false);

/// This trait allows for multiple implementations for the SegmentAllocator, instead of needing different structs and statics for different
/// platforms
///
pub trait SegAllocator {
    /// Must guarantee that a segment is returned safetly, or results in an error.
    /// It must no panic when called
    fn allocate(&self, size: usize) -> Result<Segment, AllocationError>;

    /// Allocates a MASSIVE amount of space, but should not cause an out of memory error. The method by which this achieved is system dependent
    fn allocate_massive(&self, size: usize) -> Result<Segment, AllocationError>;

    /// De-allocates a segment. Depending on the platform, this may not do anything.
    ///
    /// # Safety
    /// This function is marked unsafe because arbitrary segments can be created. Only
    /// segments created by [allocate()](trait.SegAllocator.html#tymethod.allocate) are guaranteed to not fail
    unsafe fn deallocate(&self, segment: Segment) -> bool;
}

#[cfg(windows)]
impl SegAllocator for SegmentAllocator {
    fn allocate(&self, size: usize) -> Result<Segment, AllocationError> {
        while LOCK.compare_and_swap(false, true, Ordering::Acquire) {}
        unsafe {
            let heap: HANDLE = GetProcessHeap();
            if heap.is_null() {
                return Err(AllocationError::NoHeap);
            }

            let alloc = HeapAlloc(heap, HEAP_ZERO_MEMORY, size);
            LOCK.store(false, Ordering::Release);
            #[cfg(debug_assertions)]
            #[allow(non_snake_case)]
            if !alloc.is_null() {
                let mut heap_summary: HEAP_SUMMARY = HEAP_SUMMARY {
                    cb: 0,
                    cbAllocated: 0,
                    cbCommitted: 0,
                    cbReserved: 0,
                    cbMaxReserve: 0,
                };
                match HeapSummary(heap, 0, &mut heap_summary as LPHEAP_SUMMARY) {
                    S_OK => {
                        let HEAP_SUMMARY {
                            cb: _,
                            cbAllocated,
                            cbCommitted,
                            cbReserved,
                            cbMaxReserve,
                        } = heap_summary;
                        // println!("HEAP SUMMARY");
                        // println!("\tAllocated: {:?}", cbAllocated);
                        // println!("\tCommitted: {:?}", cbCommitted);
                        // println!("\tReserved: {:?}", cbReserved);
                        // println!("\tMax Reserve: {:?}", cbMaxReserve);
                    }
                    _ => panic!("Unable to get the heap summary"),
                }
            }
            if alloc.is_null() {
                Err(AllocationError::AllocationFailed(size, errno::errno()))
            } else {
                let seg = Segment::new(alloc, heap, size);
                Ok(seg)
            }
        }
    }

    fn allocate_massive(&self, size: usize) -> Result<Segment, AllocationError> {

        unsafe {
            let alloc = VirtualAlloc(null_mut(), size, MEM_RESERVE, PAGE_READWRITE);

            /*
            let heap: HANDLE = HeapCreate(0, 0, 0);
            if heap.is_null() {
                return Err(AllocationError::HeapNotCreated);
            }

            let alloc = HeapAlloc(heap, 0, size);

             */
            if alloc.is_null() {
                Err(AllocationError::AllocationFailed(size, errno::errno()))
            } else {
                let seg = Segment::new(alloc, alloc, size);
                Ok(seg)
            }
        }
    }

    unsafe fn deallocate(&self, segment: Segment) -> bool {
            let heap: HANDLE = segment.heap;
            let ret = if heap != GetProcessHeap() {
                VirtualFree(heap as LPVOID, segment.length, MEM_RELEASE) != 0
            } else {
                HeapFree(heap, 0, segment.ptr) != 0
            }
    }
}

#[cfg(unix)]
impl SegAllocator for SegmentAllocator {
    fn allocate(&self, size: usize) -> Result<Segment, AllocationError> {
        while LOCK.compare_and_swap(false, true, Ordering::Acquire) {}
        let mmap: *mut c_void = unsafe {
            libc::mmap(
                null_mut(),
                size,
                libc::PROT_WRITE | libc::PROT_READ,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        LOCK.store(false, Ordering::Release);
        if mmap as usize == std::usize::MAX {
            Err(AllocationFailed(size, errno::errno()))
        } else {
            Ok(Segment::new(mmap, size))
        }
    }

    fn allocate_massive(&self, size: usize) -> Result<Segment, AllocationError> {
        while LOCK.compare_and_swap(false, true, Ordering::Acquire) {}
        let mmap: *mut c_void = unsafe {
            libc::mmap(
                null_mut(),
                size,
                libc::PROT_WRITE | libc::PROT_READ,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        LOCK.store(false, Ordering::Release);
        if mmap == libc::MAP_FAILED {
            Err(AllocationFailed(size, errno::errno()))
        } else {
            Ok(Segment::new(mmap, size))
        }
    }


    unsafe fn deallocate(&self, segment: Segment) -> bool {
        // while LOCK.compare_and_swap(false, true, Ordering::Acquire) { }
        libc::munmap(segment.ptr, segment.length) == 0
        // LOCK.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod test {
    use crate::mem_info::PAGE;
    use crate::page_map::PM_SZ;
    use crate::pages::external_mem_reservation::{SegAllocator, SEGMENT_ALLOCATOR};

    #[test]
    pub fn get_segment() {
        SEGMENT_ALLOCATOR
            .allocate(PAGE)
            .expect("Test must fail is this fails");
    }

    #[test]
    pub fn free_segment() {
        let segment = SEGMENT_ALLOCATOR
            .allocate(PAGE)
            .expect("Test must fail is this fails");
        assert!(unsafe { SEGMENT_ALLOCATOR.deallocate(segment) });
    }

    #[test]
    pub fn write_to_segment() {
        unsafe {
            let segment = SEGMENT_ALLOCATOR
                .allocate(PAGE)
                .expect("Test must fail is this fails");
            let ptr1 = segment.get_ptr() as *mut usize;
            (*ptr1) = 0xdeadbeaf;

            let segment = SEGMENT_ALLOCATOR
                .allocate(PAGE)
                .expect("Test must fail is this fails");
            let ptr2 = segment.get_ptr() as *mut usize;
            (*ptr2) = 0xdeadbeaf;

            assert_eq!(*ptr1, *ptr2)
        }
    }

    #[test]
    pub fn allocate_page_table_size() {
        let size = PM_SZ;
        let seg = SEGMENT_ALLOCATOR
            .allocate_massive(size as usize)
            .expect("Must be able to create a massive page for allocator to function");
        unsafe{
            SEGMENT_ALLOCATOR.deallocate(seg);
        }
    }
}
