use apfmalloc_lib::ptr::auto_ptr::AutoPtr;
use std::thread;
use std::sync::Arc;
use spin::Mutex;

#[test]
fn single_thread() {

    let ptr = Arc::new(Mutex::new(AutoPtr::new(0usize)));
    let clone = ptr.clone();
    thread::spawn(move || {
        let mut ptr = clone.lock();
        *ptr = AutoPtr::new(16usize);
    }).join().unwrap();
    assert_eq!(**ptr.lock(), 16usize);
}