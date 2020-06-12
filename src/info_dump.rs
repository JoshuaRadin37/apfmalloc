use crate::mem_info::MAX_SZ_IDX;

pub struct ThreadInfoDump {
    blocks_remaining: [usize; MAX_SZ_IDX]
}