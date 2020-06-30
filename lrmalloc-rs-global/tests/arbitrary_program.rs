extern crate lrmalloc_rs_global;

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::sync::{RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};

// ~O(2^n)
fn slow_fib(n: usize) -> Box<usize> {
    match n {
        0 => Box::new(0),
        1 => Box::new(1),
        n => Box::new(*slow_fib(n - 1) + *slow_fib(n - 2)),
    }
}

// O(n)
fn fast_fib(n: usize) -> usize {
    let mut saved = vec![];

    for i in 0..=n {
        let guard = &mut saved;
        if guard.len() <= i {
            if i < 2 {
                guard.push(i)
            } else {
                let n_1 = guard[i - 1];
                let n_2 = guard[i - 2];
                guard.push(n_1 + n_2);
            }
        } else {
            break;
        }

    }
    saved[n]
}

fn memory_fib(n: usize) -> usize {
    lazy_static::lazy_static! {
        static ref MEMORY: RwLock<Vec<usize>> = RwLock::new(Vec::new());
    }
    static MEMORY_LENGTH: AtomicUsize = AtomicUsize::new(0);

    if n >= MEMORY_LENGTH.load(Ordering::Acquire) {
        let mut memory = MEMORY.write().unwrap();
        while MEMORY_LENGTH.load(Ordering::Acquire) < n + 1 {
            let before_length = MEMORY_LENGTH.load(Ordering::Acquire);
            let fib = if before_length >= 2 {
                let n_1 = memory[before_length - 1];
                let n_2 = memory[before_length - 2];
                n_1 + n_2
            } else {
                before_length
            };

            memory.push(fib);
            MEMORY_LENGTH.compare_and_swap(before_length, before_length + 1, Ordering::Release);
        }
        memory[n]
    } else {
        let memory = MEMORY.read().unwrap();
        *memory.get(n).unwrap()
    }
}

#[test]
fn fast_fib_no_fail_global() {
    for n in (0..10).rev() {
        let fib = fast_fib(n);
        assert_eq!(
            fib,
            *slow_fib(n),
            "fast_fib({}) gave the wrong result",
            n
        );
        assert_eq!(
            memory_fib(n),
            fib,
            "memory_fib({}) gave the wrong result",
            n
        )
    }

    assert!(unsafe { lrmalloc_rs_global::OVERRIDE_MALLOC || lrmalloc_rs_global::OVERRIDE_ALIGNED_ALLOC })
}

#[test]
fn arbitrary_program_main() {
    const SIZE: usize = 64;
    let mut rng = thread_rng();
    let mut collect = (0..SIZE).map(|n| memory_fib(n)).collect::<Vec<usize>>();


    //collect.reverse();
    collect.shuffle(&mut rng);
    fn merge_sort<T: PartialOrd>(input: &mut Vec<T>) {
        fn wmerge<T : PartialOrd>(xs: &mut [T], mut i: usize, m: usize, mut j: usize, n: usize, mut w: usize) {
            while i < m && j < n {
                let index = if xs[i] < xs[j] {
                    let ret = i;
                    i += 1;
                    ret
                } else {
                    let ret = j;
                    j += 1;
                    ret
                };
                xs.swap(w, index);
                w += 1;
            }

            while i < m {
                xs.swap(w, i);
                w += 1;
                i += 1;
            }

            while j < n {
                xs.swap(w, j);
                w += 1;
                j += 1;
            }
        }

        fn wsort<T : PartialOrd>(xs: &mut [T], mut l: usize, u: usize, mut w: usize) {
            if u - l > 1 {
                let m = l + (u - l) / 2;
                imsort(xs, l, m);
                imsort(xs, m, u);
                wmerge(xs, l, m, m, u, w);
            } else {
                while l < u {
                    xs.swap(l, w);
                    l += 1;
                    w += 1;
                }
            }
        }

        fn imsort<T : PartialOrd>(xs: &mut [T], l: usize, u: usize) {
            if u - l > 1 {
                let m = l + (u - l) / 2;
                let mut w = l + u - m;
                wsort(xs, l, m, w);
                while w - l > 2 {
                    let n = w;
                    w = l + (n - l + 1) / 2;
                    wsort(xs, w, n, l);
                    wmerge(xs, l, l + n - w, n, u, w);
                }
                let mut n = w;
                while n > l {
                    let mut m = n;
                    while m < u && xs[m] < xs[m-1] {
                        xs.swap(m, m-1);

                        m += 1;
                    }

                    n -= 1;
                }
            }
        }
        let len = input.len();
        imsort(input, 0, len)
    }
    merge_sort(&mut collect);

    assert_eq!(
        collect,
        (0..SIZE).map(|n| memory_fib(n)).collect::<Vec<usize>>()
    );
}
