use core::ops::DerefMut;
use std::collections::{HashMap, LinkedList, VecDeque};
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

extern crate lrmalloc_rs_global;

struct HashedStack<T, H: Hash + Eq = T> {
    stack: VecDeque<T>,
    map: HashMap<H, usize>,
}

impl<T, H: Hash + Eq> HashedStack<T, H> {
    pub fn new() -> Self {
        Self {
            stack: Default::default(),
            map: Default::default(),
        }
    }

    pub fn push_hashed(&mut self, hash: H, val: T) {
        let pos = self.stack.len();
        self.map.insert(hash, pos);
        self.stack.push_back(val)
    }

    pub fn peak(&self) -> Option<&T> {
        self.stack.back()
    }

    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop_back()
    }

    pub fn remove(&mut self, hash: &H) -> Option<T> {
        let pos = { *&self.map[hash] };
        self.map.remove(hash);
        self.stack.remove(pos)
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }
}

pub type SelfHashedStack<T> = HashedStack<T, T>;

impl<T: Hash + Eq + Clone> SelfHashedStack<T> {
    pub fn push(&mut self, val: T) {
        self.push_hashed(val.clone(), val)
    }
}

fn main() {
    let mut hashed_stack = HashedStack::new();
    hashed_stack.push("Hello");
    hashed_stack.push("World");

    let value = hashed_stack.remove(&"Hello").unwrap();
    assert_eq!(value, "Hello");
    assert_eq!(hashed_stack.len(), 1);
    assert_eq!(hashed_stack.pop(), Some("World"));
    assert_eq!(hashed_stack.pop(), None);
}
