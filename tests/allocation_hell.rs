use lrmalloc_rs::ptr::auto_ptr::AutoPtr;
use lrmalloc_rs::{do_malloc, do_free};

const ALLOCATIONS: usize = 1_000_000; // _000;
const MAX_ALLOCATION_SIZE: usize = 512;
const ALLOCATION_BYTES: usize = 512 / 8;

#[test]
fn allocation_hell() {
    let range = (0..(MAX_ALLOCATION_SIZE as f64).log(2.0) as usize);
    let total_allocations =  (range.end - range.start) * ALLOCATIONS;
    print!("Total Allocations to perform = {}... ", total_allocations);

    for size in range.map(|shift| 1 << shift) {
        for _ in 0..ALLOCATIONS {
            let mut ptr = do_malloc(size);
            do_free(ptr);
        }
    }
    println!("done");

}