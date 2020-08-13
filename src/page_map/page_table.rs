use std::sync::atomic::{AtomicPtr, Ordering};
use crate::page_map::PageInfo;
use crate::pages::{page_alloc, independent_allocate, independent_deallocate};
use crate::mem_info::PAGE;
use std::ptr::null_mut;

const MASK: u64 = 0x1FF;

trait PageTable<R> {

    unsafe fn new() -> Self;


    fn get_mask() -> u64 {
        MASK << Self::get_shift()
    }

    fn get_shift() -> u64;

    fn get_key(addr: u64) -> usize {
        ((addr & Self::get_mask()) >> Self::get_shift()) as usize
    }

    fn get_entry(&self, addr: usize) -> &AtomicPtr<R> {
        let key = Self::get_key(addr as u64);
        unsafe {
            if key >= 512 {
                panic!("Key can't be greater than 512")
            }
            &self.get_entries()[key]
        }
    }

    unsafe fn get_entries(&self) -> &[AtomicPtr<R>; 512];
}

struct PageTableHigh {
    entries: * mut [AtomicPtr<PageTableMedHigh>; 512]
}

impl PageTable<PageTableMedHigh> for PageTableHigh {
    unsafe fn new() -> Self {
        let mem = page_alloc(PAGE).unwrap() as *mut [AtomicPtr<PageTableMedHigh>; 512];
        PageTableHigh {
            entries: mem
        }
    }

    fn get_shift() -> u64 {
        39
    }

    unsafe fn get_entries(&self) -> &[AtomicPtr<PageTableMedHigh>; 512] {
        &*self.entries
    }
}

struct PageTableMedHigh {
    entries: *mut [AtomicPtr<PageTableMedLow>; 512]
}

impl PageTable<PageTableMedLow> for PageTableMedHigh {
    unsafe fn new() -> Self {
        let mem = page_alloc(PAGE).unwrap() as *mut [AtomicPtr<PageTableMedLow>; 512];
        PageTableMedHigh {
            entries: mem
        }
    }

    fn get_shift() -> u64 {
        30
    }

    unsafe fn get_entries(&self) -> &[AtomicPtr<PageTableMedLow>; 512] {
        &* self.entries
    }
}

struct PageTableMedLow {
    entries: * mut [AtomicPtr<PageTableLow>; 512]
}
impl PageTable<PageTableLow> for PageTableMedLow {
    unsafe fn new() -> Self {
        let mem = page_alloc(PAGE).unwrap() as *mut [AtomicPtr<PageTableLow>; 512];
        PageTableMedLow {
            entries: mem
        }
    }

    fn get_shift() -> u64 {
        21
    }

    unsafe fn get_entries(&self) -> &[AtomicPtr<PageTableLow>; 512] {
        &* self.entries
    }
}

struct PageTableLow {
    entries: * mut [AtomicPtr<PageInfo>; 512]
}

impl PageTable<PageInfo> for PageTableLow {
    unsafe fn new() -> Self {
        let mem = page_alloc(PAGE).unwrap() as *mut [AtomicPtr<PageInfo>; 512];
        PageTableLow {
            entries: mem
        }
    }

    fn get_shift() -> u64 {
        12
    }

    unsafe fn get_entries(&self) -> &[AtomicPtr<PageInfo>; 512] {
        &* self.entries
    }
}

pub struct PageInfoTable {
    high_table: AtomicPtr<PageTableHigh>,
}

impl PageInfoTable {

    pub const fn new() -> Self {
        Self {
            high_table: AtomicPtr::new(null_mut())
        }
    }

    pub fn get_page_info(&self, addr: *mut u8) -> Option<PageInfo> {
        let addr = addr as usize;
        unsafe {
            let high = self.high_table.load(Ordering::Acquire);
            if high.is_null() {
                return None;
            }
            let med_high = (*high).get_entry(addr).load(Ordering::Acquire);
            if med_high.is_null() {
                return None;
            }
            let med_low = (*med_high).get_entry(addr).load(Ordering::Acquire);
            if med_low.is_null() {
                return None;
            }
            let low = (*med_low).get_entry(addr).load(Ordering::Acquire);
            if low.is_null() {
                return None;
            }
            let info = (*low).get_entry(addr).load(Ordering::Acquire);
            if info.is_null() {
                return None;
            }
            Some(*info)
        }
    }

    pub fn set_page_info(&mut self, addr: * mut u8, info: PageInfo) {
        let addr = addr as usize;
        unsafe {
            let high =
                if self.high_table.load(Ordering::Acquire).is_null() {
                    let next: *mut PageTableHigh = independent_allocate();
                    (*next).entries = independent_allocate();
                    if !self.high_table.compare_and_swap(null_mut(), next, Ordering::Relaxed).is_null() {
                        independent_deallocate((*next).entries);
                        independent_deallocate(next);
                    }
                    self.high_table.load(Ordering::Relaxed)
                } else {
                    self.high_table.load(Ordering::Relaxed)
                };
            assert!(!high.is_null());
            let med_high_ptr = (*high).get_entry(addr);
            let med_high =
                if med_high_ptr.load(Ordering::Acquire).is_null() {
                    let next: *mut PageTableMedHigh = independent_allocate();
                    (*next).entries = independent_allocate();
                    if !med_high_ptr.compare_and_swap(null_mut(), next, Ordering::Relaxed).is_null() {
                        independent_deallocate((*next).entries);
                        independent_deallocate(next);
                    }
                    med_high_ptr.load(Ordering::Relaxed)
                } else {
                    med_high_ptr.load(Ordering::Relaxed)
                };
            assert!(!med_high.is_null());
            let med_low_ptr = (*med_high).get_entry(addr);
            let med_low =
                if med_low_ptr.load(Ordering::Acquire).is_null() {
                    let next: *mut PageTableMedLow = independent_allocate();
                    (*next).entries = independent_allocate();
                    if !med_low_ptr.compare_and_swap(null_mut(), next, Ordering::Relaxed).is_null() {
                        independent_deallocate((*next).entries);
                        independent_deallocate(next);
                    }
                    med_low_ptr.load(Ordering::Relaxed)
                } else {
                    med_low_ptr.load(Ordering::Relaxed)
                };
            let low_ptr = (*med_low).get_entry(addr);
            let low =
                if low_ptr.load(Ordering::Acquire).is_null() {
                    let next: *mut PageTableLow = independent_allocate();
                    (*next).entries = independent_allocate();
                    if !low_ptr.compare_and_swap(null_mut(), next, Ordering::Relaxed).is_null() {
                        independent_deallocate((*next).entries);
                        independent_deallocate(next);
                    }
                    low_ptr.load(Ordering::Relaxed)
                } else {
                    low_ptr.load(Ordering::Relaxed)
                };
            assert!(!low.is_null());
            let a_info_ptr = (*low).get_entry(addr);
            let info_ptr =
                if a_info_ptr.load(Ordering::Acquire).is_null() {
                    let next = independent_allocate();
                    if !a_info_ptr.compare_and_swap(null_mut(), next, Ordering::Relaxed).is_null() {
                        independent_deallocate(next);
                    }
                    a_info_ptr.load(Ordering::Relaxed)
                } else {
                    a_info_ptr.load(Ordering::Relaxed)
                };
            assert!(!info_ptr.is_null());
            info_ptr.write(info)
        }

    }

    pub fn get_total_size(&self) -> usize {
        let table = self.high_table.load(Ordering::Acquire);
        if table.is_null() {
            0
        } else {
            unsafe {
                let mut sum = PAGE;
                let high = & *table;
                for i in 0..512 {
                    let med_high = high
                        .get_entry(i << PageTableHigh::get_shift())
                        .load(Ordering::Acquire);

                    if !med_high.is_null() {
                        sum += PAGE;
                        for i in 0..512 {
                            let med_low =
                                (*med_high)
                                    .get_entry(i << PageTableMedHigh::get_shift())
                                    .load(Ordering::Acquire);

                            if !med_low.is_null() {
                                sum += PAGE;
                                for i in 0..512 {
                                    let low =
                                        (*med_low)
                                            .get_entry(i << PageTableMedLow::get_shift())
                                            .load(Ordering::Acquire);

                                    if !low.is_null() {
                                        sum += PAGE;
                                        for i in 0..512 {

                                            let info =
                                                (*low)
                                                    .get_entry(i << PageTableLow::get_shift())
                                                    .load(Ordering::Acquire);

                                            if !info.is_null() {
                                                sum += std::mem::size_of::<PageInfo>()
                                            }
                                        }


                                    }
                                }
                            }
                        }
                    }
                }

                sum
            }
        }
    }


}

unsafe impl Send for PageInfoTable { }
unsafe impl Sync for PageInfoTable { }


#[cfg(test)]
mod test {
    use crate::page_map::page_table::PageInfoTable;
    use crate::ptr::auto_ptr::AutoPtr;
    use crate::page_map::PageInfo;
    use crate::pages::page_alloc;
    use crate::mem_info::PAGE;
    use std::thread;

    #[test]
    fn none_test() {
        let data = 0;
        let ptr = &data as *const i32;
        let table = PageInfoTable::new();
        assert!(table.get_page_info(ptr as *mut u8).is_none());

        println!("Total space used for table = {} bytes", table.get_total_size());
    }

    #[test]
    fn insert_single_pointer() {
        let mut table = PageInfoTable::new();

        let auto_ptr = AutoPtr::new(0usize);
        unsafe {
            let ptr= auto_ptr.get_ptr() as *mut u8;
            let info = PageInfo::default();
            table.set_page_info(ptr, info);
            assert!(table.get_page_info(ptr).is_some());
        }

        println!("Total space used for table = {} bytes", table.get_total_size());
    }

    #[test]
    fn insert_many_pointers() {
        let mut table = PageInfoTable::new();

        for _i in 0..1000 {
            let ptr = page_alloc(PAGE).unwrap();
            let info = PageInfo::default();
            table.set_page_info(ptr, info);
            assert!(table.get_page_info(ptr).is_some());

        }
        println!("Total space used for table = {} bytes", table.get_total_size());
    }

    #[test]
    fn insert_many_pointers_on_many_threads() {

        static mut TABLE: PageInfoTable = PageInfoTable::new();

        let mut vec = vec![];
        for _ in 0..16 {

            vec.push(thread::spawn(|| unsafe {
                for _i in 0..1000 {

                    let auto_ptr = page_alloc(PAGE).unwrap();
                    let info = PageInfo::default();
                    TABLE.set_page_info(auto_ptr, info);
                    assert!(TABLE.get_page_info(auto_ptr).is_some());

                }
            }));
        }
        for thread in vec {
            thread.join().unwrap();
        }


        unsafe {
            println!("Total space used for table = {} bytes", TABLE.get_total_size());
        }
    }
}

