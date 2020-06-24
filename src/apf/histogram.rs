use crate::apf::constants::INIT_HISTOGRAM_LENGTH;
use crate::thread_cache::no_tuning;
use crate::pages::external_mem_reservation::AllocationError;
use crate::{ do_realloc, allocate_type };

use std::slice::from_raw_parts_mut;

use std::ffi::c_void;
use std::mem::size_of;

/*
    Histogram class -- really just a Hashmap
*/
#[derive(Debug)]
pub struct Histogram<'a> {
    histogram: &'a mut [usize],
    max_key: usize
}

impl<'a> Histogram<'a> {
    pub fn new() -> Histogram<'a> {
        let page = allocate_type::<[usize; INIT_HISTOGRAM_LENGTH]>() as * mut usize as *mut u8;
        assert!(!page.is_null(), "Error initializing histogram: {:?}", AllocationError::AllocationFailed(INIT_HISTOGRAM_LENGTH, errno::errno()));

        let ptr = page as *mut usize;
        let histogram = unsafe {
            from_raw_parts_mut(
                ptr, 
                INIT_HISTOGRAM_LENGTH
            )
        };

        for i in 0..INIT_HISTOGRAM_LENGTH {
            unsafe { (&mut histogram[i]as *mut usize).write(0) };
        }

        Histogram {
            histogram: histogram,
            max_key: INIT_HISTOGRAM_LENGTH
        }
    }

    pub fn increment(&mut self, key: usize) -> () {
        if key >= self.max_key - 1 {
            self.grow();
        }

        self.histogram[key] += 1;
    }

    pub fn add(&mut self, key: usize, val: usize) {
        if key >= self.max_key - 1 {
            self.grow();
        }

        unsafe { (&mut self.histogram[key]as *mut usize).write(self.histogram[key] + val) };

    }

    pub fn get(&self, key: usize) -> usize {
        if key >= self.max_key {
            return 0;
        }
        self.histogram[key]
    }

    // Returns number of keys
    pub fn count(&self) -> usize {
        self.histogram.len()
    }

    pub fn grow(&mut self) {
        let new_max = self.max_key * 2;
        let page = no_tuning(|| do_realloc(self.histogram.as_mut_ptr() as *mut c_void, new_max * size_of::<usize>()) as *mut u8);
        assert!(!page.is_null(), "Error initializing histogram: {:?}", AllocationError::AllocationFailed(new_max, errno::errno()));

        let ptr = page as *mut usize;
        let histogram = unsafe { from_raw_parts_mut(ptr, new_max) };

        for i in self.max_key..new_max {
            unsafe { (&mut histogram[i]as *mut usize).write(0) };
        }

        self.histogram = histogram;
        self.max_key = new_max;
    }
}

/*#[derive(Debug)]
pub struct Histogram {
    histogram: HashMap<usize, usize>,
}

impl Histogram {
    pub fn new() -> Histogram {
        Histogram {
            histogram: HashMap::new(),
        }
    }

    pub fn increment(&mut self, key: usize) -> () {
        no_tuning(|| *self.histogram.entry(key).or_insert(0) += 1);
    }

    pub fn add(&mut self, key: usize, val: usize) {
        no_tuning(|| *self.histogram.entry(key).or_insert(0) += val);
    }

    pub fn get(&self, key: &usize) -> usize {
        match self.histogram.get(key) {
            Some(n) => *n,
            None => 0,
        }
    }

    // Returns number of keys
    pub fn count(&self) -> usize {
        self.histogram.len()
    }
} */
