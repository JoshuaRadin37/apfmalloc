use crate::allocation_data::SuperBlockState::{FULL, PARTIAL, EMPTY};

#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum SuperBlockState {
    FULL,
    PARTIAL,
    EMPTY
}

mod desc;
mod proc_heap;
pub use proc_heap::{ProcHeap, get_heaps, Heaps};
pub use desc::{Descriptor, DescriptorNode};

impl From<u64> for SuperBlockState {
    fn from(u: u64) -> Self {
        match u {
            0 => FULL,
            1 => PARTIAL,
            2 => EMPTY,
            _ => panic!("Not a valid option")
        }
    }
}

impl Into<u64> for SuperBlockState {
    fn into(self) -> u64 {
        match self {
            FULL => { 0 },
            PARTIAL => { 1 },
            EMPTY => { 2 },
        }
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self(0)
    }
}


bitfield! {
    pub struct Anchor(u64);
    impl Debug;
    pub from into SuperBlockState, state, set_state: 1, 0;
    pub avail, set_avail: 31, 2;
    pub count, set_count: 63, 32;
}




#[cfg(test)]
mod test {
    use super::*;
    use bitfield::size_of;

    #[test]
    fn anchor_packed() {
        assert_eq!(size_of::<Anchor>(), size_of::<u64>())
    }

    #[test]
    fn anchor_independent() {
        let mut anchor = Anchor::default();
        assert_eq!(anchor.state(), FULL);
        anchor.set_state(EMPTY);
        anchor.set_avail(4);
        assert_eq!(anchor.state(), EMPTY, "{:?} ",anchor);
    }
}