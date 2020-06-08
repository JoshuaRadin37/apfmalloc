use bitfield::size_of;
use lralloc_rs::{do_free, do_malloc};
use std::mem::MaybeUninit;
use core::ptr::null_mut;
use std::thread;

#[test]
fn create_and_destroy() {
    unsafe {
        let o = (do_malloc(size_of::<Option<usize>>()) as *mut MaybeUninit<Option<usize>>);
        assert_ne!(o, null_mut());
        // println!("First allocation successful");
        *o = MaybeUninit::new(Some(15));
        let o = o as *mut Option<usize>;

        do_malloc(size_of::<[usize; 64]>());
        assert_ne!(o, null_mut());
        // println!("First allocation successful");

        do_free(o as *const Option<usize>);
    }
}

#[test]
fn mass_stress() {
    for j in 0..5000 {
        let mut vec = vec![];
        for i in 0..8 {
            vec.push(thread::spawn(move ||
                {
                    do_malloc(8);
                    //println!("Thread {} says hello", j * 8 + i)
                }));
        }
        for join in vec {
            join.join();
        }
    }
}