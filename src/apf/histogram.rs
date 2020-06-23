use std::collections::HashMap;
use crate::apf::constants::INIT_HISTOGRAM_LENGTH;
use crate::allocate_type;
use crate::thread_cache::no_tuning;
use crate::pages::external_mem_reservation::AllocationError;

use std::slice::from_raw_parts_mut;

/*
    Histogram class -- really just a Hashmap
*/
/* #[derive(Debug)]
pub struct Histogram<'a> {
    histogram: &'a mut [usize],
    max_key: usize
}

impl<'a> Histogram<'a> {
    pub fn new() -> Histogram<'a> {
        let page = allocate_type::<[usize; INIT_HISTOGRAM_LENGTH]>() as * mut usize as *mut u8;
        assert!(!page.is_null(), "Error initializing trace: {:?}", Err(AllocationError::AllocationFailed(INIT_HISTOGRAM_LENGTH, errno::errno())));

        let ptr = page as *mut usize;
        let histogram = unsafe {
            from_raw_parts_mut(
                ptr, 
                INIT_HISTOGRAM_LENGTH
            );
        };

        Histogram {
            histogram: histogram,
            max_key: 0
        }
    }

    pub fn increment(&mut self, key: usize) -> () {
        if key > self.max_key {
            grow(self);
        }
    }

    pub fn add(&mut self, key: usize, val: usize) {

        no_tuning( || *self.histogram.entry(key).or_insert(0) += val);

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
}*/

#[derive(Debug)]
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
}
