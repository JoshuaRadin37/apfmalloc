extern crate lrmalloc_rs_global;

use std::collections::{HashMap, BTreeMap};

// ~O(2^n)
fn slow_fib(n: usize) -> usize {
    match n {
        0 => 0,
        1 => 1,
        n => slow_fib(n - 1) + slow_fib(n - 2),
    }
}

// O(n)
fn fast_fib(n: usize) -> usize {
    let mut saved = vec![0usize, 1];

    for i in 2..=n {
        saved.push(
            saved[i - 1] + saved[i - 2]
        );
    }

    saved[n]
}

#[test]
fn fast_fib_no_fail() {
    for n in 0..10 {
        assert_eq!(fast_fib(n), slow_fib(n), "fast_fib({}) gave the wrong result", n);
    }
}

#[test]
fn arbitrary_program_main() {
    const SIZE:usize  = 4;
    let mut collect = (0..SIZE).map(|n| fast_fib(n)).collect::<Vec<usize>>();
    collect.reverse();
    fn merge_sort<T : PartialOrd>(input: &mut Vec<T>) {
        fn merge_sort_helper<T : PartialOrd>(input: &mut [T], from: usize, to: usize) {
            let mid = (from + to) / 2;
            if mid == from {
                return;
            }
            merge_sort_helper(input, from, mid);
            merge_sort_helper(input, mid, to);
            let left = &input[from..mid];
            let right = &input[mid..to];

            let mut i = 0;
            let mut j = 0;
            let total = to - from;
            // let mut fixed = vec![];
            let mut mapping = BTreeMap::new();
            for _ in 0..total {
                if i == mid - from {
                    mapping.insert(mid + j, from + j + i);
                    continue;
                } else if j == to - mid {
                    mapping.insert(from + i, from + i + j);
                    continue;
                }
                let left_item = &left[i];
                let right_item = &right[j];

                if left_item <= right_item {
                    mapping.insert(from + i, from + i + j);
                    i += 1;
                } else {
                    mapping.insert(mid + j, from + j + i);
                    j += 1;
                }
            }
            for _ in 0..total {
                let one = *mapping.keys().map(|n| *n).collect::<Vec<usize>>().first().unwrap();

                let swap = mapping[&one];
                input.swap(one, swap);
                mapping.remove(&one);
                if one != swap {
                    let next = mapping[&swap];
                    mapping.insert(one, next);
                    mapping.remove(&swap);
                }
            }

        }
        let len = input.len();
        merge_sort_helper(input, 0, len)
    }
    merge_sort(&mut collect);

    assert_eq!(collect, (0..SIZE).map(|n| fast_fib(n)).collect::<Vec<usize>>());
}
