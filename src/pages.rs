use std::fmt;
use std::hash::{Hash};
use std::os::raw::c_void;


use bitfield::fmt::{Debug, Display, Formatter};
use spin::Mutex;
#[cfg(windows)] use winapi::um::heapapi::GetProcessHeap;

use crate::independent_collections::HashMap;
use crate::mem_info::PAGE_MASK;
pub use crate::pages::external_mem_reservation::*;

mod external_mem_reservation;


#[inline]
#[allow(unused)]
pub fn page_addr2base<T>(a: &T) -> *mut c_void {
    (a as *const T as usize & !PAGE_MASK) as *mut c_void
}

#[derive(Debug)]
pub struct PageMaskError;

impl std::error::Error for PageMaskError {}

impl Display for PageMaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct PtrHolder(*const u8);

unsafe impl Sync for PtrHolder {}
unsafe impl Send for PtrHolder {}

struct SegmentHolder {
    size_map: Option<HashMap<PtrHolder, Segment>>,
}

static SEGMENT_HOLDER: Mutex<SegmentHolder> = Mutex::new(SegmentHolder { size_map: None });

/// Returns a set of continuous pages, totaling to size bytes
pub fn page_alloc(size: usize) -> Result<*mut u8, AllocationError> {
    if size & PAGE_MASK != 0 {
        return Err(AllocationError::AllocationFailed(size, errno::errno()));
    }
    /*
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

     */

    let mut segment_holder = SEGMENT_HOLDER.lock();
    let is_none = segment_holder.size_map.is_none();
    if is_none {
        segment_holder.size_map = Some(HashMap::new());
    }
    let segment = SEGMENT_ALLOCATOR.allocate(size)?;
    let ptr = segment.get_ptr() as *mut u8;
    segment_holder
        .size_map
        .as_mut()
        .unwrap()
        .insert(PtrHolder(ptr), segment);
    Ok(ptr)
}

/// Explicitly allow overcommitting
///
/// Used for array-based page map
pub fn page_alloc_over_commit(size: usize) -> Result<*mut u8, AllocationError> {
    if size & PAGE_MASK != 0 {
        return Err(AllocationError::AllocationFailed(size, errno::errno()));
    }
    /*
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

     */
    let mut segment_holder = SEGMENT_HOLDER.lock();
    let is_none = segment_holder.size_map.is_none();
    if is_none {
        segment_holder.size_map = Some(HashMap::new());
    }
    let segment = SEGMENT_ALLOCATOR.allocate_massive(size)?;
    let ptr = segment.get_ptr() as *mut u8;
    segment_holder
        .size_map
        .as_mut()
        .unwrap()
        .insert(PtrHolder(ptr), segment);
    Ok(ptr)

    // SEGMENT_ALLOCATOR.allocate_massive(size).map(|ptr| ptr.get_ptr() as *mut u8)
}

/// Altered version of the lralloc free, which uses the drop method
/// of MMapMut struct
pub fn page_free(ptr: *const u8) -> bool {
    // unsafe { PAGE_HOLDER.dealloc(ptr) }
    let mut segment_holder = SEGMENT_HOLDER.lock();
    let holder = PtrHolder(ptr);
    if segment_holder.size_map.as_mut().unwrap().contains(&holder) {
        let segment = segment_holder.size_map.as_mut().unwrap().remove(&holder).unwrap();
        let ret = unsafe {
             SEGMENT_ALLOCATOR.deallocate(segment)
        };

        ret
    } else {
        false
    }
}

#[cfg(test)]
mod test {
    use crate::{init_malloc, MALLOC_INIT_S};
    use crate::pages::{page_alloc, page_free};
    use crate::size_classes::{SIZE_CLASSES, SizeClassData};

    #[test]
    fn get_page() {
        let ptr = page_alloc(4096).expect("Couldn't get page");
        assert!(!ptr.is_null());
        /*
        unsafe {
            assert!(PAGE_HOLDER.capacity > 0);
            assert!(PAGE_HOLDER.count >= 1)
        }

         */
    }

    #[test]
    fn deallocate() {
        let ptr = page_alloc(4096).expect("Couldn't get page");
        assert!(!ptr.is_null());
        assert!(page_free(ptr));
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
        MALLOC_INIT_S.with(|| unsafe { init_malloc() });
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
                }
                page_free(ptr as *mut u8);
            }
        }
    }
}
