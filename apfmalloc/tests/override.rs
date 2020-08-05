use apfmalloc::{OVERRIDE_ALIGNED_ALLOC, OVERRIDE_CALLOC, OVERRIDE_MALLOC};

#[test]
fn test() {
    let u = Box::new(15);

    assert_eq!(*u, 15);
    unsafe {
        assert!(OVERRIDE_ALIGNED_ALLOC || OVERRIDE_MALLOC || OVERRIDE_CALLOC);
    }
}
