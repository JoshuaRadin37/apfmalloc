use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_free, do_malloc};
use std::io::{stdout, Write};

const ALLOCATIONS: usize = 100_000; // _000;
const MAX_ALLOCATION_SIZE: usize = 2048;
const ALLOCATION_BYTES: usize = 512 / 8;

#[test]
#[ignore]
fn allocation_hell() {
    let range = (3..(MAX_ALLOCATION_SIZE as f64).log(2.0) as usize);
    let total_allocations = (range.end - range.start) * ALLOCATIONS;
    print!(
        "Total Allocations to perform = {} [{} bytes -> {} bytes]",
        total_allocations,
        1 << range.start,
        1 << range.end
    );
    stdout().flush();
    for size in range.map(|shift| 1 << shift) {
        print!("[{:3?}%]", 0);
        for i in 0..(ALLOCATIONS / 100) {
            let mut ptr = do_malloc(size);
            do_free(ptr);
            print!(
                "\u{8}\u{8}\u{8}\u{8}\u{8}\u{8}[{:3?}%]",
                (i as f64 / ALLOCATIONS as f64 * 100.0) as usize
            );
            stdout().flush();
        }
        for i in 0..(ALLOCATIONS - ALLOCATIONS / 100) {
            let mut ptr = do_malloc(size);
            do_free(ptr);
            print!(
                "\u{8}\u{8}\u{8}\u{8}\u{8}\u{8}[{:3?}%]",
                ((i + ALLOCATIONS / 100) as f64 / ALLOCATIONS as f64 * 100.0) as usize
            );
            stdout().flush();
        }
        print!("\u{8}\u{8}\u{8}\u{8}\u{8}\u{8}.");
        stdout().flush();
    }
    println!(" done");
}
