// use crate::apf::timescale_functions::{LivenessCounter, ReuseCounter};
use crate::apf::liveness_counter::LivenessCounter;
use crate::apf::reuse_counter::ReuseCounter;
use crate::apf::trace::Trace;

use gnuplot::{Figure, Axes2D, Caption, Color};

mod constants;
pub use constants::TARGET_APF;
pub mod histogram;
// pub mod timescale_functions;
pub mod liveness_counter;
pub mod reuse_counter;
pub mod trace;

/*
        -- APF Tuner --
    * One for each size container
    * Call malloc() and free() whenever those operations are performed
*/
// #[derive(Debug)]
pub struct ApfTuner<'a> {
    id: usize,
    l_counter: LivenessCounter<'a>,
    r_counter: ReuseCounter<'a>,
    // trace: Trace<'a>,
    time: usize,
    fetch_count: usize,
    dapf: usize,
    check: fn(usize) -> u32,
    get: fn(usize, usize) -> bool,
    ret: fn(usize, u32) -> bool,

    record: Option<Vec<(usize, usize)>>
}

impl ApfTuner<'_> {
    pub fn new<'a>(
        id: usize,
        check: fn(usize) -> u32,
        get: fn(usize, usize) -> bool,
        ret: fn(usize, u32) -> bool,
        use_record: bool
    ) -> ApfTuner<'a> {

        ApfTuner {
            id: id,
            l_counter: LivenessCounter::new(),
            r_counter: ReuseCounter::new(REUSE_BURST_LENGTH, REUSE_HIBERNATION_PERIOD),
            time: 0,
            fetch_count: 0,
            dapf: 0,
            check: check,
            get: get,
            ret: ret,
            record: match use_record {
                true => Some(Vec::<(usize, usize)>::new()),
                false => None
            }
        }
    }

    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    pub fn malloc(&mut self, ptr: *mut u8) -> bool {
        // dbg!("malloc");
        self.time += 1;

        if !USE_ALLOCATION_CLOCK {
            self.l_counter.inc_timer();
            self.l_counter.alloc();
        }

        self.r_counter.alloc(ptr as usize);
        self.r_counter.inc_timer();

        // If out of free blocks, fetch
        if (self.check)(self.id) == 0 {

            let demand;

            match self.demand(self.calculate_dapf().into()) {
                Some(d) => {
                    if self.record.is_some() {
                        let dapf = self.calculate_dapf();
                        let time = self.time;
                        self.record.as_mut().map(|rec| rec.push((time, dapf)));
                    }

                    demand = d;
                }
                None => {
                    return false;
                }
            }

            (self.get)(self.id, demand.ceil() as usize);
            self.count_fetch();
        }

        return true;
    }

    // Processes free event.
    // Check function returns number of available slots
    // Ret function returns number of slots to central reserve
    // Returns true if demand can be calculated (reuse counter has completed a burst), false if not
    pub fn free(&mut self, ptr: *mut u8) -> bool {
        self.r_counter.free(ptr as usize);
        if !USE_ALLOCATION_CLOCK {
            self.r_counter.inc_timer();
            self.time += 1;
            self.l_counter.inc_timer();
            self.l_counter.free();
        }

        let d = self.demand(self.calculate_dapf().into());

        if !d.is_some() || d.unwrap() < 0.0 {
            return false;
        }
        let demand = d.unwrap(); // Safe

        // If too many free blocks, return some
        if (self.check)(self.id) as f32 >= 2.0 * demand + 1.0 {
            let ceil = demand.ceil() as u32;
            (self.ret)(self.id, ceil + 1);
        } else {
        return true;
    }

    fn count_fetch(&mut self) {
        self.fetch_count += 1;
        if self.fetch_count > 1 {
            self.show_record();
        }
    }

    fn calculate_dapf(&self) -> usize {
        match self.time >= *TARGET_APF * (self.fetch_count + 1) {
            true => *TARGET_APF,
            false => *TARGET_APF * (self.fetch_count + 1) - self.time,
        }
    }

    // Average demand in windows of length k
    // Returns none if reuse counter has not completed a burst yet
    fn demand(&self, k: usize) -> Option<f32> {
        if k > self.time {
            return None;
        }

        match self.r_counter.reuse(k) {
            Some(r) => {
                match USE_ALLOCATION_CLOCK {
                    true => Some(k as f32 - r),
                    false => Some(self.l_counter.liveness(k) - self.l_counter.liveness(0) - r)
                }
            },
            None => None,
        }
    }

    pub fn record(&self) -> Option<Vec<(usize, usize)>> {
        match self.record.is_some() {
            true => self.record.clone(),
            false => None
        }
    }

    fn show_record(&mut self) {
        match &self.record {
            Some(rec) => {
                dbg!(rec.len());
                let mut x = Vec::with_capacity(rec.len());
                let mut y = Vec::with_capacity(rec.len());

                dbg!(rec);

                for i in 0..rec.len() {
                    x.push(rec[i].0);
                    y.push(i+1);
                    x.push(rec[i].0 + rec[i].1);
                    y.push(i+1);

                }
 
                let mut fg = Figure::new();
                let axes = fg.axes2d();
                for i in 0..x.len()/2 {
                    axes.lines_points(&x[i*2..i*2+2], &y[i*2..i*2+2], &[Caption("Line"), Color("black")]);
                }

                fg.show().expect("Unable to display figure");
            }, 
            None => {}
        }
    }
}
