use core::ptr::NonNull;
use core::alloc::{ AllocRef as Alloc, AllocErr, Layout, AllocInit, MemoryBlock };

use crate::spin::mutex::Mutex;
use super::Heap;
use core::ops::{Deref, DerefMut};

// pub struct LockedFrameHeap {
//     heap: Mutex<Option<Heap>>,
// }
pub struct LockedFrameHeap(Mutex<Option<Heap>>);

impl LockedFrameHeap {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }

    pub fn init(&self, heap_addr_start: usize, size: usize) {
        *self.0.lock() = unsafe { Some(Heap::new(heap_addr_start, size)) };
    }
}

impl Deref for LockedFrameHeap {
    type Target = Mutex<Option<Heap>>;
    fn deref(&self) -> &Mutex<Option<Heap>> { &self.0 }
}

impl DerefMut for LockedFrameHeap {
    fn deref_mut(&mut self) -> &mut Mutex<Option<Heap>> { &mut self.0 }
}

unsafe impl<'a> Alloc for &'a LockedFrameHeap {
    fn alloc(&mut self, layout: Layout, init: AllocInit) -> Result<MemoryBlock, AllocErr> {
        if let Some(ref mut heap) = *self.0.lock() {
            heap.allocate(layout)
        } else {
            panic!("frame heap allocate: heap not initialized");
        }
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        if let Some(ref mut heap) = *self.0.lock() {
            heap.deallocate(ptr, layout)
        } else {
            panic!("frame heap deallocate: heap not initialized");
        }
    }

}
