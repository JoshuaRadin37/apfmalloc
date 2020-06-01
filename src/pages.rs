use std::os::raw::c_void;
use crate::mem_info::PAGE_MASK;
use memmap::{MmapMut, MmapOptions};
use std::io::{ErrorKind, Error};

#[inline]
pub fn page_addr2base<T>(a : &T) -> * mut c_void {
    unsafe {
        (a as * const T as usize & !PAGE_MASK) as *mut c_void
    }
}


/// Returns a set of continuous pages, totaling to size bytes
pub fn page_alloc(size: usize) -> Result<MmapMut, ()> {
    if size & PAGE_MASK != 0 {
        return Err(())
    }

    MmapOptions::new().len(size).map_anon().map_err(|_| ())
}

/// Explicitly allow overcommitting
///
/// Used for array-based page map
///
/// TODO: Figure out how this is different
pub fn page_alloc_over_commit(size: usize) -> Result<MmapMut, ()> {
    if size & PAGE_MASK != 0 {
        return Err(())
    }

    MmapOptions::new().len(size).map_anon().map_err(|_| ())
}

/// Altered version of the lralloc free, which uses the drop method
/// of MMapMut struct
pub fn page_free(map: MmapMut) {
    let _ = map;
}