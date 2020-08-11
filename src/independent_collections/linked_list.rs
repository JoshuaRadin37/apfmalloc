use crate::ptr::auto_ptr::AutoPtr;
use crate::independent_collections::array::sync_array::IndependentAllocator;
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};
use std::ptr::NonNull;
use std::marker::PhantomData;
use std::fmt::Debug;
use std::fmt::Formatter;

#[derive(Debug)]
struct Link<T> {
    data: T,
    next: Option<AutoPtr<Link<T>, IndependentAllocator>>,
}

impl<T> Link<T> {

    fn new(data: T) -> Self {
        Self {
            data: data,
            next: None
        }
    }

    fn get_data(&self) -> &T {
        &self.data
    }

    fn get_data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    fn get_next(&self) -> & Option<AutoPtr<Link<T>, IndependentAllocator>> {
        &self.next
    }

    fn get_next_mut(&mut self) -> &mut Option<AutoPtr<Link<T>, IndependentAllocator>> {
        &mut self.next
    }

    fn remove_next(&mut self) -> Option<Link<T>> {
        let mem = std::mem::replace(&mut self.next, None);
        mem.map(|ptr| ptr.take())
    }

    fn swap_next(&mut self, new: Link<T>) -> Option<Link<T>> {
        let mem = std::mem::replace(&mut self.next, Some(AutoPtr::with_allocator(new)));
        mem.map(|ptr| ptr.take())
    }

}

/// Provides a means of having an ordered list of objects. The space for objects are allocated in the
/// heap independent of the standard allocation functions, and therefore aren't taken into account
/// for APF Tuning.
///
/// For most circumstances, using the similar [`Array`] is faster. The advantage of this structure
/// is that the data in the list is never moved
pub struct LinkedList<T> {
    length: usize,
    head: Option<Link<T>>,
}

impl<T> LinkedList<T> {

    pub fn new() -> Self {
        Self {
            length: 0,
            head: None
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }



    pub fn push_front(&mut self, val: T) {
        let node = Link::new(val);
        match &self.head {
            None => {
                self.head = Some(node);
            },
            Some(_) => {
                let old_head = std::mem::replace(&mut self.head, Some(node)).unwrap();
                let next = self.head.as_mut().unwrap().get_next_mut();
                *next = Some(AutoPtr::with_allocator(old_head))
            },
        }
        self.length += 1;
    }

    pub fn push(&mut self, val: T) {
        self.push_back(val);
    }
    pub fn push_back(&mut self, val: T) {
        if self.is_empty() {
            self.push_front(val)
        } else {
            let length = self.len();
            let mut node_ptr = self.head.as_mut().unwrap().get_next_mut();
            for _ in 0..(length - 1) {
                node_ptr = node_ptr.as_mut().unwrap().get_next_mut();
            }
            let node = Link::new(val);
            *node_ptr = Some(AutoPtr::with_allocator(node));
            self.length += 1;
        }
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        self.length -= 1;
        let next = self.head.as_mut().unwrap().remove_next();
        let ret = std::mem::replace(&mut self.head, next);
        ret.map(|link| link.data )
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        } else if self.len() == 1 {
            return self.pop_front();
        }

        let length = self.len();
        self.length -= 1;
        let mut link_ptr = self.head.as_mut();
        for _ in 1..(length - 1) {
            link_ptr = match link_ptr.map(|link| link.get_next_mut()) {
                None => None,
                Some(next) => {
                    next.as_mut().map(|ptr| &mut **ptr)
                },
            };
        }
        let ret = link_ptr.unwrap().remove_next();
        ret.map(|link| link.data )
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.length {
            return None;
        } else if index == 0 {
            match &self.head {
                None => return None,
                Some(head) => {
                    return Some(head.get_data());
                },
            }
        }

        let mut node_ptr = self.head.as_ref().unwrap().get_next();
        for _ in 1..(index) {
            node_ptr = node_ptr.as_ref().unwrap().get_next();
        }

        match node_ptr.as_ref() {
            None => None,
            Some(link) => {
                Some(link.get_data())
            },
        }

    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T>  {
        if index >= self.length {
            return None;
        } else if index == 0 {
            return match &mut self.head {
                None => None,
                Some(head) => {
                    Some(head.get_data_mut())
                },
            }
        }

        let mut node_ptr = self.head.as_mut().unwrap().get_next_mut();
        for _ in 1..(index) {
            node_ptr = node_ptr.as_mut().unwrap().get_next_mut();
        }

        match node_ptr.as_mut() {
            None => None,
            Some(link) => {
                Some(link.get_data_mut())
            },
        }

    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.length {
            return None;
        } else if index == 0 {
            return self.pop_front();
        }

        self.length -= 1;
        // Link pointer should be the link *BEFORE* the link containing the node
        let mut link_ptr = self.head.as_mut();
        for _ in 1..(index) {
            link_ptr = match link_ptr.map(|link| link.get_next_mut()) {
                None => None,
                Some(next) => {
                    next.as_mut().map(|ptr| &mut **ptr)
                },
            };
        }


        let link_ptr = link_ptr.unwrap();
        let to_be_removed = link_ptr.next.as_mut().unwrap();
        let after = to_be_removed.remove_next();

        let ret_link = match after {
            Some(after) => {
                link_ptr.swap_next(after).unwrap()
            },
            None => {
                link_ptr.remove_next().unwrap()
            }
        };
        Some(ret_link.data)
    }

    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut T> {
        self.into_iter()
    }
}

impl <S : AsRef<str>> LinkedList<S> {

    pub fn join<Separator: AsRef<str>>(self, join: Separator) -> String {
        let mut output = String::new();

        for item in self {
            if output.is_empty() {
                output = format!("{}{}{}", output, join.as_ref(), item.as_ref());
            } else {
                output = item.as_ref().to_string();
            }
        }

        output
    }
}

#[macro_export]
macro_rules! list {
    () => { $crate::independent_collections::LinkedList::new() };
    ($($item:expr),+) => {{
        let mut out = $crate::independent_collections::LinkedList::new();
        $(out.push_back($item);)*
        out
    }};
    ($item:expr; $count:expr) => {
        {
            let mut out = $crate::independent_collections::LinkedList::new();
            for _ in 0..$count {
                out.push_back($item.clone())
            }
            out
        }
    }
}

pub struct Iter<T>(LinkedList<T>);

impl<T> Iterator for Iter<T>{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}

pub struct IterMut<'a, T: 'a> {
    _list: PhantomData<&'a mut T>,
    head: Option<NonNull<Link<T>>>,
    len: usize,
}


impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        self.head.map(|link| unsafe {
            let node = &mut *link.as_ptr();
            self.len -= 1;
            if self.len > 0 {
                self.head = Some(NonNull::from(
                    node
                        .get_next_mut()
                        .as_ref()
                        .unwrap()
                ));
            }
            &mut node.data
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}


impl <T> IntoIterator for LinkedList<T> {
    type Item = T;
    type IntoIter = Iter<T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self)
    }
}

impl <'a, T> IntoIterator for &'a LinkedList<T> {
    type Item = &'a T;
    type IntoIter = Iter<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut output = list![];
        for i in 0..self.len() {
            let item = self.get(i).unwrap();
            output.push_back(item);
        }
        Iter(output)
    }
}

impl <'a, T> IntoIterator for &'a mut LinkedList<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        let ptr = match &mut self.head {
            None => None,
            Some(head) => {
                NonNull::new(head)
            },
        };
        IterMut {
            _list: Default::default(),
            head: ptr,
            len: self.length
        }
    }
}



impl<T: Clone> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        self.iter().cloned().collect()
    }
}

impl <A> FromIterator<A> for LinkedList<A> {
    fn from_iter<T: IntoIterator<Item=A>>(iter: T) -> Self {
        let mut ret = list![];
        for item in iter {
            ret.push_back(item);
        }
        ret
    }
}

impl <T> Index<usize> for LinkedList<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap_or_else(|| panic!("Index {} out of bounds", index))
    }
}

impl <T> IndexMut<usize> for LinkedList<T> {

    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap_or_else(|| panic!("Index {} out of bounds", index))
    }
}

impl <R, T : PartialEq<R>> PartialEq<LinkedList<R>> for LinkedList<T> {
    fn eq(&self, other: &LinkedList<R>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        while let (Some(item1), Some(item2)) = (self_iter.next(), other_iter.next()) {
            if item1 != item2 {
                return false;
            }
        }
        true
    }
}

impl <T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}



impl <T : Debug> Debug for LinkedList<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let list = self.iter()
            .map(|item| format!("{:?}", item))
            .collect::<LinkedList<_>>()
            .join(", ");
        write!(f, "[{}]", list)
    }
}


/*
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_back() {
        let list = list![1, 2 ,3, 4];
        assert_eq!(list.len(), 4);
        assert!(!list.is_empty());
        for i in 0..4 {
            assert_eq!(list.get(i), Some(&(i + 1)));
        }
    }

    #[test]
    fn push_front() {
        let mut list = LinkedList::new();
        list.push_front(4);
        list.push_front(3);
        list.push_front(2);
        list.push_front(1);
        assert_eq!(list.len(), 4);
        assert!(!list.is_empty());
        for i in 0..4 {
            assert_eq!(list.get(i), Some(&(i + 1)));
        }
    }

    #[test]
    fn pop_front() {
        let mut list = list![1, 2 ,3, 4];
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(3));
        assert_eq!(list.pop_front(), Some(4));
        assert!(list.is_empty())
    }

    #[test]
    fn pop_back() {
        let mut list = list![4, 3, 2, 1];
        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), Some(2));
        assert_eq!(list.pop_back(), Some(3));
        assert_eq!(list.pop_back(), Some(4));
        assert!(list.is_empty())
    }

    #[test]
    fn get() {
        let list = list![1, 2, 3, 4];
        assert_eq!(list.get(2), Some(&3));
    }

    #[test]
    fn get_mut() {
        let mut list = list![0; 4];
        {
            let item = list.get_mut(2).unwrap();
            *item = 3;
        }
        assert_eq!(list.get(2), Some(&3));
    }

    #[test]
    fn remove() {
        let mut list = list![1i32, 2, 3, 4];
        assert_eq!(list.remove(2), Some(3));
        assert_eq!(list.len(), 3);
        assert_eq!(list, list![1i32, 2, 4]);

        assert_eq!(list.remove(0), Some(1));
        assert_eq!(list, list![2, 4]);

        assert_eq!(list.remove(1), Some(4));
        assert_eq!(list, list![2]);

    }

    #[test]
    #[should_panic]
    fn out_of_bounds_panic() {
        let list = list![0i32];
        let _ = list[2];
    }

    #[test]
    fn iterator() {
        let list = list![3; 15];
        let ref_list: LinkedList<&i32> = list.iter().collect();
        assert_eq!(list.len(), ref_list.len());
        let length = list.len();
        for i in 0..length {
            assert_eq!(list[i], *ref_list[i]);
        }

    }

    #[test]
    fn mutable_iterator() {
        let mut list = list![0; 15];
        for item in &mut list {
            *item = 5;
        }
        for item in list {
            assert_eq!(item, 5);
        }
    }
}

 */