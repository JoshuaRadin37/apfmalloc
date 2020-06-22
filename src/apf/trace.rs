use crate::apf::constants::INIT_TRACE_LENGTH;
use crate::pages::page_alloc_over_commit;
use atomic::Atomic;
use std::collections::HashMap;
use std::fmt;
use std::slice::from_raw_parts_mut;
use std::vec::Vec;

/*
    Event represents allocation or free operation
    usize stores heap slot -- not sure how helpful this will be in practice, so might make it generic
*/
#[derive(Copy, Clone)]
pub enum Event {
    Alloc(usize),
    Free(usize),
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Alloc(u) => write!(f, "a{}", u),
            Free(u) => write!(f, "f{}", u),
        }
    }
}

use crate::apf::trace::Event::*;
use crate::pages::external_mem_reservation::AllocationError;
use crate::{allocate_type, do_free, do_malloc, do_realloc};
use std::ffi::c_void;
use std::mem::size_of;

// Need trace implementation that doesn't call alloc
#[derive(Debug)]
pub struct Trace<'a> {
    ptr: Option<*mut u8>,
    accesses: &'a mut [Event],
    length: usize,
    alloc_count: usize,
}

/*
    Memory trace
    Simple wrapper for vector of events
*/
impl<'a> Trace<'a> {
    pub fn new() -> Trace<'a> {
        let page = allocate_type::<[Event; INIT_TRACE_LENGTH]>() as *mut Event as *mut u8; //;do_malloc(INIT_TRACE_LENGTH * size_of::<Event>);//page_alloc_over_commit(INIT_TRACE_LENGTH);
        let page = if !page.is_null() {
            Ok(page)
        } else {
            Err(AllocationError::AllocationFailed(
                INIT_TRACE_LENGTH,
                errno::errno(),
            ))
        };
        match page {
            Ok(page) => {
                let ptr = page as *mut Event;
                let accesses = unsafe {
                    from_raw_parts_mut(
                        ptr,
                        INIT_TRACE_LENGTH, // Size?
                    )
                };

                Trace {
                    ptr: Some(page),
                    accesses: accesses,
                    length: 0,
                    alloc_count: 0,
                }
            }
            Err(e) => panic!("Error initializing trace: {:?}", e),
        }
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn alloc_length(&self) -> usize {
        self.alloc_count
    }

    pub fn add(&mut self, add: Event) -> () {
        unsafe {
            if self.length == self.accesses.len() - 1 {
                let new_max = self.accesses.len() * 2;
                let page = do_realloc(
                    self.accesses.as_mut_ptr() as *mut c_void,
                    new_max * size_of::<Event>(),
                ) as *mut u8; //page_alloc_over_commit(INIT_TRACE_LENGTH);
                let page = if !page.is_null() {
                    Ok(page)
                } else {
                    Err(AllocationError::AllocationFailed(new_max, errno::errno()))
                };
                match page {
                    Ok(page) => {
                        let ptr = page as *mut Event;
                        let accesses = unsafe {
                            from_raw_parts_mut(
                                ptr, new_max, // Size?
                            )
                        };

                        self.ptr = Some(page);
                        self.accesses = accesses;
                    }
                    Err(e) => panic!("Error initializing trace: {:?}", e),
                }
            }
            (&mut self.accesses[self.length] as *mut Event).write(add);
        }
        self.length += 1;
        match add {
            Alloc(_) => {
                self.alloc_count += 1;
            }
            Free(_) => {}
        };

        /* Unnecessary, as trace now extends in size
        if self.length >= INIT_TRACE_LENGTH {
            panic!("ERROR: Exceeded trace length");
        }

         */
    }

    // For testing only, I hope
    pub fn extend(&mut self, vec: Vec<Event>) -> () {
        for i in 0..vec.len() {
            // self.accesses[self.length] = vec[i];
            unsafe {
                (&mut self.accesses[self.length] as *mut Event).write(vec[i]);
            }
            self.length += 1;
            match vec[i] {
                Alloc(_) => {
                    self.alloc_count += 1;
                }
                Free(_) => {}
            };
        }

        self.length += vec.len();
    }

    pub fn get(&self, index: usize) -> Event {
        self.accesses[index]
    }

    // Counts objects referenced in trace
    pub fn object_count(&self) -> usize {
        // This is dumb
        let mut seen = HashMap::new();

        for i in 0..self.length() {
            match &self.get(i) {
                Alloc(s) => {
                    if !seen.contains_key(s) {
                        seen.insert(s.clone(), true);
                    }
                }
                Free(s) => {
                    if !seen.contains_key(s) {
                        seen.insert(s.clone(), true);
                    }
                }
            };
        }

        seen.len()
    }

    // Converts trace to vector of free intervals represented (si, ei)
    pub fn free_intervals(&self) -> Vec<(usize, usize)> {
        let mut frees = HashMap::<usize, usize>::new();
        let mut result = Vec::new();

        let mut alloc_clock = 0;

        for i in 0..self.length() {
            match self.get(i) {
                Free(s) => {
                    frees.insert(s.clone(), alloc_clock);
                }
                Alloc(e) => {
                    match frees.get(&e) {
                        Some(&s) => {
                            result.push((s, alloc_clock));
                        } // Should format error to include index
                        None => {}
                    }
                    alloc_clock += 1;
                }
            }
        }

        result
    }

    // Converts tract to vector of free intervals represented by (s_i, e_i)
    // Does not use allocation clock
    pub fn free_intervals_alt(&self) -> Vec<(usize, usize)> {
        let mut frees = HashMap::<usize, usize>::new();
        let mut result = Vec::new();

        for i in 0..self.length() {
            match self.get(i) {
                Free(s) => {
                    frees.insert(s.clone(), i);
                }
                Alloc(e) => {
                    match frees.get(&e) {
                        Some(&s) => {
                            result.push((s, i));
                        } // Should format error to include index
                        None => {}
                    }
                }
            }
        }

        result
    }

    // Check validity of trace -- might be useful later
    pub fn valid(&self) -> bool {
        let mut alloc = HashMap::<usize, bool>::new();

        for i in 0..self.length() {
            match self.get(i) {
                Alloc(s) => {
                    match alloc.insert(s, true) {
                        Some(b) => {
                            if b == true {
                                return false;
                            }
                        } // If already allocated, fail
                        _ => {}
                    }
                }
                Free(s) => {
                    match alloc.insert(s, false) {
                        Some(b) => {
                            if b == false {
                                return false;
                            }
                        } // If already freed, fail
                        _ => {
                            return false;
                        } // If never allocated, fail
                    }
                }
            }
        }

        return true;
    }
}

impl Drop for Trace<'_> {
    fn drop(&mut self) {
        match self.ptr {
            None => {}
            Some(ptr) => {
                do_free(ptr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_clock() {
        let mut t = Trace::new();
        t.extend(vec![
            Alloc(3),
            Free(3),
            Free(2),
            Free(1),
            Alloc(1),
            Alloc(2),
        ]);
        assert_eq!(t.free_intervals(), vec![(1, 1), (1, 2)]);
    }

    /* #[test]
    fn test_length() {
        let mut t = Trace::new();
        t.extend(vec![Alloc(1), Alloc(2), Free(1)]);
        assert_eq!(t.length(), 3);
    }

    #[test]
    fn test_obj_count() {
        let mut t = Trace::new();
        t.extend(vec![Alloc(1), Alloc(2), Alloc(4), Free(1)]);
        assert_eq!(t.object_count(), 3);
    }

    #[test]
    fn test_valid() {
        let mut t = Trace::new();
        t.extend(vec![Alloc(1), Alloc(2), Free(1), Free(2), Alloc(5), Free(5)]);
        assert_eq!(t.valid(), true);
    }

    #[test]
    fn test_invalid() {
        let mut t = Trace::new();
        t.extend(vec![Alloc(3), Free(3), Alloc(3), Free(3), Free(3)]);
        assert_eq!(t.valid(), false);
    }

    #[test]
    fn test_intervals() {
        let mut t = Trace::new();
        t.extend(vec![Free(1), Free(3), Alloc(3), Alloc(1), Free(2), Free(3), Free(1), Alloc(2), Alloc(1), Alloc(3)]);
        assert_eq!(t.free_intervals(), vec![(1, 2), (0, 3), (4, 7), (6, 8), (5, 9)]);
    }

    #[test]
    fn test_intervals_2() {
        let mut t = Trace::new();
        t.extend(vec![Alloc(1), Alloc(2), Alloc(3), Free(3), Free(2), Free(1),
                      Alloc(1), Alloc(2), Alloc(3), Free(3), Free(2), Free(1),
                      Alloc(1), Alloc(2), Alloc(3)]);
        assert_eq!(t.free_intervals(), vec![(5, 6), (4, 7), (3, 8), (11, 12), (10, 13), (9, 14)]);
    } */
}
