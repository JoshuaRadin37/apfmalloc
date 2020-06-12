use lrmalloc_rs::{do_aligned_alloc, do_free, do_realloc};
use std::os::raw::c_void;


pub static mut OVERRIDE_MALLOC: bool = false;
pub static mut OVERRIDE_CALLOC: bool = false;
pub static mut OVERRIDE_REALLOC: bool = false;
pub static mut OVERRIDE_FREE: bool = false;
pub static mut OVERRIDE_ALIGNED_ALLOC: bool = false;

#[no_mangle]
extern "C" fn malloc(size: usize) -> *mut c_void {

    unsafe {
        OVERRIDE_MALLOC = true;
    }
    do_aligned_alloc(8, size) as *mut c_void
}

#[no_mangle]
extern "C" fn calloc(num: usize, size: usize) -> *mut c_void {

    unsafe {
        OVERRIDE_CALLOC = true;
    }
    let ret = malloc(num * size) as *mut u8;
    unsafe {
        for i in 0..(num * size) {
            *ret.offset(i as isize) = 0;
        }
    }
    ret as *mut c_void
}

#[no_mangle]
extern "C" fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {

    unsafe {
        OVERRIDE_REALLOC = true;
    }
    do_realloc(ptr, new_size)
}

#[no_mangle]
extern "C" fn free(ptr: *mut c_void) {

    unsafe {
        OVERRIDE_FREE = true;
    }
    do_free(ptr)
}

#[no_mangle]
extern "C" fn aligned_alloc(alignment: usize, size: usize) -> *mut c_void {

    unsafe {
        OVERRIDE_ALIGNED_ALLOC = true;
    }
    do_aligned_alloc(alignment, size) as *mut c_void
}

#[no_mangle]
extern "C" fn check_override() -> bool {
    unsafe {
        let ptr = malloc(8);
        if !OVERRIDE_MALLOC {
            return false;
        }
        let new_ptr = realloc(ptr, 64);
        assert_ne!(new_ptr, ptr);
        if !OVERRIDE_REALLOC {
            return false;
        }
        let calloced = calloc(8, 8);
        assert_ne!(new_ptr, calloced);
        if !OVERRIDE_CALLOC {
            return false;
        }
        do_free(new_ptr);
        do_free(calloced);
        if !OVERRIDE_FREE {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn overrides_malloc() {
        unsafe {
            OVERRIDE_MALLOC = false;
            let _ret = libc::malloc(8);
            assert!(OVERRIDE_MALLOC, "Malloc wasn't overwritten!")
        }
    }
    #[test]
    fn overrides_calloc() {
        unsafe {
            OVERRIDE_CALLOC = false;
            let _ret = libc::calloc(1, 8);
            assert!(OVERRIDE_CALLOC, "Calloc wasn't overwritten!")
        }
    }
    #[test]
    fn overrides_realloc() {
        unsafe {
            OVERRIDE_REALLOC = false;
            let first = libc::malloc(8);
            let _ret = libc::realloc(first, 8);
            assert!(OVERRIDE_REALLOC, "Realloc wasn't overwritten!")
        }
    }
    #[test]
    fn overrides_free() {
        unsafe {
            let ret = libc::malloc(8);
            OVERRIDE_FREE = false;
            libc::free(ret);
            assert!(OVERRIDE_FREE, "Free wasn't overwritten!")
        }
    }
}
