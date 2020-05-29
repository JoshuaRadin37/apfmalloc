
use std::sync::atomic::AtomicPtr;
use crate::allocation_data::{DescriptorNode, ProcHeap, get_heaps};
use std::ptr::null_mut;
use crate::mem_info::MAX_SZ_IDX;
use lazy_static::lazy_static;
use crossbeam::atomic::AtomicCell;
use std::cell::{Cell, RefCell};
use crate::size_classes::init_size_class;
use std::process::id;

#[macro_use] pub mod macros;
mod size_classes;
mod mem_info;
mod allocation_data;

#[macro_use]
extern crate bitfield;


static mut AVAILABLE_DESC: AtomicPtr<DescriptorNode> = AtomicPtr::new(null_mut());
static mut MALLOC_INIT: bool = false;



unsafe fn init_malloc() {
    MALLOC_INIT = true;
    init_size_class();

    todo!("sPageMap.init()");

    for idx in 0..MAX_SZ_IDX {
        let heap = get_heaps().get_heap_at_mut(idx);
        heap.size_class_index = idx;

    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::allocation_data::get_heaps;

    #[test]
    fn heaps_valid() {
        let heap = get_heaps();
        let p_heap = heap.get_heap_at_mut(0);

    }
}
