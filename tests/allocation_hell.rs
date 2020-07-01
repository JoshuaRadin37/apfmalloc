use lrmalloc_rs::{do_free, do_malloc};
use std::io::{stdout, Write};

const ALLOCATIONS: usize = 10_000; // _000;
const MAX_ALLOCATION_SIZE: usize = 2048;

const FANCY_DISPLAY: bool = false;

macro_rules! fancy {
    ($ex:stmt) => {
        if $crate::FANCY_DISPLAY {
            $ex
        }
    };
    ($bl:block) => {
        if $crate::FANCY_DISPLAY $bl
    };
}

#[test]
fn allocation_hell() {
    let range = 3..(MAX_ALLOCATION_SIZE as f64).log(2.0) as usize;
    let total_allocations = (range.end - range.start) * ALLOCATIONS;
    fancy!(print!(
        "Total Allocations to perform = {} [{} bytes -> {} bytes]",
        total_allocations,
        1 << range.start,
        1 << range.end
    ));

    unsafe {
        fancy!(stdout().flush().unwrap());
        for size in range.map(|shift| 1 << shift) {
            fancy!(print!("[{:3?}%]", 0));
            for i in 0..(ALLOCATIONS / 100) {
                let ptr = do_malloc(size);
                do_free(ptr);
                fancy!(print!(
                    "\u{8}\u{8}\u{8}\u{8}\u{8}\u{8}[{:3?}%]",
                    (i as f64 / ALLOCATIONS as f64 * 100.0) as usize
                ));
                fancy!(stdout().flush().unwrap());
            }
            for i in 0..(ALLOCATIONS - ALLOCATIONS / 100) {
                let ptr = do_malloc(size);
                do_free(ptr);
                fancy!(print!(
                    "\u{8}\u{8}\u{8}\u{8}\u{8}\u{8}[{:3?}%]",
                    ((i + ALLOCATIONS / 100) as f64 / ALLOCATIONS as f64 * 100.0) as usize
                ));
                fancy!(stdout().flush().unwrap());
            }
            fancy!(print!("\u{8}\u{8}\u{8}\u{8}\u{8}\u{8}."));
            fancy!(stdout().flush().unwrap());
        }
    }
    fancy!(println!(" done"));
}
