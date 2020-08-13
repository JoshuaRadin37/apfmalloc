pub const INIT_TRACE_LENGTH: usize = 1 << 14;
#[allow(unused)]
pub const INIT_HISTOGRAM_LENGTH: usize = 1 << 5;
// pub const REUSE_BURST_LENGTH: usize = 1000;
// pub const REUSE_HIBERNATION_PERIOD: usize = 2000;
pub const USE_ALLOCATION_CLOCK: bool = true;
use crate::thread_cache::no_tuning;
use crate::env::{get_env_as_usize};
lazy_static::lazy_static! {
pub static ref TARGET_APF: usize = no_tuning(|| get_env_as_usize("TARGET_APF").unwrap_or(2500));
pub static ref REUSE_BURST_LENGTH: usize = no_tuning(|| get_env_as_usize("BURST_LENGTH").unwrap_or(300));
pub static ref REUSE_HIBERNATION_PERIOD: usize = no_tuning(|| get_env_as_usize("HIBERNATION_PERIOD").unwrap_or(*REUSE_BURST_LENGTH*2));
}
