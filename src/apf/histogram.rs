use std::collections::HashMap;

/*
    Histogram class -- really just a Hashmap
*/
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
        *self.histogram.entry(key).or_insert(0) += 1;
    }

    pub fn add(&mut self, key: usize, val: usize) -> () {
        *self.histogram.entry(key).or_insert(0) += val;
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
