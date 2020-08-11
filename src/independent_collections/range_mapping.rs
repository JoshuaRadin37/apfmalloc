//! This a range based binary tree, meant to track the page info for pointers without taking exactly
//! 2 TB of space required by the original implementation of the page map

use crate::page_map::PageInfo;
use crate::independent_collections::Array;
use std::ptr::null_mut;
use std::ops::RangeInclusive;
use crate::allocation_data::{ProcHeap, Descriptor};
use crate::mem_info::{PAGE_MASK, PAGE};
use std::cell::RefCell;

struct Node {
    range: RangeInclusive<usize>,
    inner: NodeInner
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.range == other.range
    }
}

impl Node {

    fn create_parent_node(&self, other: &Self) -> Self {
        let new_range = *self.range.start().min(other.range.start())..=*self.range.end().max(other.range.end());
        let (less, more) = if self.range.end() < other.range.start() {
            (self as *const Node as *mut Node, other as *const Node as *mut Node)
        } else {
            (other as *const Node as *mut Node, self as *const Node as *mut Node)
        };

        Node {
            range: new_range,
            inner: NodeInner::Children { less, more }
        }
    }

    pub fn find_info_for_ptr<T : ?Sized>(&self, ptr: * const T) -> Option<&PageInfo> {
        self.find_info(ptr as *mut u8 as usize)
    }

    fn find_info(&self, address: usize) -> Option<&PageInfo> {
        if self.range.contains(&address) {
            match &self.inner {
                NodeInner::Info(info) => {
                    Some(info)
                },
                NodeInner::Children { less, more } => {
                    unsafe {
                        let less = & **less;
                        let more = & **more;

                        if less.range.contains(&address) {
                            less.find_info(address)
                        } else if more.range.contains(&address) {
                            more.find_info(address)
                        } else {
                            None
                        }
                    }
                },
            }
        } else {
            None
        }
    }

    pub fn depth(&self) -> usize {
        match &self.inner {
            NodeInner::Info(_) => {
                1
            },
            NodeInner::Children { less, more } => {
                unsafe {
                    (**less).depth().max((**more).depth()) + 1
                }
            },
        }
    }

    pub unsafe fn reachable(&self, node: &Node) -> bool {
        if self == node {
            true
        } else if let NodeInner::Children { less, more } = &self.inner {
            (**less).reachable(node) || (**more).reachable(node)
        } else {
            false
        }
    }
}

enum NodeInner {
    Info(PageInfo),
    Children { less: *mut Node, more: *mut Node }
}

/// A struct that is able to store the information of Page mappings by storing the ranges of pointers
/// The objective of this struct is to be able to get the [`PageInfo`] of a pointer in `O(log n)` time, where
/// `n` is the total amount of super blocks allocated
///
/// # Example
/// ```
/// use apfmalloc_lib::independent_collections::PageRangeMapping;
/// use apfmalloc_lib::allocation_data::{get_heaps, desc::Descriptor};
/// use apfmalloc_lib::pages::{SEGMENT_ALLOCATOR, SegAllocator};
/// let mut page_map = PageRangeMapping::new();
/// // Optional
/// page_map.init_with_capacity(100);
/// let heap = get_heaps().get_heap_at_mut(1);
/// let ptr = SEGMENT_ALLOCATOR.allocate(4096).unwrap().get_ptr() as *mut u8;
/// let desc = unsafe { &mut *Descriptor::alloc() };
/// page_map.update_page_map(Some(heap), ptr, Some(desc), 1);
/// ```
///
/// [`PageInfo`]: ../../page_map/struct.PageInfo.html
pub struct PageRangeMapping {
    head: *mut Node,
    memory_array: RefCell<Array<Array<Node>>>,
    array_num: RefCell<usize>
}

unsafe impl Sync for PageRangeMapping { }
unsafe impl Send for PageRangeMapping { }

impl PageRangeMapping {

    /// Creates a new `PageRangeMapping` with no capacity
    pub const fn new() -> Self {
        PageRangeMapping {
            head: null_mut(),
            memory_array: RefCell::new(Array::new()),
            array_num: RefCell::new(0)
        }
    }

    fn must_add_arrays(&self) -> bool {
        if self.memory_array.borrow().is_empty(){
            true
        } else if *self.array_num.borrow() == self.capacity() {
            let borrow = self.memory_array.borrow();
            let last_array = borrow.get(*self.array_num.borrow()).unwrap();
            last_array.capacity() == last_array.len()
        } else {
            false
        }
    }


    fn heapify(&self, node: Node) -> *mut Node {
        let array_num = {
            *self.array_num.borrow()
        };
        if self.must_add_arrays() {
            let last_len = self.len();
            self.grow();
            for _ in last_len..=self.capacity() {
                self.memory_array.borrow_mut().push(Array::with_capacity(PAGE / std::mem::size_of::<PageInfo>()))
            }
            *self.array_num.borrow_mut() = last_len;
        } else if let Some(inner_working_set) = self.memory_array.borrow().get(array_num) {
            if inner_working_set.len() == inner_working_set.capacity() {
                *self.array_num.borrow_mut() += 1;
            }
        }

        let mut borrow = self.memory_array.borrow_mut();
        if let Some(working_set) = borrow.get_mut(*self.array_num.borrow()) {
            working_set.push(node);
            working_set.last_mut().unwrap() as *mut Node
        } else {
            null_mut()
        }
    }

    /// Grows the capacity of the backing array to `(self.capacity() + 1) * 2`
    pub fn grow(&self) -> usize {
        let new_capacity = (self.capacity() + 1) * 2;
        self.memory_array.borrow_mut().reserve(new_capacity);
        self.capacity()
    }

    pub fn init_with_capacity(&mut self, capacity: usize) {
        self.memory_array = RefCell::new(Array::with_capacity(capacity / 2 - 2));
    }

    fn create_range_node(&mut self, range: RangeInclusive<usize>, info: PageInfo) -> *mut Node {
        let node = Node {
            range,
            inner: NodeInner::Info(info)
        };

        self.heapify(node)
    }

    pub fn get_page_info<T : ?Sized>(&self, ptr: *const T) -> Option<&PageInfo> {
        if self.head.is_null() {
           return None;
        }
        let head = unsafe { & *self.head };
        head.find_info_for_ptr(ptr)
    }

    #[inline]
    pub fn set_page_info<T>(&mut self, ptr: *const T, info: PageInfo) {
        self.set_page_info_for_ptr(ptr as *mut u8, info)
    }

    unsafe fn insert_node(&mut self, node: *mut Node) {
        let _depth = self.depth();
        let _len = self.node_len();
        let _ranges = self.ranges_stored();
        if self.head.is_null() {
            self.head = node;
            return;
        }


        //let mut last = null_mut();
        let mut ptr = self.head.clone();
        let mut last = (&mut self.head) as *mut *mut Node;

        let main_range = & (*node).range;
        while let NodeInner::Children { less, more } = &mut (*ptr).inner {
            let less_range = &(**less).range;
            let more_range = &(**more).range;
            if main_range.end() < less_range.start() {
                // less than less
                (*ptr).range = *main_range.start()..=*(*ptr).range.end();
                ptr = less.clone();
                last = less;

                break;
            } else if less_range.start() < main_range.start() &&
                    less_range.end() > main_range.end() {
                // in less
                ptr = less.clone();
                last = less;
            } else if main_range.start() > more_range.end() {
                // more than more
                (*ptr).range = *(*ptr).range.start()..=*main_range.end();
                ptr = more.clone();
                last = more;

                break;
            } else if more_range.start() < main_range.start() &&
                    main_range.end() > main_range.end() {
                // in more
                ptr = more.clone();
                last = more;

            } else {
                // in the middle
                // lets force it to the most

                // (*ptr).range = *main_range.start()..=*more_range.end();
                ptr = more.clone();
                last = more;

                break;
            }
        }

        let combined = (*node).create_parent_node(&*ptr);
        /*
        self.memory_array.push(combined);
        let created_ptr = self.memory_array.last().unwrap() as *const Node;

         */

        let x = self.heapify(combined);
        *last = x;
        assert!((*self.head).reachable(&*x));
        //ptr.write(combined);
    }




    fn set_page_info_for_ptr(&mut self, ptr: *mut u8, info: PageInfo) {
        let range = (ptr as usize)..=(ptr as usize);
        let node = self.create_range_node(range, info);
        unsafe {
            self.insert_node(node);
        }
    }

    pub fn update_page_map(
        &mut self,
        heap: Option<&mut ProcHeap>,
        ptr: *mut u8,
        desc: Option<&mut Descriptor>,
        size_class_index: usize,
    ) {
        let mut info = PageInfo::default();
        info.set_ptr(desc.map_or(null_mut(), |d| d as *mut Descriptor),
                     size_class_index,);

        if heap.is_none() {
            self.set_page_info_for_ptr(ptr, info);
            return;
        }

        let heap = heap.unwrap();
        let sb_size = heap.get_size_class().sb_size as usize;
        assert_eq!(
            sb_size & PAGE_MASK,
            0,
            "sb_size must be a multiple of a page"
        );
        let range = (ptr as usize)..=(ptr as usize + sb_size - 1);
        let node = self.create_range_node(range, info);
        unsafe {
            self.insert_node(node);
            assert!((*self.head).reachable(&*node));
        }
    }

    pub fn capacity(&self) -> usize {
        self.memory_array.borrow().capacity()
    }

    pub fn node_capacity(&self) -> usize {
        let mut sum = 0;
        for arr in & *self.memory_array.borrow() {
            sum += arr.capacity();
        }
        sum
    }

    pub fn len(&self) -> usize {
        self.memory_array.borrow().len()
    }

    pub fn node_len(&self) -> usize {
        let mut sum = 0;
        for arr in & *self.memory_array.borrow() {
            sum += arr.len();
        }
        sum
    }

    pub fn ranges_stored(&self) -> usize {
        let mut sum = 0;
        for arr in & *self.memory_array.borrow() {
            for node in arr {
                if let NodeInner::Info(_) = node.inner {
                    sum += 1;
                }
            }
        }
        sum
    }

    pub fn address_range(&self) -> Option<&RangeInclusive<usize>> {
        if self.head.is_null() {
            None
        } else {
            unsafe {
                Some(&(*self.head).range)
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn depth(&self) -> usize {
        if !self.head.is_null() {
            unsafe {
                (*self.head).depth()
            }
        } else {
            0
        }
    }


}

#[cfg(test)]
mod test {

    #[test]
    fn collection_grows() {

    }
}


