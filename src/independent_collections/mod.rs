//! Provides an several collection types similar to those in the standard library
//! However, all allocation is done without using any allocator, and
//! instead uses direct memory allocation from the OS.

mod array;
pub use array::*;

mod hash_map;
pub use hash_map::*;
