use crate::apf::histogram::Histogram;

/*
    Liveness Counter
    At each alloc or free operation, call alloc() and free() methods accordingly
    Update timestep with inc_timer()
*/
#[derive(Debug)]
pub struct LivenessCounter<'a> {
    n: usize,                    // Timer
    m: usize,                    // Number of objects
    alloc_sum: Histogram<'a>,    // Sum of allocation times before time
    alloc_counts: Histogram<'a>, // Number of allocations before time
    free_sum: Histogram<'a>,     // Sum of free times before time
    free_counts: Histogram<'a>,  // Number of frees before time
}

impl LivenessCounter<'_> {
    pub fn new<'a>() -> LivenessCounter<'a> {
        LivenessCounter {
            n: 0, // Start at 1 or 0?
            m: 0,
            alloc_sum: Histogram::new(), // Need to add anything at start?
            alloc_counts: Histogram::new(),
            free_sum: Histogram::new(),
            free_counts: Histogram::new(),
        }
    }

    // Call whenever memory is allocated
    pub fn alloc(&mut self) {
        self.alloc_sum.add(self.n, self.n);
        self.alloc_counts.increment(self.n);
        self.m += 1;
    }

    // Call whenever memory is freed
    pub fn free(&mut self) {
        self.free_sum.add(self.n, self.n);
        self.free_counts.increment(self.n);
    }

    // According to the paper, the timestep can be updated after either every operation or only allocations
    pub fn inc_timer(&mut self) {
        self.n += 1;
        self.alloc_counts
            .add(self.n, self.alloc_counts.get(self.n - 1));
        self.alloc_sum.add(self.n, self.alloc_sum.get(self.n - 1));
        self.free_counts
            .add(self.n, self.free_counts.get(self.n - 1));
        self.free_sum.add(self.n, self.free_sum.get(self.n - 1));
    }

    // Evaluates liveness for windows of size k
    pub fn liveness(&self, k: usize) -> f32 {
        let i = self.n as isize - k as isize + 1;
        if i < 0 {
            return 0.0;
        } else {
            let i = i as usize;
            let tmp1 = (self.m - self.free_counts.get(i)) * i + self.free_sum.get(i);
            let tmp2 =
                self.alloc_counts.get(k) * k + self.alloc_sum.get(self.n) - self.alloc_sum.get(k);
            ((tmp1 + self.m * k - tmp2) as f32) / i as f32
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_liveness_counter() {
        let mut lc = LivenessCounter::new();
        lc.inc_timer();
        lc.alloc(); // a1
        lc.inc_timer();
        lc.alloc(); // a2
        lc.inc_timer();
        lc.alloc(); // a3

        assert_eq!(lc.liveness(1), 2.0);
    }
}
