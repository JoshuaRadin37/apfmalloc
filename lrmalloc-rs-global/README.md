# LRMalloc-rs Global Allocator
This package automatically makes the lrmalloc-rs package the allocator for a program


When built produces 3 different library files:
1. A dynamic library (system dependent)
2. A static library (liblrmalloc_rs_global.a)
3. A rust library (liblrmalloc_rs_global.rlib)

The rust library output allows the user to expose the lrmalloc-rs API and structs, while
including the static or dynamic library keeps them hidden.

## What does the library file do?
When the library file is linked with a program or the package is included in a crate, it
overrides the default allocator. Instead of using the system's allocator, the program will
use lrmalloc-rs. A global allocator does not need to be set by the user in the case of rust.
This is done by having the 4 following C FFI functions:
```rust
#[no_mangle]
extern "C" fn malloc(size: usize) -> *mut c_void { }

#[no_mangle]
extern "C" fn calloc(num: usize, size: usize) -> *mut c_void { }

#[no_mangle]
extern "C" fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void { }

#[no_mangle]
extern "C" fn free(ptr: *mut c_void) { }

#[no_mangle]
extern "C" fn aligned_alloc(alignment: usize, size: usize) -> *mut c_void { }
```
These translate to the follow C functions:
```c
void* malloc(size_t size);
void* calloc(size_t num, size_t size);
void* realloc(void* ptr, size_t new_size);
void free(void* ptr);
void* aligned_alloc(size_t alignment, size_t size);
```

To link in rust, you 

## The header file: lrmalloc_rs.h
Optionally, a header file named lrmalloc_rs.h can be included. This is unnecessary to include
in the project if the library, but does expose one additional function, which is also available in the
rust library. The function `check_override()` will run a few very quick tests to see if the
correct implementation of `malloc`, `calloc`, etc. are being used.

