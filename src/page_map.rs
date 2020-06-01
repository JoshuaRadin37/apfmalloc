use crate::mem_info::{LG_PAGE, MAX_SZ_IDX};
use crate::allocation_data::Descriptor;
use bitfield::size_of;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr::{null_mut, slice_from_raw_parts_mut};
use crossbeam::atomic::AtomicCell;
use memmap::MmapMut;
use crate::pages::page_alloc_over_commit;

/// Assuming x84-64, which has 48 bits for addressing
/// TODO: Modify based on arch
pub const PM_NHS: usize = 14;
/// Insignificant low bits
pub const PM_NLS: usize = LG_PAGE;
/// Middle bits
pub const PM_SB: usize = 64 - PM_NHS - PM_NLS;

/// To get the key from a address
/// 1. Shift to remove insignificant low bits
/// 2. Apply mask of middle significant bits
pub const PM_KEY_SHIFT: usize = PM_NLS;
pub const PM_KEY_MASK: u64 = (1u64 << PM_SB as u64) - 1;

/// Associates metadata to each allocator page
/// implemented with a static array
pub const SC_MASK: u64 = (1u64 << 6) - 1;

#[derive(Clone)]
pub struct PageInfo {
    desc: Option<* mut Descriptor>
}

unsafe impl Send for PageInfo {}
unsafe impl Sync for PageInfo {}


impl Default for PageInfo {
    fn default() -> Self {
        Self { desc: None }
    }
}

impl PageInfo {

    pub fn set(&mut self, desc: &mut Descriptor, sc_idx: usize) {

        let ptr = desc as *mut Descriptor;

        if ptr as usize & SC_MASK as usize != 0 ||
            sc_idx >= MAX_SZ_IDX {
            self.desc = None;
            return;
        }

        let desc =
            (ptr as usize | sc_idx) as *mut Descriptor;
        self.desc = Some(desc);
    }

    pub fn set_ptr(&mut self, desc: *mut Descriptor, sc_idx: usize) {

        let ptr = desc;

        if ptr as usize & SC_MASK as usize != 0 ||
            sc_idx >= MAX_SZ_IDX {
            self.desc = None;
            return;
        }

        let desc =
            (ptr as usize | sc_idx) as *mut Descriptor;
        self.desc = Some(desc);
    }

    pub fn get_desc(&self) -> Option<*mut Descriptor> {
        match &self.desc {
            None => { None },
            Some(ptr) => {
                let desc = *ptr as u64 | !SC_MASK;
                Some(desc as *mut Descriptor)
            },
        }
    }

    pub fn get_size_class_index(&self) -> Option<usize> {
        match &self.desc {
            None => None,
            Some(desc) => {
                Some(*desc as usize & !SC_MASK as usize)
            }
        }

    }
}

pub const PM_SZ: u64 = (1u64 << PM_SB as u64) * size_of::<PageInfo>() as u64;

pub struct PageMap<'a> {
    map: Option<MmapMut>,
    page_map: &'a [AtomicCell<PageInfo>]
}

impl PageMap<'_> {

    pub fn init(&mut self) {
        let map = page_alloc_over_commit(PM_SZ as usize);
        match map {
            Ok(mut map) => {
                let ptr = map.as_mut_ptr() as * mut AtomicCell<PageInfo>;
                let slice = unsafe {
                    &mut *slice_from_raw_parts_mut(ptr, (1u64 << PM_SB as u64) as usize)
                };
                self.page_map = slice;

                self.map = Some(map);



            },
            Err(_) => {
                panic!("Error creating memory map")
            },
        }
    }

    #[inline]
    pub fn get_page_info<T>(&self, ptr: * const T) -> &PageInfo {
        let key = self.addr_to_key(ptr);
        let ptr = &self.page_map[key];
        unsafe {& *ptr.as_ptr()}
    }

    #[inline]
    pub fn set_page_info<T>(&self, ptr: * const T, info: PageInfo) {
        let key = self.addr_to_key(ptr);
        let ptr = &self.page_map[key];
        ptr.store(info);
    }

    #[inline]
    fn addr_to_key<T>(&self, ptr: * const T) -> usize {
        let key = (ptr as usize >> PM_KEY_SHIFT) & PM_KEY_MASK as usize;
        key
    }


}

pub static mut S_PAGE_MAP: PageMap = PageMap {
    map: None,
    page_map: &[]
};



