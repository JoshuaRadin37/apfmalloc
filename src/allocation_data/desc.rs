use crate::mem_info::{CACHE_LINE, CACHE_LINE_MASK};
use std::sync::atomic::AtomicPtr;
use super::Anchor;
use crate::allocation_data::proc_heap::ProcHeap;

#[repr(packed)]
pub struct DescriptorNode {
    desc: Option<&'static mut Descriptor>
}

impl DescriptorNode {

    pub fn set(&mut self, desc: &'static Descriptor, count: u64) {
        let usize_desc = desc as *const Descriptor as u64;
        assert_eq!(usize_desc & CACHE_LINE_MASK as u64, 0);
        unsafe {
            let pointer = (usize_desc | (count & CACHE_LINE_MASK as u64)) as *mut Descriptor;
            let reference = &mut *pointer;
            self.desc = Some(reference);
        }

    }

    pub fn get_desc(&self) -> Option<&'static mut Descriptor> {
        match self.desc.as_deref() {
            None => None,
            // This seems disgusting
            Some(desc) => {
                let usize_desc = desc as *const Descriptor as u64;
                let fixed_ptr = usize_desc & !CACHE_LINE_MASK as u64;
                unsafe {
                    Some(&mut *(fixed_ptr as *mut Descriptor))
                }
            }
        }
    }

    pub fn get_counter(&self) -> Option<u64> {
        match self.desc.as_deref() {
            None => None,
            // This seems disgusting
            Some(desc) => {
                let usize_desc = desc as *const Descriptor as u64;
                Some(usize_desc & CACHE_LINE_MASK as u64)
            }
        }
    }
}


/// This is a super block descriptor
/// needs to be cache aligned
/// Descriptors are never freed, and should be statically allocated
#[repr(align(64))]
pub struct Descriptor {
    pub next_free: AtomicPtr<DescriptorNode>,
    pub next_partial: AtomicPtr<DescriptorNode>,
    pub anchor: Anchor,
    pub super_block: *mut u8,
    pub proc_heap: *mut ProcHeap,
    pub block_size: u32,
    pub max_count: u32
}

impl Descriptor {
    pub const fn new(next_free: AtomicPtr<DescriptorNode>, next_partial: AtomicPtr<DescriptorNode>, anchor: Anchor, super_block: *mut u8, proc_heap: *mut ProcHeap, block_size: u32, max_count: u32) -> Self {
        Descriptor { next_free, next_partial, anchor, super_block, proc_heap, block_size, max_count }
    }
}



