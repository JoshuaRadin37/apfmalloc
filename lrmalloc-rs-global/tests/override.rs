use lrmalloc_rs::dump_info;

#[test]
fn test() {
    let u = Box::new(15);

    assert_eq!(*u, 15);
    unsafe {
        use lrmalloc_rs_global::OVERRIDE_MALLOC;
        assert!(OVERRIDE_MALLOC);
    }
}
