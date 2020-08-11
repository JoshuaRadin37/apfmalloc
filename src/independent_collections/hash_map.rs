use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::hash::{BuildHasher, Hash, Hasher};
use std::hash::BuildHasherDefault;
use std::iter::Iterator;
use std::ops::{Index, IndexMut};

use crate::independent_collections::Array;

pub(super) mod sync_hash_map;
pub(super) mod swiss_hash_map;

struct Bucket<K, V>
where
    K: Eq + Hash,
{
    hash: u64,
    key: K,
    value: V,
}

struct HashMapInner<K, V>
where
    K: Eq + Hash,
{
    buckets: Array<Array<Bucket<K, V>>>,
}

impl<K, V> HashMapInner<K, V>
where
    K: Eq + Hash,
{
    fn get_hash<H>(&self, mut hasher: H, key: &K) -> u64
    where
        H: Hasher,
    {
        key.hash(&mut hasher);
        let ret = hasher.finish();
        ret % self.buckets.len() as u64
    }
}

pub struct HashMap<K, V>
where
    K: Eq + Hash,
{
    hash: BuildHasherDefault<DefaultHasher>,
    inner: HashMapInner<K, V>,
    containers_used: usize,
    len: usize,
}

impl<K: Eq + Hash, V> Debug for HashMap<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(size = {})", self.len())
    }
}

impl<K, V> HashMap<K, V>
where
    K: Eq + Hash,
{
    pub fn new() -> Self {
        static INITIAL_CAPACITY: usize = 1001;
        Self::with_capacity(INITIAL_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let buckets = Array::of_size(capacity);
        Self {
            hash: Default::default(),
            inner: HashMapInner { buckets: buckets },
            containers_used: 0,
            len: 0
        }
    }

    fn spread(&self) -> f64 {
        if self.len == 0 {
            f64::NAN
        } else {
            self.containers_used as f64 / self.len as f64
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn get_hash(&self, key: &K) -> u64 {
        self.inner.get_hash(self.hash.build_hasher(), key)
    }

    fn get_rehash(&self, key: &K, new_capacity: usize) -> u64 {
        let mut hasher = self.hash.build_hasher();
        key.hash(&mut hasher);
        let ret = hasher.finish();
        ret % new_capacity as u64
    }

    fn grow(&mut self) {

        let new_array = Array::of_size_using(|| Array::new(), self.inner.buckets.len() * 2 + 1);
        let new_capacity = self.inner.buckets.len() * 2 + 1;
        /*
        for _ in 0..new_capacity {
            new_array.push(Array::new())
        }

         */

        let old = std::mem::replace(&mut self.inner, HashMapInner { buckets: new_array });

        self.containers_used = 0;

        for old_buckets in old.buckets {
            for mut bucket in old_buckets {
                let new_hash = self.get_rehash(&bucket.key, new_capacity);
                bucket.hash = new_hash;
                let index = new_hash as usize;
                let array = &mut self.inner.buckets[index];
                if array.is_empty() {
                    self.containers_used += 1;
                }
                array.push(bucket);
            }
        }

    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let (ret, _) = self.insert_keep_key(key, value);
        ret
    }

    pub fn insert_keep_key(&mut self, key: K, value: V) -> (Option<V>, &K) {
        {
            if self.len() >= self.inner.buckets.len() / 2 && self.spread() < 0.5
                || self.len() == self.inner.buckets.len() - 1
            {
                self.grow();
            }
        }
        let hash = self.get_hash(&key);
        let buckets = &mut self.inner.buckets[hash as usize];
        if buckets.len() == 0 {
            self.containers_used += 1;
        }

        let mut old_index = None;
        let iterator = Array::iter(&buckets);
        let enumerate = iterator.enumerate();
        for (index, bucket) in enumerate {
            if bucket.key.eq(&key) {
                old_index = Some(index);
            }
        }

        self.len += 1;
        let ret = match old_index {
            None => {
                let bucket = Bucket { hash, key, value };
                buckets.push(bucket);
                (None, &buckets.last().unwrap().key)
            }
            Some(old_index) => {
                let bucket = &mut buckets[old_index];
                let val = std::mem::replace(&mut bucket.value, value);
                (Some(val), &bucket.key)
            }
        };
        ret
    }

    /// Inserts the key value pair only if the key was already present in the map
    pub fn replace(&mut self, key: K, value: V) -> Result<V, ()> {
        let hash = self.get_hash(&key);
        let buckets = &mut self.inner.buckets[hash as usize];
        let mut old_index = None;
        for (index, bucket) in buckets.iter().enumerate() {
            if bucket.key.eq(&key) {
                old_index = Some(index);
            }
        }

        let ret = match old_index {
            None => Err(()),
            Some(old_index) => {
                let bucket = &mut buckets[old_index];
                let val = std::mem::replace(&mut bucket.value, value);
                Ok(val)
            }
        };
        ret
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = self.get_hash(key);
        let buckets = &self.inner.buckets[hash as usize];
        for bucket in buckets.iter() {
            if bucket.key.eq(key) {
                return Some(&bucket.value);
            }
        }
        None
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let hash = self.get_hash(key);
        let buckets = &mut self.inner.buckets[hash as usize];
        for bucket in buckets.iter_mut() {
            if bucket.key.eq(key) {
                return Some(&mut bucket.value);
            }
        }
        None
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = self.get_hash(&key);

        let buckets = &mut self.inner.buckets[hash as usize];
        let mut old_index = None;
        for (index, bucket) in buckets.iter().enumerate() {
            if bucket.key.eq(&key) {
                old_index = Some(index);
            }
        }
        self.len -= 1;
        let ret = match old_index {
            None => None,
            Some(index) => {
                let bucket = buckets.remove(index).unwrap();
                Some(bucket.value)
            }
        };

        ret
    }

    pub fn contains(&self, key: &K) -> bool {
        let hash = self.get_hash(&key);
        let buckets = &self.inner.buckets[hash as usize];
        for bucket in buckets.iter() {
            if bucket.key.eq(&key) {
                return true;
            }
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }



}

impl <'a, K: Hash + Eq + Clone, V> HashMap<K, V> {
    pub fn entry(&mut self, key: K) -> HashMapEntry<'_, K, V> {
        HashMapEntry::get_from_map(self, key)
    }
}

enum HashMapEntryInner {
    Present { bucket: usize, index: usize },
    NotPresent,
}

pub struct HashMapEntry<'a, K: Hash + Eq, V> {
    map: &'a mut HashMap<K, V>,
    key: K,
    entry: HashMapEntryInner,
}

impl<'a, K: Hash + Eq + Clone, V> HashMapEntry<'a, K, V> {
    fn get_from_map(map: &'a mut HashMap<K, V>, key: K) -> HashMapEntry<'a, K, V> {
        let hash = map.get_hash(&key);
        let entry = match map.inner.buckets[hash as usize]
            .iter()
            .position(|bucket| &bucket.key == &key)
        {
            None => HashMapEntryInner::NotPresent,
            Some(index) => {

                HashMapEntryInner::Present {
                    bucket: hash as usize,
                    index,
                }
            },
        };

        HashMapEntry { map, key, entry }
    }

    pub fn or_insert(self, value: V) -> &'a mut V {
        match self.entry {
            HashMapEntryInner::Present { bucket, index } => unsafe {


                &mut self
                    .map
                    .inner
                    .buckets
                    .get_unchecked_mut(bucket)
                    .get_unchecked_mut(index)
                    .value
            },
            HashMapEntryInner::NotPresent => {
                let map = self.map;
                map.insert(self.key.clone(), value);
                map.get_mut(&self.key).unwrap()
            }
        }
    }
}

impl<K: Hash + Eq, V> Index<&K> for HashMap<K, V> {
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.get(&index).expect("Key not present in map")
    }
}

impl<K: Hash + Eq, V> IndexMut<&K> for HashMap<K, V> {
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.get_mut(&index).expect("Key not present in map")
    }
}



pub struct HashSet<K: Hash + Eq>(HashMap<K, ()>);

impl<K: Hash + Eq> HashSet<K> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }

    pub fn insert(&mut self, val: K) {
        self.0.insert(val, ());
    }

    pub fn remove(&mut self, val: &K) {
        self.0.remove(val);
    }

    pub fn contains(&self, val: &K) -> bool {
        self.0.contains(val)
    }
}

#[cfg(test)]
mod test {
    use crate::independent_collections::hash_map::{HashMap, HashSet};

    #[test]
    fn insert_and_get_val() {
        let mut map = HashMap::new();
        map.insert(5, "Hello World!");
        if let Some(string) = map.get(&5) {
            assert_eq!(*string, "Hello World!");
        } else {
            panic!("Hello World should be present")
        }
    }

    #[test]
    fn grow() {
        let mut map = HashSet::with_capacity(11);
        for i in 0..15 {
            map.insert(i);
        }
        assert!(map.contains(&14))
    }

    #[test]
    fn remove_kvp() {
        let mut map = HashMap::new();
        map.insert(5, "Hello World!");
        assert!(map.contains(&5));
        assert_eq!(map.len(), 1);
        let val = map
            .remove(&5)
            .expect("If gotten here, the value must exist");
        assert!(!map.contains(&5));
        assert!(map.is_empty());
        assert_eq!(val, "Hello World!")
    }

    #[test]
    #[should_panic]
    fn illegal_access() {
        let map: HashMap<usize, usize> = HashMap::new();
        let _i = map[&15];
    }

    #[test]
    fn or_insert() {
        let mut map: HashMap<usize, usize> = HashMap::new();
        let entry = map.entry(10).or_insert(0);
        assert_eq!(entry, &mut 0);
        *entry = 10;
        assert_eq!(entry, &mut 10);
        assert_ne!(map.entry(10).or_insert(0), &mut 0);
    }
}
