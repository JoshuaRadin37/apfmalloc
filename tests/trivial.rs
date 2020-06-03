use lralloc_rs::{do_malloc, do_free};
use bitfield::size_of;
use std::mem::MaybeUninit;

#[test]
fn run() {
    unsafe {
        let o = &mut *(do_malloc(size_of::<Option<usize>>()) as *mut MaybeUninit<Option<usize>>);

        *o = MaybeUninit::new(Some(8));
        let o = &o.assume_init();
        assert_eq!(o, &Some(8));

        do_free(o as *const _);
    }
}