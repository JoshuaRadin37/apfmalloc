use bitfield::size_of;
use lralloc_rs::{do_free, do_malloc};
use std::mem::MaybeUninit;

#[test]
fn run() {
    unsafe {
        let o = (do_malloc(size_of::<Option<usize>>()) as *mut MaybeUninit<Option<usize>>);

        *o = MaybeUninit::new(Some(15));
        let o = o as *mut Option<usize>;

        do_malloc(size_of::<[usize; 64]>());

        do_free(o as *const Option<usize>);
    }
}
