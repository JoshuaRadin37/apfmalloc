use crate::mem_info::{MAX_SZ, MAX_SZ_IDX};

pub struct SizeClassData {
    pub block_size: u32,
    pub sb_size: u32,
    pub block_num: u32,
    pub cache_block_num: u32,
}

impl SizeClassData {
    pub fn get_block_num(&self) -> usize {
        self.block_num as usize
    }
}

pub static mut SIZE_CLASS_LOOK_UP: [usize; MAX_SZ + 1] = [0; MAX_SZ + 1];
#[inline]
pub fn get_size_class(size: usize) -> usize {
    if size > MAX_SZ {
        return 0;
    }
    unsafe {
        // using .clone() just in case
        SIZE_CLASS_LOOK_UP[size].clone()
    }
}

pub unsafe fn init_size_class() {
    // Get the number of blocks in the superblocks to be correct
    let i = MAX_SZ_IDX;
    for sc_index in 1..i {
        let sc = &mut SIZE_CLASSES[sc_index];
        let block_size = sc.block_size;
        let mut sb_size = sc.sb_size;
        if sb_size > block_size && (sb_size % block_size == 0) {
            continue;
        }

        // increase the super block size
        while block_size >= sb_size {
            sb_size += sc.block_size
        }

        sc.sb_size = sb_size;
    }

    // increase super block size if needed
    for sc_index in 1..MAX_SZ_IDX {
        let sc = &mut SIZE_CLASSES[sc_index];
        let mut sb_size = sc.sb_size;
        let max = page_ceiling!(*crate::apf::TARGET_APF * sc.block_size as usize) as u32;
        while sb_size < max {
            sb_size += sc.sb_size;
        }

        sc.sb_size = sb_size;
    }

    // fill in missing fields
    for sc_index in 1..MAX_SZ_IDX {
        let sc = &mut SIZE_CLASSES[sc_index];

        sc.block_num = sc.sb_size / sc.block_size;
        sc.cache_block_num = sc.block_num;
        assert!(sc.block_num > 0);
        assert!(sc.block_num >= sc.cache_block_num);
    }

    // first size reserved
    let mut lookup_idx = 0;
    for sc_index in 1..(MAX_SZ_IDX as u32) {
        let sc = &SIZE_CLASSES[sc_index as usize];
        let block_size = sc.block_size;
        while lookup_idx <= block_size {
            SIZE_CLASS_LOOK_UP[lookup_idx as usize] = sc_index as usize;
            lookup_idx += 1;
        }
    }
}
// size class data, from jemalloc 5.0
macro_rules! size_classes {
    () => {
        [
            SizeClassData {
                block_size: 0,
                sb_size: 0,
                block_num: 0,
                cache_block_num: 0,
            },
            $crate::sc!(0, 3, 3, 0, no, yes, 1, 3),
            $crate::sc!(1, 3, 3, 1, no, yes, 1, 3),
            $crate::sc!(2, 3, 3, 2, no, yes, 3, 3),
            $crate::sc!(3, 3, 3, 3, no, yes, 1, 3),
            $crate::sc!(4, 5, 3, 1, no, yes, 5, 3),
            $crate::sc!(5, 5, 3, 2, no, yes, 3, 3),
            $crate::sc!(6, 5, 3, 3, no, yes, 7, 3),
            $crate::sc!(7, 5, 3, 4, no, yes, 1, 3),
            $crate::sc!(8, 6, 4, 1, no, yes, 5, 4),
            $crate::sc!(9, 6, 4, 2, no, yes, 3, 4),
            $crate::sc!(10, 6, 4, 3, no, yes, 7, 4),
            $crate::sc!(11, 6, 4, 4, no, yes, 1, 4),
            $crate::sc!(12, 7, 5, 1, no, yes, 5, 5),
            $crate::sc!(13, 7, 5, 2, no, yes, 3, 5),
            $crate::sc!(14, 7, 5, 3, no, yes, 7, 5),
            $crate::sc!(15, 7, 5, 4, no, yes, 1, 5),
            $crate::sc!(16, 8, 6, 1, no, yes, 5, 6),
            $crate::sc!(17, 8, 6, 2, no, yes, 3, 6),
            $crate::sc!(18, 8, 6, 3, no, yes, 7, 6),
            $crate::sc!(19, 8, 6, 4, no, yes, 1, 6),
            $crate::sc!(20, 9, 7, 1, no, yes, 5, 7),
            $crate::sc!(21, 9, 7, 2, no, yes, 3, 7),
            $crate::sc!(22, 9, 7, 3, no, yes, 7, 7),
            $crate::sc!(23, 9, 7, 4, no, yes, 1, 7),
            $crate::sc!(24, 10, 8, 1, no, yes, 5, 8),
            $crate::sc!(25, 10, 8, 2, no, yes, 3, 8),
            $crate::sc!(26, 10, 8, 3, no, yes, 7, 8),
            $crate::sc!(27, 10, 8, 4, no, yes, 1, 8),
            $crate::sc!(28, 11, 9, 1, no, yes, 5, 9),
            $crate::sc!(29, 11, 9, 2, no, yes, 3, 9),
            $crate::sc!(30, 11, 9, 3, no, yes, 7, 9),
            $crate::sc!(31, 11, 9, 4, yes, yes, 1, 9),
            $crate::sc!(32, 12, 10, 1, no, yes, 5, no),
            $crate::sc!(33, 12, 10, 2, no, yes, 3, no),
            $crate::sc!(34, 12, 10, 3, no, yes, 7, no),
            $crate::sc!(35, 12, 10, 4, yes, yes, 2, no),
            $crate::sc!(36, 13, 11, 1, no, yes, 5, no),
            $crate::sc!(37, 13, 11, 2, yes, yes, 3, no),
            $crate::sc!(38, 13, 11, 3, no, yes, 7, no),
        ]
    };
}

pub static mut SIZE_CLASSES: [SizeClassData; MAX_SZ_IDX] = size_classes!();
