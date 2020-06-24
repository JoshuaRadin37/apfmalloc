use lrmalloc_rs::dump_info;
use lrmalloc_rs_global::{OVERRIDE_MALLOC, OVERRIDE_ALIGNED_ALLOC, OVERRIDE_CALLOC};

#[test]
fn test() {
    let u = Box::new(15);

    assert_eq!(*u, 15);
    unsafe {

        assert!(OVERRIDE_ALIGNED_ALLOC || OVERRIDE_MALLOC || OVERRIDE_CALLOC);
    }
}


