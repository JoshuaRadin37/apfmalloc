extern crate libc;

pub trait Allocator {
    // fn allocate(&self, size: usize)
}

/*

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

*/

#[cfg(test)]
mod test {}
