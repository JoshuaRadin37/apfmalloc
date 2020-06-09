use crate::apf::constants::{
    REUSE_BURST_LENGTH, REUSE_HIBERNATION_PERIOD, TARGET_APF, USE_ALLOCATION_CLOCK,
};
use crate::apf::timescale_functions::{LivenessCounter, ReuseCounter};
use crate::apf::trace::Trace;

mod constants;
mod histogram;
mod timescale_functions;
mod trace;

/*
        -- APF Tuner --
    * One for each thread
    * Call malloc() and free() whenever those operations are performed
*/
#[derive(Copy, Clone)]
pub struct ApfTuner {
    l_counter: LivenessCounter,
    r_counter: ReuseCounter,
    trace: Trace,
    time: u16,
    fetch_count: u16,
    dapf: u16,
    check: fn(usize) -> u32,
    get: fn(usize, usize) -> bool,
    ret: fn(usize, u32) -> bool
}

impl ApfTuner {
    pub fn new(check: fn(usize) -> u32, get: fn(usize, usize) -> bool, ret: fn(usize, u32) -> bool) -> ApfTuner {
        ApfTuner {
            l_counter: LivenessCounter::new(),
            r_counter: ReuseCounter::new(),
            trace: Trace::new(),
            time: 0,
            fetch_count: 0,
            dapf: 0,
            check: check,
            get: get,
            ret: ret
        }
    }

    pub fn malloc(&mut self, ptr: *mut u8) -> bool {
        self.time += 1;

        self.l_counter.inc_timer();
        self.l_counter.alloc();

        self.r_counter.alloc(ptr as usize);
        self.r_counter.inc_timer();

        // If out of free blocks, fetch
        if (self.check)(0) == 0 {
            let demand;
            match self.demand(self.calculate_dapf().into()) {
                Some(d) => {
                    demand = d;
                }
                None => {
                    return false;
                }
            }

            (self.get)(0, demand.ceil() as usize);
            self.count_fetch();
        } 
        return true;
    }

    // Processes free event.
    // Check function returns number of available slots
    // Ret function returns number of slots to central reserve
    // Returns true if demand can be calculated (reuse counter has completed a burst), false if not
    pub fn free(&mut self, ptr: *mut u8) -> bool {
        self.l_counter.free();
        self.r_counter.free(ptr as usize);
        if !USE_ALLOCATION_CLOCK {
            self.time += 1;
        }

        let d = self.demand(self.calculate_dapf().into());
        if !d.is_some() {
            return false;
        }
        let demand = d.unwrap(); // Safe

        // If too many free blocks, return some
        if (self.check)(0) as f32 >= 2.0 * demand + 1.0 {
            let demand;
            match self.demand(self.calculate_dapf().into()) {
                Some(d) => {
                    demand = d;
                }
                None => {
                    return false;
                }
            }

            (self.ret)(0, demand.ceil() as u32 + 1);
        }
        return true;
    }

    fn count_fetch(&mut self) {
        self.fetch_count += 1;
    }

    fn calculate_dapf(&self) -> u16 {
        let mut dapf = TARGET_APF * (self.fetch_count + 1) - self.time;
        if dapf <= 0 {
            dapf = TARGET_APF;
        }
        dapf
    }

    // Average demand in windows of length k
    // Returns none if reuse counter has not completed a burst yet
    fn demand(&self, k: usize) -> Option<f32> {
        match self.r_counter.reuse(k) {
            Some(r) => Some(self.l_counter.liveness(k) - self.l_counter.liveness(0) - r),
            None => None,
        }
    }
}
