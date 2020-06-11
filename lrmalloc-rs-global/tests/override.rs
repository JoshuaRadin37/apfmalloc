
#[test]
fn test() {
    let u = Box::new(15);

    assert_eq!(*u, 15);
    unsafe {
        use lrmalloc_rs_global::OVERRIDE_MALLOC;
        assert!(OVERRIDE_MALLOC);
    }
}

#[test]
fn im_a_test() {
    let x = 3;
    assert_ne!(x, 4, "3 and 4 should not be equal");
}