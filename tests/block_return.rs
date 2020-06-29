use lrmalloc_rs::allocation_data::get_heaps;
use lrmalloc_rs::{do_free, do_malloc, dump_info};
use std::sync::atomic::Ordering;
use std::thread;

#[test]
fn threads_return_extra_to_heap() {
    let heaps = get_heaps().get_heap_at(1);
    assert!(heaps.partial_list.load(Ordering::Acquire).is_none());
    let handle = thread::spawn(move || {
        let ret = do_malloc(8);
        assert!(heaps.partial_list.load(Ordering::Acquire).is_none());
        unsafe { &*ret }
    });

    let ptr = handle.join().expect("Didn't acquire a pointer");
    unsafe {
        do_free(ptr);
    }
    let new_ptr = unsafe { &*do_malloc(8) };
    assert_eq!(new_ptr as *const u8, ptr as *const u8);
    dump_info!();
}
