use std::sync::atomic::{AtomicPtr, Ordering, AtomicBool};
use crate::allocation_data::DescriptorNode;
use crate::size_classes::{SizeClassData, SIZE_CLASSES};
use std::ptr::{null_mut, null};
use crate::mem_info::MAX_SZ_IDX;
use std::borrow::{Borrow, BorrowMut};
use std::ops::{Index, IndexMut};
use std::mem::MaybeUninit;
use crossbeam::atomic::AtomicCell;
use memmap::MmapMut;
use bitfield::size_of;

#[repr(align(64))]
pub struct ProcHeap {
    pub partial_list: AtomicPtr<DescriptorNode>,
    pub size_class_index: usize
}

impl ProcHeap {
    pub const fn new(partial_list: *mut DescriptorNode, size_class_index: usize) -> Self {
        let ptr = AtomicPtr::new(partial_list);
        ProcHeap { partial_list: ptr, size_class_index }
    }

    pub fn get_size_class_index(&self) -> usize {
        self.size_class_index
    }

    pub fn get_size_class(&self) -> &mut SizeClassData {
        unsafe { &mut SIZE_CLASSES[self.size_class_index] }
    }

    pub const fn default() -> Self {
        Self::new(
            null_mut(),
            0
        )
    }
}

unsafe impl Sync for ProcHeap { }

unsafe impl Send for ProcHeap { }

impl Default for ProcHeap {
    fn default() -> Self {
        ProcHeap::default()
    }
}


#[repr(transparent)]
pub struct Heaps(MaybeUninit<MmapMut>);

impl Heaps {
    pub const fn uninit() -> Self {
        Heaps(MaybeUninit::uninit())
    }

    pub fn new(field0: MmapMut) -> Self {
        Heaps(MaybeUninit::new(field0))
    }

    pub fn get_heap_at(&self, index: usize) -> &ProcHeap {
        self.0[index].borrow()
    }

    pub fn get_heap_at_mut(&mut self, index: usize) -> &mut ProcHeap {
        self.0[index].borrow_mut()
    }
}

static mut HEAPS: Heaps = Heaps::uninit();
static mut HEAP_INIT: AtomicBool = AtomicBool::new(false);

unsafe fn init_heaps() {
    let map = MmapMut::map_anon(size_of::<Heaps>()).expect("Should be able to get the map");
}

pub fn get_heaps() -> &'static mut Heaps {
    unsafe {
        if !HEAP_INIT.load(Ordering::Acquire) {
            init_heaps();
            HEAP_INIT.store(true, Ordering::Release)
        }

        &mut *HEAPS
    }
}

