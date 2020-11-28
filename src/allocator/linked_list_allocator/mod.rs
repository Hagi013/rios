use core::mem;
use core::alloc::{Layout, AllocError as AllocErr};
use core::ptr::NonNull;

pub mod hole;
use hole::HoleList;
use hole::Hole;

pub struct Heap {
    bottom: usize,
    size: usize,
    holes: HoleList,
}

impl Heap {
    pub const fn empty() -> Heap {
        Heap {
            bottom: 0,
            size: 0,
            holes: HoleList::empty(),
        }
    }

    pub unsafe fn init(&mut self, heap_bottom: usize, heap_size: usize) {
        self.bottom = heap_bottom;
        self.size = heap_size;
        self.holes = HoleList::new(heap_bottom, heap_size);
    }

    pub unsafe fn new(heap_bottom: usize, heap_size: usize) -> Heap {
        Heap {
            bottom: heap_bottom,
            size: heap_size,
            holes: HoleList::new(heap_bottom, heap_size),
        }
    }

    pub fn allocate_first_fit(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocErr> {
        let mut size = layout.size();
        if size < HoleList::min_size() {
            size = HoleList::min_size();
        }
        let size = align_up(size, mem::align_of::<Hole>());
        let layout = Layout::from_size_align(size, layout.align()).unwrap();

        self.holes.allocate_first_fit(layout)
    }

    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let mut size = layout.size();
        if size < HoleList::min_size() {
            size = HoleList::min_size();
        }
        let size = align_up(size, mem::align_of::<Hole>());
        let layout = Layout::from_size_align(size, layout.align()).unwrap();
        self.holes.deallocate(ptr, layout);
    }

    pub fn bottom(&self) -> usize {
        self.bottom
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn top(&self) -> usize {
        self.bottom + self.size
    }

    pub unsafe fn extend(&mut self, by: usize) {
        let top = self.top();
        let layout = Layout::from_size_align(by, 1).unwrap();
        self.holes.deallocate(NonNull::new_unchecked(top as *mut u8), layout);
        self.size += by;
    }
}

pub fn align_down(addr: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        addr & !(align - 1)
    } else if align == 0 {
        addr
    } else {
        panic!("`align` must be a power of 2");
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + align - 1, align)
}