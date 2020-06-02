use crate::mem_info::{CACHE_LINE, CACHE_LINE_MASK};
use std::sync::atomic::AtomicPtr;
use super::Anchor;
use crate::allocation_data::proc_heap::ProcHeap;
use crossbeam::atomic::AtomicCell;
use atomic::{Atomic, Ordering};
use crate::AVAILABLE_DESC;
use crate::pages::page_alloc;
use lazy_static::lazy_static;

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

impl Default for DescriptorNode {
    fn default() -> Self {
        Self {
            desc: None
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
    pub anchor: Atomic<Anchor>,
    pub super_block: *mut u8,
    pub proc_heap: *mut ProcHeap,
    pub block_size: u32,
    pub max_count: u32
}

/*
/// The intitial descriptor holder
struct DescriptorHolder {}


lazy_static! {
static ref DESCRIPTORS_SPACE

}

 */

impl Descriptor {
    pub fn new(next_free: AtomicPtr<DescriptorNode>, next_partial: AtomicPtr<DescriptorNode>, anchor: Anchor, super_block: *mut u8, proc_heap: *mut ProcHeap, block_size: u32, max_count: u32) -> Self {
        Descriptor { next_free, next_partial, anchor: Atomic::new(anchor), super_block, proc_heap, block_size, max_count }
    }

    pub fn retire(&'static mut self) {
        self.block_size = 0;
        let old_head = unsafe {AVAILABLE_DESC.load(Ordering::Acquire) };
        let mut new_head: DescriptorNode = DescriptorNode::default();
        loop {
            self.next_free.store(old_head, Ordering::Release);

            new_head.set(self, unsafe {(*old_head).get_counter() }.expect("Counter Should exist") + 1);
            if unsafe {AVAILABLE_DESC.compare_exchange_weak(
                old_head,
                &mut new_head,
                Ordering::Acquire,
                Ordering::Release
            ).is_ok() } {
                break;
            }
        }
    }

    pub unsafe fn alloc() -> * mut Descriptor {
        let old_head = AVAILABLE_DESC.load(Ordering::Acquire).as_mut().unwrap();
        loop {
            let desc = old_head.get_desc();
            match desc {
                Some(desc) => {
                    let mut new_head = desc.next_free.load(Ordering::Acquire).as_mut().unwrap();
                    new_head.set(new_head.get_desc().unwrap(), old_head.get_counter().unwrap());
                    if AVAILABLE_DESC.compare_exchange_weak(old_head, new_head, Ordering::Acquire, Ordering::Release).is_ok() {
                        return desc as *mut Descriptor;
                    }
                },
                None => {
                    // let ptr = page_alloc(DE)
                }
            }
        }
    }

}

pub fn desc_retire(desc: &mut Descriptor) {

}



