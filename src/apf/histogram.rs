use std::collections::HashMap;
use crate::thread_cache::no_tuning;

/*
    Histogram class -- really just a Hashmap
*/
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
}
