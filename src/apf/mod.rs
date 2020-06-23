use crate::apf::constants::{REUSE_BURST_LENGTH, REUSE_HIBERNATION_PERIOD, USE_ALLOCATION_CLOCK};
use crate::apf::timescale_functions::{LivenessCounter, ReuseCounter};
use crate::apf::trace::Trace;

mod constants;
pub use constants::TARGET_APF;
pub mod histogram;
pub mod timescale_functions;
pub mod trace;

/*
        -- APF Tuner --
    * One for each size container
    * Call malloc() and free() whenever those operations are performed
*/
#[derive(Debug)]
pub struct ApfTuner<'a> {
    id: usize,
    l_counter: LivenessCounter,
    r_counter: ReuseCounter<'a>,
    trace: Trace<'a>,
    time: usize,
    fetch_count: usize,
    dapf: usize,
    check: fn(usize) -> u32,
    get: fn(usize, usize) -> bool,
    ret: fn(usize, u32) -> bool,
}

impl ApfTuner<'_> {
    pub fn new<'a>(
        id: usize,
        check: fn(usize) -> u32,
        get: fn(usize, usize) -> bool,
        ret: fn(usize, u32) -> bool,
    ) -> ApfTuner<'a> {
        let tuner = ApfTuner {
            id: id,
            l_counter: LivenessCounter::new(),
            r_counter: ReuseCounter::new(REUSE_BURST_LENGTH, REUSE_HIBERNATION_PERIOD),
            trace: Trace::new(),
            time: 0,
            fetch_count: 0,
            dapf: 0,
            check: check,
            get: get,
            ret: ret,
        };
        tuner
    }

    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    pub fn malloc(&mut self, ptr: *mut u8) -> bool {
        // dbg!("malloc");
        self.time += 1;

        self.l_counter.inc_timer();
        self.l_counter.alloc();

        self.r_counter.alloc(ptr as usize);
        self.r_counter.inc_timer();

        // If out of free blocks, fetch
        if (self.check)(self.id) == 0 {
            let demand;
            match self.demand(self.calculate_dapf().into()) {
                Some(d) => {
                    demand = d;
                }
                None => {
                    return false;
                }
            }

            (self.get)(self.id, demand.ceil() as usize);
            self.count_fetch();
        }
        else {
            let alt = (self.check)(self.id);
            let dummy: usize;
        }
        return true;
    }

    // Processes free event.
    // Check function returns number of available slots
    // Ret function returns number of slots to central reserve
    // Returns true if demand can be calculated (reuse counter has completed a burst), false if not
    pub fn free(&mut self, ptr: *mut u8) -> bool {
        // dbg!("free");
        self.r_counter.free(ptr as usize);
        if !USE_ALLOCATION_CLOCK {
            self.time += 1;
            self.l_counter.inc_timer();
        }

        self.l_counter.free();

        if !USE_ALLOCATION_CLOCK {
            self.r_counter.inc_timer();
        }

        let d = self.demand(self.calculate_dapf().into());
        if !d.is_some() {
            return false;
        }
        let demand = d.unwrap(); // Safe

        // If too many free blocks, return some
        if (self.check)(self.id) as f32 >= 2.0 * demand + 1.0 {
            let demand;
            match self.demand(self.calculate_dapf().into()) {
                Some(d) => {
                    demand = d;
                }
                None => {
                    return false;
                }
            }
            if demand < 0.0 {
                return false;
            }
            let ciel = demand.ceil() as u32;
            (self.ret)(self.id, ciel + 1);
        }
        else {
            let alt = (self.check)(self.id);
            let dummy: usize;
        }
        return true;
    }

    fn count_fetch(&mut self) {
        self.fetch_count += 1;
    }

    fn calculate_dapf(&self) -> usize {
        let dapf;

        if self.time >= *TARGET_APF * (self.fetch_count + 1) {
            dapf = *TARGET_APF;
        } else {
            dapf = *TARGET_APF * (self.fetch_count + 1) - self.time;
        }

        dapf
    }

    // Average demand in windows of length k
    // Returns none if reuse counter has not completed a burst yet
    fn demand(&self, k: usize) -> Option<f32> {
        if k > self.time {
            return None;
        }

        match self.r_counter.reuse(k) {
            Some(r) => Some(self.l_counter.liveness(k) - self.l_counter.liveness(0) - r),
            None => None,
        }
    }
}
