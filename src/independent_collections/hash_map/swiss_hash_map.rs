use std::collections::hash_map::RandomState;
use std::hash::{Hash, BuildHasher, Hasher};
use crate::independent_collections::Array;
use crate::independent_collections::array::RawArray;
use std::ptr::NonNull;


#[repr(u8)]
#[derive(Debug, PartialEq)]
enum Control {
    Empty,
    Deleted,
    Full
}

impl From<u8> for Control {
    fn from(c: u8) -> Self {
        match c {
            0b1111_1111 => Control::Empty,
            0b1000_0000 => Control::Deleted,
            c if c >> 7 == 0 => Control::Full,
            c => {
                panic!("Invalid Control Byte: {}", c)
            }
        }
    }
}


pub struct SwissHashMap<K : Eq + Hash, V> {
    random_state: RandomState,
    ctrl: Array<u8>,
    data: RawArray<(K, V)>,
    growth_left: usize,
    capacity: usize,
    len: usize,
}

struct SwissHash { h1: usize, h2: u8 }

impl<K: Eq + Hash, V> SwissHashMap<K, V> {

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            random_state: Default::default(),
            ctrl: Array::of_size(capacity),
            data: RawArray::with_capacity(capacity),
            growth_left: capacity,
            capacity: capacity,
            len: 0
        }
    }

    fn get_hash(&self, key: &K) -> u64 {
        let mut hasher = self.random_state.build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn get_swiss_hash(&self, key: &K) -> SwissHash {
        let hash = self.get_hash(key);
        let control = (hash >> (64 - 7)) as u8;
        let h1 = (hash as usize) % self.capacity;
        SwissHash { h1, h2: control }
    }

    unsafe fn find<F : Fn(&K) -> bool>(&self, hash: SwissHash, eq: F) -> Option<NonNull<(K, V)>> {
        let mut point = hash.h1;
        while Control::from(self.ctrl[point]) == Control::Full {
            let ctrl = self.ctrl[point];
            if ctrl != hash.h2 {
                break;
            } else {
                let ptr = &self.data[point];
                let (key, _) = ptr;
                if eq(key) {
                    return NonNull::new(ptr as *const _ as *mut _)
                }
            }
            point += 1;
        }
        None
    }

    pub fn insert(&mut self, key: K, val: V) {
        let hash = self.get_swiss_hash(&key);
        unsafe {
            if let Some(mut ptr) = self.find(hash, |other| &key == other) {
                let bucket = (key, val);
                *ptr.as_mut() = bucket;
            }
        }
        
    }
}

