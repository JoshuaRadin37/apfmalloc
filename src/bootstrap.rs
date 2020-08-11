use std::process::exit;
use std::ptr::null_mut;

use spin::Mutex;

use crate::independent_collections::Array;
use crate::mem_info::MAX_SZ_IDX;
use crate::pages::{SegAllocator, Segment, SEGMENT_ALLOCATOR};
use crate::thread_cache::ThreadCacheBin;
// use crate::page_map::RANGE_PAGE_MAP;
// use crate::allocation_data::Descriptor;
// use std::ffi::c_void;

#[allow(unused)]
pub static mut bootstrap_cache: Mutex<[ThreadCacheBin; MAX_SZ_IDX]> =
    Mutex::new([ThreadCacheBin::new(); MAX_SZ_IDX]);

static _use_bootstrap: Mutex<bool> = Mutex::new(true);

pub fn use_bootstrap() -> bool {
    *_use_bootstrap.lock()
}

/// When using the bootstrap, all threads allocate from a single source location which does not require any heap allocation itself
/// This is useful for systems that require heap space to allocate static variables of types that implement copy.
#[allow(unused)]
pub fn set_use_bootstrap(val: bool) {
    *_use_bootstrap.lock() = val;
}

pub struct BootstrapReserve {
    mem: Array<Segment>,
    next: *mut u8,
    avail: usize,
    max: usize,
}

impl BootstrapReserve {
    pub const fn new(size: usize) -> Self {
        Self {
            mem: Array::new(),
            next: null_mut(),
            avail: 0,
            max: size,
        }
    }

    pub fn init(&mut self) {
        /*
        match &mut self.mem {
            None => {

                self.mem =
                    Some(
                        SEGMENT_ALLOCATOR
                            .allocate(self.max)
                            .unwrap_or_else(|_| exit(-1)),
                    );
                // self.mem = Some(SEGMENT_ALLOCATOR.allocate(self.avail).unwrap_or_else(|_| exit(-1)));
                self.next = self.mem.as_ref().unwrap().get_ptr() as *mut u8;
                self.avail = self.max;
            }
            Some(seg) => {
                *seg = SEGMENT_ALLOCATOR
                    .allocate(self.max)
                    .unwrap_or_else(|_| exit(-1));
                self.next = seg.get_ptr() as *mut u8;
                self.avail = self.max;
            }
        }
         */
        let mem = SEGMENT_ALLOCATOR
            .allocate(self.max)
            .unwrap_or_else(|_| exit(-1));
        self.next = mem.get_ptr() as *mut u8;
        self.avail = self.max;
        self.mem.push(mem);
    }

    unsafe fn add_new_segment(&mut self, request_size: usize) {
        let size = self.max.max(request_size);
        let mem = SEGMENT_ALLOCATOR
            .allocate(size)
            .unwrap_or_else(|_| exit(-1));
        self.next = mem.get_ptr() as *mut u8;
        self.avail = size;
        self.mem.push(mem);
    }

    pub unsafe fn allocate(&mut self, size: usize) -> *mut u8 {
        if size > self.avail {
            //return null_mut();
            self.add_new_segment(size);
        }

        let ret = self.next;
        /*
        let mut guard = RANGE_PAGE_MAP.write();
        let desc = &mut *Descriptor::alloc();
        desc.block_size = size as u32;
        desc.max_count = 1;
        desc.super_block = Some(Segment::new(ret as *mut c_void, size));
        guard.update_page_map(
            None,
            ret,
            Some(desc),
            0
        );

         */
        self.next = self.next.offset(size as isize);
        self.avail -= size;
        ret
    }

    #[allow(unused)]
    pub fn ptr_in_bootstrap<T: ?Sized>(&self, ptr: *const T) -> bool {
        for segment in &self.mem {
            let start = segment.get_ptr() as usize;
            let end = start + segment.len();
            if ptr as *const u8 as usize >= start && (ptr as *const u8 as usize) < end {
                return true;
            }
        }
        false
    }
}

#[allow(unused)]
const KB: usize = 1028;
#[allow(unused)]
const MB: usize = 1028 * KB;

pub static mut bootstrap_reserve: Mutex<BootstrapReserve> =
    Mutex::new(BootstrapReserve::new(128 * KB));
