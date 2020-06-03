use crate::allocation_data::Descriptor;
use crate::mem_info::{LG_PAGE, MAX_SZ_IDX, PAGE, PAGE_MASK};
use bitfield::size_of;

use crossbeam::atomic::AtomicCell;
use std::ptr::slice_from_raw_parts_mut;

use crate::pages::page_alloc_over_commit;
use std::mem::MaybeUninit;
use atomic::Atomic;
use atomic::Ordering;
#[cfg(windows)] use winapi::shared::minwindef::LPVOID;
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::winnt::{MEM_COMMIT, PAGE_READWRITE};
use winapi::ctypes::c_void;

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

#[derive(Clone, Copy)]
pub struct PageInfo {
    desc: Option<*mut Descriptor>,
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

        if ptr as usize & SC_MASK as usize != 0 || sc_idx >= MAX_SZ_IDX {
            self.desc = None;
            return;
        }

        let desc = (ptr as usize | sc_idx) as *mut Descriptor;
        self.desc = Some(desc);
    }

    pub fn set_ptr(&mut self, desc: *mut Descriptor, sc_idx: usize) {
        let ptr = desc;

        if ptr as usize & SC_MASK as usize != 0 || sc_idx >= MAX_SZ_IDX {
            self.desc = None;
            return;
        }

        let desc = (ptr as usize | sc_idx) as *mut Descriptor;
        self.desc = Some(desc);
    }

    pub fn get_desc(&self) -> Option<*mut Descriptor> {
        match &self.desc {
            None => None,
            Some(ptr) => {
                let desc = *ptr as u64 | !SC_MASK;
                Some(desc as *mut Descriptor)
            }
        }
    }

    pub fn get_size_class_index(&self) -> Option<usize> {
        match &self.desc {
            None => None,
            Some(desc) => Some(*desc as usize & !SC_MASK as usize),
        }
    }
}

pub const PM_SZ: u64 = (1u64 << PM_SB as u64)  * size_of::<PageInfo>() as u64;

pub struct PageMap<'a> {
    mem_location: Option<*mut u8>,
    page_map: &'a [Atomic<PageInfo>],
}

impl PageMap<'_> {
    pub fn init(&mut self) {
        println!("PM_NLS = {:?}", PM_NLS);
        println!("PM_NHS = {:?}", PM_NHS);
        println!("PM_SB = {:?}", PM_SB);
        println!("PageInfo size = {:?}", size_of::<PageInfo>());
        println!("PM_SZ = {:?}", PM_SZ);
        let map = page_alloc_over_commit(PM_SZ as usize);
        match map {
            Ok(map) => {
                let ptr = map as *mut MaybeUninit<Atomic<PageInfo>>;
                let slice_before =
                    unsafe {
                        let length = (1u64 << PM_SB as u64);
                        &mut *slice_from_raw_parts_mut(ptr, length as usize)
                    };
                for place in slice_before.into_iter() {
                    if cfg!(windows) {
                        unsafe {
                            self.unsafe_set_page_info(ptr, place as * mut MaybeUninit <Atomic<PageInfo>>, PageInfo::default());
                        }
                    }
                    // *place = MaybeUninit::new(Atomic::new(PageInfo::default()));
                }
                let slice =
                    unsafe {
                        &mut *slice_from_raw_parts_mut(ptr as *mut Atomic<PageInfo>, (1u64 << PM_SB as u64) as usize)
                    };

                self.page_map = slice;

                self.mem_location = Some(map);
            }
            Err(e) => panic!("Error creating memory map: {:?}", e),
        }
    }

    unsafe fn unsafe_set_page_info(&self, base_ptr : *mut MaybeUninit<Atomic<PageInfo>>, ptr: *mut MaybeUninit<Atomic<PageInfo>>, info:PageInfo) {
        let key = self.unsafe_addr_to_key(base_ptr, ptr);
        if cfg!(windows) {
            let x = unsafe {  self.commit_page(base_ptr as *mut u8, key) };
            #[cfg(debug_assertions)]
            println!("Page allocating to: {:?}, Pointer: {:?}", x, ptr);
        }
        *ptr = MaybeUninit::new(Atomic::new(info));

        //ptr.store(info, Ordering::Release);
    }

    #[inline]
    fn unsafe_addr_to_key<T>(&self, base_ptr: *const MaybeUninit<Atomic<PageInfo>>, ptr: *const T) -> usize {
        println!("ptr: {:x?}", ptr);
        let i = (ptr as usize >> PM_KEY_SHIFT);
        println!("i: {:x?}", i);
        println!("KEY_MASK: {:x?}", PM_KEY_MASK);
        let key = (i - (base_ptr as usize >> PM_KEY_SHIFT)) & PM_KEY_MASK as usize;
        println!("key: {:?}", key);
        key
    }

    #[inline]
    pub fn get_page_info<T>(&self, ptr: *const T) -> PageInfo {
        let key = self.addr_to_key(ptr);
        if cfg!(windows) {
            unsafe { self.commit_page(self.mem_location.unwrap() as *mut u8, key) };
        }
        let ptr = &self.page_map[key];
        ptr.load(Ordering::Acquire)
    }

    #[inline]
    pub fn set_page_info<T>(&self, ptr: *const T, info: PageInfo) {
        let key = self.addr_to_key(ptr);
        if cfg!(windows) {
            unsafe { self.commit_page(self.mem_location.unwrap() as *mut u8, key) };
        }
        let ptr = &self.page_map[key];
        ptr.store(info, Ordering::Release);
    }

    #[inline]
    fn addr_to_key<T>(&self, ptr: *const T) -> usize {
        println!("ptr: {:x?}", ptr);
        let i = (ptr as usize >> PM_KEY_SHIFT);
        println!("i: {:x?}", i);
        println!("KEY_MASK: {:x?}", PM_KEY_MASK);
        let key = (i - (self.mem_location.unwrap() as usize >> PM_KEY_SHIFT)) & PM_KEY_MASK as usize;
        println!("key: {:?}", key);
        key
    }

    #[cfg(windows)]
    unsafe fn get_page(&self, start_location: *mut u8, key: usize) -> LPVOID {
        /*
        let offset: isize = (key * size_of::<Atomic<PageInfo>>()) as isize;
        let offset_ptr = start_location.offset(offset) as usize;
        let mask = (PM_KEY_MASK as usize) << PM_KEY_SHIFT;
        let masked = offset_ptr & mask;
        println!("PAGE_MASK: {:x?}", mask);
        let page = masked as LPVOID;

         */
        let page = start_location.offset((key * PAGE) as isize) as LPVOID;
        page
    }

    #[cfg(windows)]
    unsafe fn commit_page(&self, start_location: *mut u8, key: usize) -> *mut c_void {
        let page = self.get_page(start_location, key);
        let alloc = VirtualAlloc(page, PAGE, MEM_COMMIT, PAGE_READWRITE);
        if alloc.is_null() {
            panic!("Couldn't commit page")
        }
        alloc
    }
}

pub static mut S_PAGE_MAP: PageMap = PageMap {
    mem_location: None,
    page_map: &[],
};
