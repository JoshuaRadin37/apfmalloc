use crate::allocation_data::DescriptorNode;
use crate::mem_info::MAX_SZ_IDX;
use crate::size_classes::{SizeClassData, SIZE_CLASSES};

use crate::single_access::SingleAccess;
use atomic::Atomic;
use crate::independent_collections::Array;

#[repr(align(64))]
pub struct ProcHeap {
    pub partial_list: Atomic<Option<DescriptorNode>>,
    pub size_class_index: usize,
}

impl ProcHeap {
    pub fn new(partial_list: DescriptorNode, size_class_index: usize) -> Self {
        let ptr = Atomic::new(Some(partial_list));
        ProcHeap {
            partial_list: ptr,
            size_class_index,
        }
    }

    pub fn new_none(size_class_index: usize) -> Self {
        let ptr = Atomic::new(None);
        ProcHeap {
            partial_list: ptr,
            size_class_index,
        }
    }

    pub fn get_size_class_index(&self) -> usize {
        self.size_class_index
    }

    pub fn get_size_class(&self) -> &mut SizeClassData {
        unsafe { &mut SIZE_CLASSES[self.size_class_index] }
    }

    pub fn default() -> Self {
        Self {
            partial_list: Atomic::new(None),
            size_class_index: 0,
        }
    }
}

unsafe impl Sync for ProcHeap {}

unsafe impl Send for ProcHeap {}

impl Default for ProcHeap {
    fn default() -> Self {
        ProcHeap::default()
    }
}

#[repr(transparent)]
pub struct Heaps(Array<ProcHeap>);

impl Heaps {
    const fn uninit() -> Self {
        Heaps(Array::new())
    }

    fn as_heaps_mut(&mut self) -> &mut [ProcHeap] {
        &mut *self.0
    }

    #[allow(unused)]
    fn as_heaps(&self) -> &[ProcHeap] {
        & *self.0
    }

    #[allow(unused)]
    pub fn get_heap_at(&self, index: usize) -> &ProcHeap {
        &self.as_heaps()[index]
        // self.0[index].borrow()
    }

    pub fn get_heap_at_mut(&mut self, index: usize) -> &mut ProcHeap {
        &mut self.as_heaps_mut()[index]
        //self.0[index].borrow_mut()
    }
}

static mut HEAPS: Heaps = Heaps::uninit();

unsafe fn init_heaps() {
    HEAPS = Heaps(Array::of_size(MAX_SZ_IDX));
}

pub fn get_heaps() -> &'static mut Heaps {
    static HEAPS_INIT_S: SingleAccess = SingleAccess::new();

    unsafe {
        /*
        if !HEAP_INIT.compare_and_swap(false, true, Ordering::Acquire) {
            init_heaps();
            //HEAP_INIT.store(true, Ordering::Release)
        }

         */
        HEAPS_INIT_S.with(|| init_heaps());

        &mut HEAPS
    }
}
