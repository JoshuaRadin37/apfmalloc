//! Provides an several collection types similar to those in the standard library
//! However, all allocation is done without using any allocator, and
//! instead uses direct memory allocation from the OS.

mod array;
pub use array::{ArrayIterator, Array, ArrayDeque, RawArray};

mod linked_list;
pub use linked_list::*;

mod hash_map;
pub use hash_map::{HashMap, HashSet};

mod range_mapping;
pub use range_mapping::*;

pub mod sync {
    pub use super::array::sync_array::*;
    pub use super::hash_map::sync_hash_map::*;
}
