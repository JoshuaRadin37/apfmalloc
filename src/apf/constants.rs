pub const INIT_TRACE_LENGTH: usize = 1 << 14;
#[allow(unused)]
pub const INIT_HISTOGRAM_LENGTH: usize = 1 << 5;
// pub const REUSE_BURST_LENGTH: usize = 1000;
// pub const REUSE_HIBERNATION_PERIOD: usize = 2000;
pub const USE_ALLOCATION_CLOCK: bool = true;
use crate::thread_cache::no_tuning;
lazy_static::lazy_static! {
pub static ref TARGET_APF: usize = no_tuning(|| option_env!("TARGET_APF").map(|apf| apf.parse::<usize>().unwrap_or(2500)).unwrap_or(2500));
pub static ref REUSE_BURST_LENGTH: usize = no_tuning(|| option_env!("BURST_LENGTH").map(|apf| apf.parse::<usize>().unwrap_or(300)).unwrap_or(300));
pub static ref REUSE_HIBERNATION_PERIOD: usize = no_tuning(|| option_env!("HIBERNATION_PERIOD").map(|apf| apf.parse::<usize>().unwrap_or(*REUSE_BURST_LENGTH*2)).unwrap_or(*REUSE_BURST_LENGTH*2));
}
