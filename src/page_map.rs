use crate::allocation_data::Descriptor;
use crate::mem_info::{LG_PAGE, MAX_SZ, MAX_SZ_IDX, PAGE};
use bitfield::size_of;

use std::ptr::slice_from_raw_parts_mut;

use crate::pages::page_alloc_over_commit;
use crate::size_classes::get_size_class;
use atomic::Atomic;
use atomic::Ordering;
use std::ptr::null_mut;
#[cfg(windows)]
use winapi::ctypes::c_void;
#[cfg(windows)]
use winapi::shared::minwindef::LPCVOID;
#[cfg(windows)]
use winapi::shared::minwindef::LPVOID;
#[cfg(windows)]
use winapi::um::memoryapi::VirtualAlloc;
#[cfg(windows)]
use winapi::um::memoryapi::VirtualQuery;
#[cfg(windows)]
use winapi::um::winnt::{MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_READWRITE};
#[cfg(windows)]
use winapi::um::winuser::OffsetRect;

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PageInfo {
    desc: *mut Descriptor,
}

unsafe impl Send for PageInfo {}
unsafe impl Sync for PageInfo {}

/*
#[repr(C)]
#[derive(Clone, Copy)]
union _PageInfo {
    _unused: [usize; 2],
    info: PageInfo
}



impl From<&mut _PageInfo> for PageInfo {
    fn from(p: &mut _PageInfo) -> Self {
        unsafe {
            let internals = p._unused;

            if let info = p.info {

            } else {
                p.info = PageInfo::default();
            }

            if let [0, _] = internals {
                p.info = PageInfo::default();
            }

            p.info
        }
    }
}

impl From<&Atomic<_PageInfo>> for PageInfo {
    fn from(a: &Atomic<_PageInfo>) -> Self {
        let mut temp = a.load(Ordering::Acquire);
        let copy = temp.clone();
        let output = PageInfo::from(&mut temp);
        a.compare_exchange(copy, temp, Ordering::Acquire, Ordering::Release);
        output
    }
}

 */

impl Default for PageInfo {
    fn default() -> Self {
        Self { desc: null_mut() }
    }
}
/*
impl From<PageInfo> for _PageInfo {
    fn from(p: PageInfo) -> Self {
        _PageInfo { info: p }
    }
}

 */

impl PageInfo {
    pub fn set(&mut self, desc: &mut Descriptor, sc_idx: usize) {
        let ptr = desc as *mut Descriptor;

        if ptr as usize & SC_MASK as usize != 0 || sc_idx >= MAX_SZ_IDX {
            self.desc = null_mut();
            return;
        }

        let desc = (ptr as usize | sc_idx) as *mut Descriptor;
        self.desc = desc;
    }

    pub fn set_ptr(&mut self, desc: *mut Descriptor, sc_idx: usize) {
        let ptr = desc;

        if ptr as usize & SC_MASK as usize != 0 || sc_idx >= MAX_SZ_IDX {
            self.desc = null_mut();
            return;
        }

        let desc = (ptr as usize | sc_idx) as *mut Descriptor;
        self.desc = desc;
    }

    pub fn get_desc(&self) -> Option<*mut Descriptor> {
        match self.desc {
            ptr if ptr == null_mut() => None,
            ptr => Some(ptr),
        }
    }

    pub fn get_size_class_index(&self) -> Option<usize> {
        match self.desc {
            ptr if ptr == null_mut() => None,
            desc => Some({
                /*
                let x = *desc;
                x as usize & SC_MASK as usize

                 */
                unsafe {
                    let d = &*desc;
                    if d.block_size > MAX_SZ as u32 {
                        return Some(0);
                    }
                    let ret = get_size_class(d.block_size as usize);
                    ret
                }
            }),
        }
    }
}

pub const PM_SZ: u64 = (1u64 << PM_SB as u64) * size_of::<PageInfo>() as u64;

pub struct PageMap<'a> {
    mem_location: Option<*mut u8>,
    page_map: &'a [Atomic<PageInfo>],
}

impl PageMap<'_> {
    pub fn init(&mut self) {
        //println!("PM_NLS = {:?}", PM_NLS);
        // println!("PM_NHS = {:?}", PM_NHS);
        // println!("PM_SB = {:?}", PM_SB);
        // assert_eq!(size_of::<PageInfo>(), size_of::<PageInfo>());
        // println!("PageInfo size = {:?}", size_of::<PageInfo>());
        // println!("PM_SZ = {:?}", PM_SZ);
        let map = page_alloc_over_commit(PM_SZ as usize);
        match map {
            Ok(map) => {
                let ptr = map as *mut Atomic<PageInfo>;
                /*
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

                 */
                let slice = unsafe {
                    &mut *slice_from_raw_parts_mut(
                        ptr as *mut Atomic<PageInfo>,
                        (1u64 << PM_SB as u64) as usize,
                    )
                };

                self.page_map = slice;

                self.mem_location = Some(map);
            }
            Err(e) => panic!("Error creating memory map: {:?}", e),
        }
    }

    /*
    unsafe fn unsafe_set_page_info(&self, base_ptr : *mut MaybeUninit<Atomic<PageInfo>>, ptr: *mut MaybeUninit<Atomic<PageInfo>>, info:PageInfo) {
        let key = self.unsafe_addr_to_key(base_ptr, ptr);
        #[cfg(windows)] {
            let x = unsafe {  self.commit_page(base_ptr as *mut u8, key) };
            #[cfg(debug_assertions)]
            println!("Page allocating to: {:?}, Pointer: {:?}", x, ptr);
        }
        *ptr = MaybeUninit::new(Atomic::new(info));

        //ptr.store(info, Ordering::Release);
    }

    #[inline]

    fn unsafe_addr_to_key<T>(&self, base_ptr: *const MaybeUninit<Atomic<PageInfo>>, ptr: *const T) -> usize {
        /*
        println!("ptr: {:x?}", ptr);
        let i = (ptr as usize >> PM_KEY_SHIFT);
        println!("i: {:x?}", i);
        println!("KEY_MASK: {:x?}", PM_KEY_MASK);
        let key = (i - (base_ptr as usize >> PM_KEY_SHIFT)) & PM_KEY_MASK as usize;
        println!("key: {:?}", key);

         */
        let key = ((ptr as usize) >> PM_KEY_SHIFT) & PM_KEY_MASK as usize;
        key
    }

     */

    #[inline]
    pub fn get_page_info<T : ?Sized>(&self, ptr: *const T) -> PageInfo {
        let key = self.addr_to_key(ptr);
        //println!("GET KEY: {:?}", key);
        let ptr = &self.page_map[key];
        #[cfg(windows)]
        {
            unsafe {
                // self.commit_page(self.mem_location.unwrap() as *mut u8, key)
                self.commit_page_of_ptr(ptr);
            };
        }
        let info: PageInfo = ptr.load(Ordering::Acquire);
        info
    }

    #[inline]
    pub fn set_page_info<T>(&self, ptr: *const T, info: PageInfo) {
        let key = self.addr_to_key(ptr);
        //println!("SET KEY: {:?}", key);
        let ptr = &self.page_map[key];
        #[cfg(windows)]
        {
            unsafe {
                // let page_ptr = self.commit_page(self.mem_location.unwrap() as *mut u8, key);
                self.commit_page_of_ptr(ptr);
                // println!("Page ptr: {:x?}, Ptr: {:x?}", page_ptr, ptr as * const _);
            };
        }
        ptr.store(info, Ordering::Release);
    }

    #[inline]
    fn addr_to_key<T : ?Sized>(&self, ptr: *const T) -> usize {
        /*
        println!("ptr: {:x?}", ptr);
        let i = (ptr as usize >> PM_KEY_SHIFT);
        println!("i: {:x?}", i);
        println!("KEY_MASK: {:x?}", PM_KEY_MASK);
        let mem_loc = self.mem_location.unwrap();
        let key = (i - (mem_loc as usize >> PM_KEY_SHIFT)) & PM_KEY_MASK as usize;
        println!("key: {:?}", key);

         */
        let key = ((ptr as * const u8 as usize) >> PM_KEY_SHIFT) & PM_KEY_MASK as usize;
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

        let mut info: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION {
            BaseAddress: null_mut(),
            AllocationBase: null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            State: 0,
            Protect: 0,
            Type: 0,
        };

        if VirtualQuery(
            page as LPCVOID,
            &mut info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == 0
        {
            panic!("Failed to query the virtual address");
        } else {
            if info.State | MEM_COMMIT == info.State {
                return page;
            }
        }

        let alloc = VirtualAlloc(page, PAGE, MEM_COMMIT, PAGE_READWRITE);
        if alloc.is_null() {
            panic!("Couldn't commit page")
        }
        alloc
    }

    #[cfg(windows)]
    unsafe fn commit_page_of_ptr<T>(&self, ptr: *const T) -> *mut c_void {
        let page_addr = self.addr_to_key(ptr) << PM_KEY_SHIFT;
        let page_ptr = page_addr as *mut c_void;

        let mut info: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION {
            BaseAddress: null_mut(),
            AllocationBase: null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            State: 0,
            Protect: 0,
            Type: 0,
        };

        if VirtualQuery(
            page_ptr as LPCVOID,
            &mut info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == 0
        {
            panic!("Failed to query the virtual address");
        } else {
            if info.State | MEM_COMMIT == info.State {
                return page_ptr;
            }
        }

        let alloc = VirtualAlloc(page_ptr, PAGE, MEM_COMMIT, PAGE_READWRITE);
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
