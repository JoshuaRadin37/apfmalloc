use crate::allocation_data::DescriptorNode;
use crate::mem_info::MAX_SZ_IDX;
use crate::size_classes::{SizeClassData, SIZE_CLASSES};
use std::ptr::slice_from_raw_parts_mut;


use crate::single_access::SingleAccess;
use atomic::Atomic;
use bitfield::size_of;
use memmap::MmapMut;
use std::mem::MaybeUninit;

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
pub struct Heaps(Option<MmapMut>);

impl Heaps {
    const fn uninit() -> Self {
        Heaps(None)
    }

    fn as_heaps_mut(&mut self) -> &mut [ProcHeap] {
        unsafe {
            let map = &mut self.0.as_mut().unwrap()[0];
            let ptr: *mut ProcHeap = (map as *mut u8) as *mut ProcHeap;
            std::slice::from_raw_parts_mut(ptr, MAX_SZ_IDX)
        }
    }

    #[allow(unused)]
    fn as_heaps(&self) -> &[ProcHeap] {
        unsafe {
            let map = &self.0.as_ref().unwrap()[0];
            let ptr = map as *const u8 as *const ProcHeap;
            std::slice::from_raw_parts(ptr, MAX_SZ_IDX)
        }
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
    let mut map = MmapMut::map_anon(size_of::<ProcHeap>() * MAX_SZ_IDX)
        .expect("Should be able to get the map");
    let ptr = map.as_mut_ptr() as *mut MaybeUninit<ProcHeap>;
    let slice = &mut *slice_from_raw_parts_mut(ptr, MAX_SZ_IDX);

    for (index, proc) in slice.into_iter().enumerate() {
        *proc = MaybeUninit::new(ProcHeap::new_none(index))
    }
    HEAPS = Heaps(Some(map))
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
