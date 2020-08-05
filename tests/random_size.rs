use apfmalloc_lib::{do_free, do_malloc};
use rand::{thread_rng, Rng};

const ALLOCATIONS: usize = 10_000; // _000;
const MAX_ALLOCATION_SIZE: usize = 8;

#[test]
fn random_size_test() {
    let mut vec = vec![];
    let mut rand = thread_rng();

    for _ in 0..ALLOCATIONS {
        let size = rand.gen_range(0, MAX_ALLOCATION_SIZE);
        println!("size = {}", size);
        let ptr = do_malloc(size);
        assert!(!ptr.is_null());
        vec.push(ptr);
    }

    unsafe {
        for ptr in vec {
            do_free(ptr);
        }
    }
}
