use crate::allocation_data::SuperBlockState::{EMPTY, FULL, PARTIAL};

#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum SuperBlockState {
    FULL,
    PARTIAL,
    EMPTY,
}

mod desc;
mod proc_heap;
pub use desc::{Descriptor, DescriptorNode};
pub use proc_heap::{get_heaps, Heaps, ProcHeap};

impl From<u64> for SuperBlockState {
    fn from(u: u64) -> Self {
        match u {
            0 => FULL,
            1 => PARTIAL,
            2 => EMPTY,
            _ => panic!("Not a valid option"),
        }
    }
}

impl Into<u64> for SuperBlockState {
    fn into(self) -> u64 {
        match self {
            FULL => 0,
            PARTIAL => 1,
            EMPTY => 2,
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

impl Clone for Anchor {
    fn clone(&self) -> Self {
        let Anchor(inner) = self;
        Self(*inner)
    }
}

impl Copy for Anchor {}

impl PartialEq for Anchor {
    fn eq(&self, other: &Self) -> bool {
        if self.state() != other.state() {
            return false;
        }
        if self.avail() != other.avail() {
            return false;
        }
        if self.count() != other.count() {
            return false;
        }

        true
    }
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
        assert_eq!(anchor.state(), EMPTY, "{:?} ", anchor);
    }

    #[test]
    fn anchor_copy_equality() {
        let mut anchor = Anchor::default();
        anchor.set_state(EMPTY);
        anchor.set_avail(4);
        let other = anchor.clone();
        assert_eq!(other, anchor);
    }
}
