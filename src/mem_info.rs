use std::mem::size_of;

/// The maximum size class index
pub const MAX_SZ_IDX: usize = 40usize;
pub const LG_MAX_SIZE_IDX: usize = 6_usize;
/// The memory size of a block in the maximum size class
pub const MAX_SZ: usize = (1 << 13) + (1 << 11) * 3;
pub const LG_PTR: usize = size_of::<*const usize>();
/// cache line is 64 bytes
pub const LG_CACHE_LINE: usize = 6;
/// a Page is is 4kb
pub const LG_PAGE: usize = 12;
/// a huge page is 2mb
pub const LG_HUGE_PAGE: usize = 21;


pub const PTR_SIZE: usize = 1usize << LG_PTR;
pub const CACHE_LINE: usize = 1usize << LG_CACHE_LINE;
pub const PAGE: usize = 1usize << LG_PAGE;
pub const HUGE_PAGE: usize = 1 << LG_HUGE_PAGE;

pub const PTR_MASK: usize = PTR_SIZE - 1;
pub const CACHE_LINE_MASK: usize = CACHE_LINE - 1;
pub const PAGE_MASK: usize = PAGE - 1;

pub const MIN_ALIGN: usize = LG_PTR;

pub const DESCRIPTOR_BLOCK_SZ: usize = 16 * PAGE;

pub fn align_val(val: usize, align: usize) -> usize {

    (val + align - 1) & (!align + 1)
}

/// Given a size and an alignment, gives an adjusted size
pub fn align_size(size: usize, align: usize) -> usize {
    if align < size {
        align_val(size, align)
    } else {
        align
    }
}

pub fn align_addr(addr: usize, align: usize) -> *const usize {
    ((addr + align - 1) & (!align + 1)) as *const usize
}
