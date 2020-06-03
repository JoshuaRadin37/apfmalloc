extern crate libc;

use std::ffi::c_void;

pub trait Allocator {

    fn allocate(&self, size: usize)

}



pub fn malloc(size: usize) -> * mut c_void {
    unsafe {
        _libc_malloc(size)
    }
}

pub fn malloc_type<T>() -> * mut T {
    malloc(std::mem::size_of::<T>()) as *mut T
}

pub fn free<T>(ptr: * mut T){
    unsafe {
        _libc_free(ptr as *mut c_void)
    }
}



#[cfg(test)]
mod test {
    use bitfield::size_of;
    use std::ffi::c_void;
    use crate::pages::external_mem_reservation::{malloc_type, free};

    #[test]
    fn extern_functions() {
        unsafe {
            let allocated = malloc_type::<usize>(); //unsafe { &mut *(__malloc(size_of::<usize>()) as * mut usize) };
            *allocated = 0xdeadbeaf;
            unsafe {
                free(allocated)
            }
        }
    }
}