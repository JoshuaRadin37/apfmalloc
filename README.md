# apfmalloc
##### By Joshua Radin and Elias Neuman-Donihue

An implementation of `malloc(3)` written in Rust. This implementation uses Allocations Per Fetch (APF) Tuning to
determine the amount of blocks to allocate into thread caches.

Based on the lrmalloc allocator, found here in this [github repo](https://github.com/ricleite/lrmalloc)

This repo has two significant parts:
- `apfmalloc-lib`
- `apfmalloc`

The design of the allocator itself is entirely contained within `apfmalloc-lib`,
and the `apfmalloc` crate is just a combination of C bindings and a Rust Global Allocator (which can be disabled).

The main outward facing functions that can be used from this library are:
- Manual Size (and Align)
    - `do_malloc(size: usize)` allocates at minimum `size` amount of bytes
    - `do_aligned_alloc(align: usize, size: usize)` allocates at minimum `size` amount of bytes,
    with the additional requirement that the memory returned by this function is a integral multiple of the
    `align` parameter
    - `do_realloc(ptr: *mut c_void, size: usize)` takes a previously allocated pointer from the heap, and if the new `size` is 
    greater than the previous size, will move the data stored at `ptr` to a new location. If this move occurs, the old
    `ptr` is then invalid
    - `do_free<T>(ptr: *const T)` releases the memory allocated at the `ptr`, allowing it to be used
    again by other allocations. The `ptr` becomes invalid after this function.
- Automatic Size and Align
    - `allocate_type<T>()` uses it's type parameter to pass arguments to 
    `do_aligned_alloc`. The data pointed to by the result of this function is _uninitialized_.
    - `allocate_val<T>(val: T)` uses `allocate_type<T>`, then if memory is successfully allocated, initializes the pointer to
    `val`

Notes
1. All functions besides `do_free`, which has no return value, will return a null pointer if
they fail.
2. Any double free will cause errors that can not be caught.
3. Using `do_realloc` with a null pointer as an input is equivalent to calling `do_malloc`


## Memory Movement Overflow

## Useful Included Types
### Auto Pointer
The type `AutoPtr<T>` is automatically managed pointer allocated from the heap using `do_aligned_alloc`, and allows access to the data stored in it.

`AutoPtr<T>` does not implement `Copy`, and can only have one owner at a time. Once an `AutoPtr` goes out of scope, it
will deallocate the space used by it.

Example:
```rust
fn main() {
    let mut p = AutoPtr::new(16usize);
    *p = 42;
    println!("{} + 53 = {}", p, *p + 53)
} // p is dropped here, and the memory is free'd
```

### Independent Collections

Any allocation that goes through `do_malloc`, `do_realloc`, etc. will be tracked by the APF Tuner.
As such, its useful to have types that are designed to skip these steps.

#### `RawArray<T>`

The `RawArray<T>` is a block of memory with a certain capacity. The `RawArray<T>` type automatically handles allocation of the memory segment by settings it's capacity. The array can be expanded later on, and might move the allocated memory. Accessing elements in a `RawArray<T>` is unsafe, as it doesn't manage whether there is an element initializde at an index, or whether an index is out of bounds. Taking pointers from a raw array and attempting to use them later can be unsafe, as re-allocation could move memory and make the last location invalid. 

Upon dropping a `RawArray<T>`, none of it's elements will be dropped.

#### `Array<T>`

The `Array<T>` type is a managed array, very similar to the standard library `Vec<T>` type. The array does not allow for access of uninitialized elements. It will expand the backing memory when needed. Since the `Array<T>` type tracks the number of elements in the array, more complex methods and traits are implemented on it. `Array<T>`s can be convertered into iterators using the `into_iter()` method. This allows for a function to take ownership of the elements of an `Array<T>`.

Upon dropping an `Array<T>`, the elements of the array are dropped as well. As such, any type that is backed by `Array<T>`s will always deallocate any heap allocated memory, and not have any memory leaks.

#### `HashMap<K, V>`

This is a hash map implementation that is backed by `Array`s. 

