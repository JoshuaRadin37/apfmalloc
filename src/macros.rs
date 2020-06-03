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

#[macro_export]
macro_rules! min_align {
    () => {
        std::mem::sizeof::<*const ()>
    };
}

/*
macro_rules! align_val {
    ($val:expr, $align:expr) => {
        unsafe {
            let align = std::mem::align_of_val($expr);

        }
    };
}
 */

macro_rules! page_ceiling {
    ($s:expr) => {{
        let page = $crate::mem_info::PAGE;
        ($s + page - 1) & !(page - 1)
    }};
}
