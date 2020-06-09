extern crate lrmalloc_rs_global;

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

    let collect = (0..20).map(|n| fast_fib(n)).collect::<Vec<usize>>();
    fn merge_sort(input: Vec<usize>) {

    }
}
