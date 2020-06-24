use crate::mem_info::{align_addr, CACHE_LINE, CACHE_LINE_MASK, DESCRIPTOR_BLOCK_SZ};

use super::Anchor;
use crate::allocation_data::proc_heap::ProcHeap;

use crate::pages::page_alloc;
use crate::AVAILABLE_DESC;
use atomic::{Atomic, Ordering};

use std::mem::MaybeUninit;
use std::ptr::null_mut;

#[derive(Copy, Clone, Debug)]
pub struct DescriptorNode {
    desc: *mut Descriptor,
}

impl Default for DescriptorNode {
    fn default() -> Self {
        Self { desc: null_mut() }
    }
}

unsafe impl Send for DescriptorNode {}
unsafe impl Sync for DescriptorNode {}

impl DescriptorNode {
    pub const fn new() -> Self {
        Self { desc: null_mut() }
    }

    pub fn set(&mut self, desc: Option<&'static Descriptor>, count: u64) {
        if desc.is_some() {
            let usize_desc = desc.unwrap() as *const Descriptor as u64;
            assert_eq!(usize_desc & CACHE_LINE_MASK as u64, 0);
            let pointer = (usize_desc | (count & CACHE_LINE_MASK as u64)) as *mut Descriptor;
            self.desc = pointer;
        } else {
            // panic!("descriptor can not be None");
            self.desc = null_mut();
        }
    }

    pub fn get_desc(&self) -> Option<&'static mut Descriptor> {
        let usize_desc = self.desc as u64;
        let fixed_ptr = usize_desc & !CACHE_LINE_MASK as u64;
        if (fixed_ptr as *const Descriptor).is_null() {
            None
        } else {
            Some(unsafe { &mut *(fixed_ptr as *mut Descriptor) })
        }
    }

    pub fn get_counter(&self) -> u64 {
        let usize_desc = self.desc as u64;
        usize_desc & CACHE_LINE_MASK as u64
    }
}

impl From<*mut Descriptor> for DescriptorNode {
    fn from(d: *mut Descriptor) -> Self {
        Self { desc: d }
    }
}

/// This is a super block descriptor
/// needs to be cache aligned
/// Descriptors are never freed, and should be statically allocated
#[repr(align(64))]
#[derive(Debug)]
pub struct Descriptor {
    pub next_free: Atomic<Option<DescriptorNode>>,
    pub next_partial: Atomic<Option<DescriptorNode>>,
    pub anchor: Atomic<Anchor>,
    pub super_block: *mut u8,
    pub proc_heap: *mut ProcHeap,
    pub block_size: u32,
    pub max_count: u32,
}

//

impl Default for Descriptor {
    fn default() -> Self {
        Self {
            next_free: Atomic::new(None),
            next_partial: Atomic::new(None),
            anchor: Atomic::new(Anchor::default()),
            super_block: null_mut(),
            proc_heap: null_mut(),
            block_size: 0,
            max_count: 0,
        }
    }
}

/*
/// The intitial descriptor holder
struct DescriptorHolder {}


lazy_static! {
static ref DESCRIPTORS_SPACE

}

 */

impl Descriptor {
    pub fn retire(&'static mut self) {
        self.block_size = 0;
        let mut avail = AVAILABLE_DESC.lock();
        let old_head = *avail;
        let mut new_head: DescriptorNode = DescriptorNode::default();
        self.next_free.store(Some(old_head), Ordering::Release);

        new_head.set(Some(self), old_head.get_counter() + 1);
        *avail = new_head;
        /*
           if {
               AVAILABLE_DESC
                   .compare_exchange_weak(old_head, new_head, Ordering::Acquire, Ordering::Release)
                   .is_ok()
           } {
               break;
           }

        */
    }

    pub unsafe fn alloc() -> *mut Descriptor {
        let mut avail = AVAILABLE_DESC.lock();
        let old_head = *avail; //AVAILABLE_DESC.load(Ordering::Acquire);
        loop {
            let desc = old_head.get_desc();
            return if desc.is_none() {
                let ptr =
                    page_alloc(DESCRIPTOR_BLOCK_SZ).expect("Creating a descriptor block failed");
                let ret = ptr as *mut MaybeUninit<Descriptor>;
                // organize list with the rest of the descriptors
                // and add to available descriptors

                let mut prev: *mut MaybeUninit<Descriptor> = null_mut();

                let descriptor_size = std::mem::size_of::<Descriptor>() as isize;
                let mut curr_ptr = ptr.offset(descriptor_size);
                curr_ptr = align_addr(curr_ptr as usize, CACHE_LINE) as *mut u8;
                let first = curr_ptr as *mut MaybeUninit<Descriptor>;
                let max = ptr as usize + DESCRIPTOR_BLOCK_SZ;
                let mut count = 0;
                while (curr_ptr as usize + descriptor_size as usize) < max {
                    let curr = curr_ptr as *mut MaybeUninit<Descriptor>;
                    unsafe { *curr = MaybeUninit::new(Descriptor::default()) }
                    if !prev.is_null() {
                        let prev_init = &mut *(*prev).as_mut_ptr();
                        prev_init.next_free.store(
                            Some(DescriptorNode::from(curr_ptr as *mut Descriptor)),
                            Ordering::Release,
                        );
                    }

                    prev = curr;
                    curr_ptr = curr_ptr.offset(1);
                    curr_ptr = align_addr(curr_ptr as usize, CACHE_LINE) as *mut u8;
                    count += 1;
                }
                //info!("Generated {} Descriptors", count);

                let prev = &mut *(prev as *mut Descriptor);
                prev.next_free.store(None, Ordering::Release);

                // let old_head: DescriptorNode = AVAILABLE_DESC.load(Ordering::Acquire);
                let mut new_head: DescriptorNode = DescriptorNode::default();
                // loop {
                //prev.next_free.store(Some(old_head), Ordering::Release);
                new_head.set(Some(&mut *(first as *mut Descriptor)), 0);
                /*
                if AVAILABLE_DESC
                    .compare_exchange_weak(
                        old_head,
                        new_head,
                        Ordering::Acquire,
                        Ordering::Release,
                    )
                    .is_ok()
                {
                    break;
                }

                 */
                *avail = new_head;
                // }

                ret as *mut Descriptor
            } else {
                let desc = desc.unwrap();
                let mut new_head = desc.next_free.load(Ordering::Acquire);

                match &mut new_head {
                    None => {}
                    Some(new_head) => match new_head.get_desc() {
                        Some(desc) => {
                            new_head.set(Some(desc), old_head.get_counter());
                        }
                        None => {
                            new_head.set(None, old_head.get_counter());
                        }
                    },
                }

                /*
                if AVAILABLE_DESC
                    .compare_exchange_weak(
                        old_head,
                        new_head,
                        Ordering::Acquire,
                        Ordering::Release,
                    )
                    .is_ok()
                {
                    return desc as *mut Descriptor;
                }
                 */
                *avail = new_head.unwrap_or(DescriptorNode::new());
                desc
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::allocation_data::SuperBlockState;
    use std::mem::size_of;

    #[test]
    fn descriptor_list_good() {
        unsafe {
            let ptr = page_alloc(DESCRIPTOR_BLOCK_SZ).expect("Creating a descriptor block failed");
            let ret = ptr as *mut MaybeUninit<Descriptor>;
            // organize list with the rest of the descriptors
            // and add to available descriptors

            let mut prev: *mut MaybeUninit<Descriptor> = null_mut();

            let descriptor_size = std::mem::size_of::<Descriptor>() as isize;
            let mut curr_ptr = ptr.offset(descriptor_size);
            curr_ptr = align_addr(curr_ptr as usize, CACHE_LINE) as *mut u8;
            let first = curr_ptr as *mut MaybeUninit<Descriptor>;
            let max = ptr as usize + DESCRIPTOR_BLOCK_SZ;
            let mut count = 0;
            while (curr_ptr as usize + descriptor_size as usize) < max {
                let curr = curr_ptr as *mut MaybeUninit<Descriptor>;
                unsafe { *curr = MaybeUninit::new(Descriptor::default()) }
                if !prev.is_null() {
                    let prev_init = &mut *(prev as *mut Descriptor);
                    prev_init.next_free.store(
                        Some(DescriptorNode::from(curr_ptr as *mut Descriptor)),
                        Ordering::Release,
                    );
                }

                prev = curr;
                curr_ptr = curr_ptr.offset(descriptor_size);
                curr_ptr = align_addr(curr_ptr as usize, CACHE_LINE) as *mut u8;
                count += 1;
            }
            //info!("Generated {} Descriptors", count);

            let prev = &mut *(prev as *mut Descriptor);
            prev.next_free.store(None, Ordering::Release);

            // let old_head: DescriptorNode = AVAILABLE_DESC.load(Ordering::Acquire);
            let mut new_head: DescriptorNode = DescriptorNode::default();
            // loop {
            //prev.next_free.store(Some(old_head), Ordering::Release);
            new_head.set(Some(&mut *(first as *mut Descriptor)), 0);

            assert!(prev.next_free.load(Ordering::Relaxed).is_none());

            let mut curr: Option<*mut Descriptor> = Some(prev as *mut Descriptor);
            let mut prev: Option<*mut Descriptor> = None;
            loop {
                if (*curr.unwrap()).next_free.load(Ordering::Acquire).is_some() {
                    assert_eq!(
                        (*curr.unwrap())
                            .next_free
                            .load(Ordering::Acquire)
                            .unwrap()
                            .desc,
                        prev.unwrap(),
                        "Descriptor at {:x?} has incorrect next free pointer",
                        curr.unwrap()
                    );
                }

                let descriptor = curr.unwrap().read();
                let anchor = descriptor.anchor.load(Ordering::Acquire);
                assert_eq!(anchor.state(), SuperBlockState::FULL);
                assert_eq!(anchor.avail(), 0);
                assert_eq!(anchor.count(), 0);

                prev = curr.clone();
                curr = Some(curr.unwrap().offset(-1));
                if curr.unwrap() == ret as *mut Descriptor {
                    break;
                }
            }
        }
    }
}
