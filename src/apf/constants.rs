pub const INIT_TRACE_LENGTH: usize = 1 << 14;
pub const REUSE_BURST_LENGTH: usize = 20000;
pub const REUSE_HIBERNATION_PERIOD: usize = 40000;
pub const USE_ALLOCATION_CLOCK: bool = true;
use crate::thread_cache::no_tuning;
lazy_static::lazy_static! {
pub static ref TARGET_APF: usize = no_tuning(|| option_env!("TARGET_APF").map(|apf| apf.parse::<usize>().unwrap_or(5000)).unwrap_or(5000));
}
/*
{
    crate::thread_cache::skip_tuners.with(
            |b| unsafe {
                *b.get() = true;
            }
        );
    let target = option_env!("TARGET_APF").map(|apf| apf.parse::<usize>().unwrap_or(100)).unwrap_or(100);
    crate::thread_cache::skip_tuners.with(
            |b| unsafe {
                *b.get() = false;
            }
        );
    target
}; // No idea what this should be
}

 */
pub const MAX_N: usize = 150;
