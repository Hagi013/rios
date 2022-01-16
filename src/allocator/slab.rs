use core::alloc::{ AllocError as AllocErr, Layout };
use core::ptr::NonNull;
use core::slice::SliceIndex;

use super::Graphic;
use super::super::Printer;
use core::fmt::Write;

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

    pub fn new_test(start_addr: usize, slab_size: usize, block_size: usize) -> Self {
        let num_of_blocks: usize = slab_size / block_size;
        // block size = 128, start_addr = 0xb0200000, slab_size = 0x200000(2MB)
        // num_of_blocks = slab_size / block_size = 16kb
        let mut printer = Printer::new(400, 100, 0);
        write!(printer, "{:x}", start_addr).unwrap();
        let mut printer = Printer::new(400, 115, 0);
        write!(printer, "{:x}", slab_size).unwrap();
        let mut printer = Printer::new(400, 130, 0);
        write!(printer, "{:x}", block_size).unwrap();
        Slab {
            block_size,
            free_block_list: unsafe { FreeBlockList::new_test(start_addr, block_size, num_of_blocks) },
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

    pub fn allocate_128(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocErr> {
        match self.free_block_list.pop_128() {
            Some(block) => {
                Ok(unsafe { NonNull::slice_from_raw_parts(NonNull::new_unchecked(block.addr() as *mut u8), layout.size()) })
            },
            None => Err(AllocErr),
        }
    }

    pub fn allocate_test(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocErr> {
        match self.free_block_list.pop_test() {
            Some(block) => {
                let mut printer = Printer::new(400, 300, 0);
                write!(printer, "{:?}", layout.size()).unwrap();
                let mut printer = Printer::new(400, 315, 0);
                write!(printer, "{:x}", block.addr() as *mut u8 as u8).unwrap();

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

static mut TMP_IDX: usize = 0;
static mut TMP_STACK: [usize; 1000] = [0xfff; 1000];

impl FreeBlockList {
    unsafe fn new(start_addr: usize, block_size: usize, num_of_blocks: usize) -> Self {
        let mut new_list = FreeBlockList::new_empty();
        for i in 0..num_of_blocks {
            let new_block = (start_addr + i * block_size) as *mut FreeBlock;
            new_list.push(&mut *new_block);
        }
        new_list
    }

    unsafe fn new_test(start_addr: usize, block_size: usize, num_of_blocks: usize) -> Self {
        let mut new_list = FreeBlockList::new_empty();
        for i in 0..num_of_blocks {
            let new_block = (start_addr + i * block_size) as *mut FreeBlock;
            new_list.push(&mut *new_block);
            if i == 0 {
                let mut printer = Printer::new(400, 145, 0);
                write!(printer, "{:x}", (start_addr + i * block_size)).unwrap();
            }
            if i == num_of_blocks - 1 {
                let mut printer = Printer::new(400, 160, 0);
                write!(printer, "{:x}", (start_addr + i * block_size)).unwrap();
                let mut printer = Printer::new(400, 175, 0);
                write!(printer, "{:x}", i).unwrap();
                let mut printer = Printer::new(400, 190, 0);
                write!(printer, "{:x}", new_block as usize).unwrap(); // 0xb03fff80
            }
        }
        let mut printer = Printer::new(400, 205, 0);
        write!(printer, "{:x}", &mut new_list as *mut FreeBlockList as usize).unwrap(); // 0x3ffe8b
        let mut printer = Printer::new(400, 220, 0);
        write!(printer, "{:x}", new_list.len).unwrap();

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

    fn pop_128(&mut self) -> Option<&'static mut FreeBlock> {
        self.head.take().map(|node| {
            // unsafe { TMP_STACK[TMP_IDX] = node.addr() };
            if node.next.is_some() && node.next.as_ref().unwrap().addr() == 0x69696761 {
                let mut printer = Printer::new(400, 435, 0);
                write!(printer, "{:x}", &self.head as *const _ as usize).unwrap(); // 0x29c664
            }
            self.head = node.next.take();
            if self.head.is_some() && self.head.as_ref().unwrap().addr() == 0x69696761 {
                let mut printer = Printer::new(400, 390, 0);
                write!(printer, "{:x}", self.head.as_ref().unwrap().addr()).unwrap();
                let mut printer = Printer::new(400, 405, 0);
                write!(printer, "{:x}", node.addr()).unwrap();
                // let mut printer = Printer::new(400, 420, 0);
                // write!(printer, "{:x}", unsafe { TMP_IDX }).unwrap();
                // let mut printer = Printer::new(400, 450, 0);
                // write!(printer, "{:x}", unsafe { &TMP_STACK as *const [usize; 100] as usize }).unwrap();
                // let mut printer = Printer::new(400, 465, 0);
                // write!(printer, "{:x}", unsafe { TMP_STACK[TMP_IDX] }).unwrap();

                // loop {} // これでstacktraceだせる？
            }
            self.len -= 1;
            // unsafe { TMP_IDX += 1 };
            node
        })
    }

    fn pop_test(&mut self) -> Option<&'static mut FreeBlock> {
        let mut printer = Printer::new(400, 375, 0);
        write!(printer, "{:x}", &self.head as *const _ as usize).unwrap(); // 0x29c664
        self.head.take().map(|node| {
            let mut printer = Printer::new(400, 330, 0);
            write!(printer, "{:x}", node.addr()).unwrap(); // 0x69696761？？
            let mut printer = Printer::new(400, 345, 0);
            write!(printer, "{:x}", node as *const _ as usize).unwrap();
            let mut printer = Printer::new(400, 360, 0);
            write!(printer, "{:x}", &node.next as *const Option<&'static mut FreeBlock> as usize).unwrap();
            self.head = node.next.take(); // ここでGPエラーになる
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
