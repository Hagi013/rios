use core::alloc::{ AllocError as AllocErr, Layout };
use core::ptr::NonNull;
use core::slice::SliceIndex;

// use super::Graphic;
// use super::super::Printer;
// use core::fmt::Write;

pub struct Slab {
    block_size: usize,
    free_block_list: FreeBlockList,
}

impl Slab {
    pub fn new(start_addr: usize, slab_size: usize, block_size: usize) -> Self {
        let num_of_blocks: usize = slab_size / block_size;
        Slab {
            block_size,
            free_block_list: unsafe { FreeBlockList::new(start_addr, block_size, num_of_blocks) },
        }
    }

    pub fn used_blocks(&self) -> usize {
        self.free_block_list.len()
    }

    pub unsafe fn grow(&mut self, start_addr: usize, slab_size: usize) {
        let num_of_blocks: usize = slab_size / self.block_size;
        let mut block_list: FreeBlockList = unsafe { FreeBlockList::new(start_addr, slab_size, num_of_blocks) };
        while let Some(block) = block_list.pop() {
            self.free_block_list.push(block);
        }
    }

    pub fn allocate(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocErr> {
        match self.free_block_list.pop() {
            // Some(block) => Ok(unsafe {
            //     MemoryBlock {
            //         ptr: NonNull::new_unchecked(block.addr() as *mut u8),
            //         size: layout.size()
            //     }
            // }),r
            Some(block) => {
                Ok(unsafe { NonNull::slice_from_raw_parts(NonNull::new_unchecked(block.addr() as *mut u8), layout.size()) })
            },
            None => Err(AllocErr),
        }
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        let ptr: *mut FreeBlock = ptr.as_ptr() as *mut FreeBlock;
        unsafe { self.free_block_list.push(&mut *ptr); }
    }
}

struct FreeBlockList {
    len: usize,
    head: Option<&'static mut FreeBlock>,
}

impl FreeBlockList {
    unsafe fn new(start_addr: usize, block_size: usize, num_of_blocks: usize) -> Self {
        let mut new_list = FreeBlockList::new_empty();
        for i in 0..num_of_blocks {
            let new_block = (start_addr + i * block_size) as *mut FreeBlock;
            new_list.push(&mut *new_block);
        }
        new_list
    }

    fn new_empty() -> FreeBlockList {
        FreeBlockList {
            len: 0,
            head: None,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn pop(&mut self) -> Option<&'static mut FreeBlock> {
        self.head.take().map(|node| {
            self.head = node.next.take();
            self.len -= 1;
            node
        })
    }

    fn push(&mut self, free_block: &'static mut FreeBlock) {
        free_block.next = self.head.take();
        self.len += 1;
        self.head = Some(free_block);
    }

    fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

impl Drop for FreeBlockList {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
    }
}

struct FreeBlock {
    next: Option<&'static mut FreeBlock>,
}

impl FreeBlock {
    fn addr(&self) -> usize {
        self as *const _ as usize
    }
}
