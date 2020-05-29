

#[macro_export]
macro_rules! size_class_bin_yes {
    ($block:expr, $pages:expr) => {
        SizeClassData {
            block_size: $block,
            sb_size: ($pages * $crate::mem_info::PAGE) as u32,
            block_num: 0,
            cache_block_num: 0,
        }
    };
}


#[macro_export]
macro_rules! sc {
    ($index:expr, $lg_grp:expr, $lg_delta:expr, $ndelta:expr, $psz:expr, yes, $pgs:expr, $lg_delta_lookup:expr) => {
        $crate::size_class_bin_yes!(((1usize << $lg_grp) + ($ndelta << $lg_delta)) as u32, $pgs)
    };
}


