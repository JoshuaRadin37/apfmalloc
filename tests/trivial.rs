use bitfield::size_of;
use core::ptr::null_mut;
use apfmalloc_lib::{do_free, do_malloc};
use std::mem::MaybeUninit;
use std::thread;

#[test]
fn create_and_destroy() {
    unsafe {
        let o = do_malloc(size_of::<Option<usize>>()) as *mut MaybeUninit<Option<usize>>;
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

mod mass_stress {
    use super::*;
    use apfmalloc_lib::ptr::auto_ptr::AutoPtr;

    #[test]
    fn mass_thread_spawn_stress() {
        for _j in 0..50 {
            let mut vec = vec![];
            for _ in 0..8 {
                vec.push(thread::spawn(move || {
                    let i = AutoPtr::new(0xdeadbeafusize);
                    *i
                }));
            }
            for join in vec {
                match join.join() {
                    Ok(val) => {
                        assert_eq!(val, 0xdeadbeaf);
                    }
                    Err(e) => {
                        if let Some(e) = e.downcast_ref::<&'static str>() {
                            panic!("Received error: {}", e);
                        } else {
                            panic!("Received unknown error: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    #[test]
    // #[ignore]
    fn mass_thread_spawn_stress_comparison() {
        for _j in 0..50 {
            let mut vec = vec![];
            for _ in 0..8 {
                vec.push(thread::spawn(move || {
                    Box::new(0usize)
                    //println!("Thread {} says hello", j * 8 + i)
                }));
            }
            for join in vec {
                join.join().unwrap();
            }
        }
    }

    #[test]
    fn mass_thread_allocate_stress() {
        for _ in 0..8 {
            let mut vec = vec![];

            vec.push(thread::spawn(move || {
                let mut vec = vec![];
                for _j in 0..500000 {
                    vec.push(AutoPtr::new(3799i16))
                    //println!("Thread {} says hello", j * 8 + i)
                }
                vec
            }));

            for join in vec {
                let _v = join.join().unwrap();
            }
        }
    }

    #[test]
    // #[ignore]
    fn mass_thread_allocate_stress_comparison() {
        for _ in 0..8 {
            let mut vec = vec![];

            vec.push(thread::spawn(move || {
                let mut vec = vec![];
                for _j in 0..500000 {
                    vec.push(Box::new(3799i16))
                    //println!("Thread {} says hello", j * 8 + i)
                }
                vec
            }));

            for join in vec {
                let _v = join.join().unwrap();
            }
        }
    }
}
