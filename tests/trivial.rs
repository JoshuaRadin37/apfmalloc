use bitfield::size_of;
use lralloc_rs::{do_free, do_malloc};
use std::mem::MaybeUninit;

#[test]
fn run() {
    unsafe {
        let o = &mut *(do_malloc(size_of::<Option<usize>>()) as *mut MaybeUninit<Option<usize>>);


        *o = MaybeUninit::new(Some(8));
        let o = &o.assume_init();
        assert_eq!(o, &Some(8));

        do_free(o as *const Option<usize>);
    }
}
