use crate::apf::timescale_functions::{LivenessCounter, ReuseCounter};
use crate::apf::trace::Trace;

use std::collections::HashMap;

mod histogram;
mod trace;
mod timescale_functions;

static REUSE_BURST_LENGTH: usize = 9;
static REUSE_HIBERNATION_LENGTH: usize = 20;
static USE_ALLOCATION_CLOCK: bool = true;

static APF: u16 = 10; // No idea what this should be

pub struct ThreadApfTuner {
	id: usize,
	l_counter: LivenessCounter,
	r_counter: ReuseCounter,
	trace: Trace,
	fetch_count: u16,
	time: usize
}

impl ThreadApfTuner {
	pub fn new(id: usize) -> ThreadApfTuner {
		ThreadApfTuner {
			id,
			l_counter: LivenessCounter::new(),
			r_counter: ReuseCounter::new(REUSE_BURST_LENGTH, REUSE_HIBERNATION_LENGTH),
			trace: Trace::new(),
			fetch_count: 0,
			time: 0
		}
	}

	pub fn malloc(&mut self, ptr: *mut u8) {
		self.l_counter.alloc();
		self.r_counter.alloc(ptr as usize);
		self.time += 1;
	}

	pub fn free(&mut self, ptr: *mut u8) {
		self.l_counter.free();
		self.r_counter.free(ptr as usize);
		if !USE_ALLOCATION_CLOCK { self.time += 1; }
	}

	pub fn count_fetch(&mut self) {
		self.fetch_count += 1;
	}

	// Average demand in windows of length k
	// Returns none if reuse counter has not completed a burst yet
	pub fn demand(&self, k: usize) -> Option<f32> {
		match self.r_counter.reuse(k) {
			Some(r) => Some(self.l_counter.liveness(k) - self.l_counter.liveness(0) - r),
			None => None
		}
	}
}

pub struct GlobalApfTuner {
	dynamic_apf: u16,
	threads: HashMap<usize, ThreadApfTuner>
}

impl GlobalApfTuner {
	pub fn new() -> GlobalApfTuner {
		GlobalApfTuner {
			dynamic_apf: APF,
			threads: HashMap::<usize, ThreadApfTuner>::new()
		}
	}

	pub fn add_thread(&mut self, id: usize) {
		self.threads.insert(id, ThreadApfTuner::new(id));
	}

	// Not sure if next two methods should exist

	// Mallocs to thread
	pub fn malloc_to(&mut self, id: usize, ptr: *mut u8) -> bool {
		return false;
	}

	// Frees from thread
	pub fn free_from(&mut self, id: usize, ptr: *mut u8) -> bool {
		return false;
	}
}