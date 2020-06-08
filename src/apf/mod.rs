use crate::apf::timescale_functions::{LivenessCounter, ReuseCounter};
use crate::apf::trace::Trace;

use std::collections::HashMap;

mod histogram;
mod trace;
mod timescale_functions;

static REUSE_BURST_LENGTH: usize = 9;
static REUSE_HIBERNATION_LENGTH: usize = 20;
static USE_ALLOCATION_CLOCK: bool = true;

static TARGET_APF: u16 = 10; // No idea what this should be

/*
		-- APF Tuner --
	* One for each thread
	* Call malloc() and free() whenever those operations are performed
*/
pub struct ApfTuner<'a> {
	l_counter: LivenessCounter,
	r_counter: ReuseCounter,
	trace: Trace,
	time: u16,
	fetch_count: u16,
	dapf: u16,
	check: &'a dyn Fn() -> u16,
	get: &'a dyn Fn(usize) -> bool,
	ret: &'a dyn Fn(usize) -> bool
}

impl ApfTuner<'_> {

	// Constructor -- takes functions check, get, and ret, which get number of free blocks, fetch more, and return some, respectively
	pub fn new<'a>(check: &'a dyn Fn() -> u16, get: &'a dyn Fn(usize)  -> bool, ret: &'a dyn Fn(usize) -> bool) -> ApfTuner<'a> {
		ApfTuner {
			l_counter: LivenessCounter::new(),
			r_counter: ReuseCounter::new(REUSE_BURST_LENGTH, REUSE_HIBERNATION_LENGTH),
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
		self.l_counter.alloc();
		self.r_counter.alloc(ptr as usize);
		self.time += 1;

		// If out of free blocks, fetch
		if (self.check)() == 0 { 
			let demand;
			match self.demand(self.calculate_dapf().into()) {
				Some(d) => { demand = d; }
				None => { return false;}
			}

			(self.get)(demand.ceil() as usize); 
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
		if !USE_ALLOCATION_CLOCK { self.time += 1; }

		let d = self.demand(self.calculate_dapf().into());
		if !d.is_some() { return false; }
		let demand = d.unwrap(); // Safe

		// If too many free blocks, return some
		if (self.check)() as f32 >= 2.0 * demand + 1.0 { 
			let demand;
			match self.demand(self.calculate_dapf().into()) {
				Some(d) => { demand = d; }
				None => { return false;}
			}

			(self.ret)(demand.ceil() as usize + 1); 
		}
		return true;
	}

	fn count_fetch(&mut self) {
		self.fetch_count += 1;
	}

	fn calculate_dapf(&self) -> u16 {
		let mut dapf = TARGET_APF * (self.fetch_count + 1) - self.time;
		if dapf <= 0 { dapf = TARGET_APF; }
		dapf
	}

	// Average demand in windows of length k
	// Returns none if reuse counter has not completed a burst yet
	fn demand(&self, k: usize) -> Option<f32> {
		match self.r_counter.reuse(k) {
			Some(r) => Some(self.l_counter.liveness(k) - self.l_counter.liveness(0) - r),
			None => None
		}
	}
}