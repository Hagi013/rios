#![feature(allocator_api)]
use core::alloc::{Layout, Allocator as Alloc};
use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr::{self, read_volatile, write_volatile};

use alloc::boxed;
use alloc::string::String;
use alloc::alloc::Global;
use alloc::borrow::ToOwned;

use super::asmfunc;
use core::borrow::{Borrow, BorrowMut};

use super::boot_info::BootInfo;

use super::super::allocator::LockedHeap;

use crate::spin::mutex::Mutex;

use super::graphic::Graphic;
use super::super::Printer;
use core::fmt::Write;

use crate::asmfunc::{load_cr0, store_cr0, load_cr3, store_cr3, set_pg_flag};

const PTE_PRESENT: u16 = 0x0001;   // P bit
const PTE_RW: u16 = 0x0002;        // R bit
const PTE_USER: u16 = 0x0004;      // U/S bit
const PTE_PWT: u16 = 0x0008;      // Page Write Through bit
const PTE_PCD: u16 = 0x0010;      // Page Cache Disable bit
const PTE_ACCESS: u16 = 0x0020;    // A bit
const PTE_DIRTY: u16 = 0x0040;     // D bit
const PTE_G: u16 = 0x0100;     // Global bit

// const PAGE_DIR_BASE_ADDR: u32 = 0x00400000;
const PAGE_DIR_BASE_ADDR: u32 = 0x00a00000;
const NUM_OF_ENTRY: usize = 1024;    // 0x10_0000_0000
const PAGE_TABLE_BASE_ADDR: u32 = PAGE_DIR_BASE_ADDR + (size_of::<u32>() * NUM_OF_ENTRY) as u32;
const KERNEL_BASE_ADDR: u32 = 0x0000_0000;
const ADDRESS_MSK: u32 = 0xfffff000;
const SIZE_OF_PAGE: usize = 4096;

static KERNEL_TABLE: Mutex<Option<PageTableImpl<&LockedHeap>>> = Mutex::new(None);

pub fn set_kernel_table(table: PageTableImpl<&LockedHeap>) {
    {
        unsafe {
            *KERNEL_TABLE.lock() = Some(table);
        };
    }
}


pub fn init_paging(table: PageTableImpl<&LockedHeap>)
{
    set_kernel_table(table);
    // paging開始
    {
        match *KERNEL_TABLE.lock() {
            Some(ref table) => {
                // Cr0のPGフラグをOnにする
                set_pg_flag();
                let cr3 = load_cr3();
                let mut printer = Printer::new(100, 500, 0);
                write!(printer, "{:x}", cr3).unwrap();
            },
            None => panic!("Error init_paging."),
        };
    }
}

pub fn set_kernel_table_allocator(allocator: &'static LockedHeap) {
    {
        match *KERNEL_TABLE.lock() {
            Some(ref mut table) => {
                table.set_allocator(allocator);
            },
            None => panic!("Error set_kernel_table_allocator."),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct Entry(u32);

impl Entry {
    pub fn unused() -> Entry {
        Entry(0)
    }
    pub fn address(&self) -> u32 {
        self.0 & ADDRESS_MSK
    }
    pub fn set(&mut self, start: u32, flags: u16) {
        self.0 = start | (flags | PTE_PRESENT) as u32;
    }
    pub fn present(&self) -> bool { self.0 & PTE_PRESENT as u32 > 0 }
    pub fn accessed(&self) -> bool { self.0 & PTE_ACCESS as u32 > 0 }
    pub fn dirty(&self) -> bool { self.0 & PTE_DIRTY as u32 > 0 }
    pub fn writable(&self) -> bool { self.0 & PTE_RW as u32 > 0 }
}


enum PageDirectory {}
enum PageTable {}

pub trait Level {}
impl Level for PageDirectory {}
impl Level for PageTable {}

trait TableLevel: Level {
    type NextLevel: Level;
}
impl TableLevel for PageDirectory {
    type NextLevel = PageTable;
}

#[repr(align(4096))]
struct Table<L>
where
    L: Level,
{
     entries: [Entry; NUM_OF_ENTRY],
    _phantom: PhantomData<L>,
}

impl<L> Table<L>
where
    L: Level,
{
    fn new() -> Table<L> {
        Table { entries: [Entry::unused(); NUM_OF_ENTRY], _phantom: PhantomData }
    }

    pub fn set_address(&mut self, index: usize, address: u32, user_accessible: bool) -> Result<(), String> {
        let flags = PTE_PRESENT | PTE_RW | if user_accessible { PTE_USER } else { 0 };
        self.entries[index].set(address, flags);
        Ok(())
    }
}

impl<L> Table<L>
where
    L: TableLevel,
{
    pub fn create_next_table(&mut self, index: usize, user_accessible: bool, physical_base_virtual_address: u32) -> Result<*mut Table<L::NextLevel>, String> {
        let table_size: usize = size_of::<u32>() * NUM_OF_ENTRY;
        let start_address: *mut [u32; size_of::<u32>() * NUM_OF_ENTRY] = (PAGE_TABLE_BASE_ADDR + (index * table_size) as u32) as *mut [u32; size_of::<u32>() * NUM_OF_ENTRY];
        unsafe { *start_address = [0x00000000; size_of::<u32>() * NUM_OF_ENTRY] };
        let flags = PTE_PRESENT | PTE_RW | if user_accessible { PTE_USER } else { 0 };
        self.entries[index].set(start_address as u32, flags);
        let virtual_table_address = self.entries[index].address() + physical_base_virtual_address;
        unsafe { Ok(virtual_table_address as *mut Table<L::NextLevel>) }
    }
}


pub struct PageTableImpl<A: 'static + Alloc> {
    physical_base_virtual_address: u32,
    global_allocator: Option<A>,
}

impl<A> PageTableImpl<A>
where
    A: 'static + Alloc,
{
    pub fn initialize() -> Result<Self, String> {
        Self::bury_zero();
        store_cr3(PAGE_DIR_BASE_ADDR);

        // 0x00000000 - 0x003fffff ⇒ direct mapping(0x00000000(dir index: 0) - 0x003fffff(dir index: 0))
        Self::map_entry(0x00000000, 0x00000000, 0x00400000, KERNEL_BASE_ADDR);

        let mut boot_info: BootInfo = BootInfo::new();
        // 0xfd000000 - 0xfd0c0400(VRAM) ⇒ 0x00400000(dir index: 1) -  0x007fffff(dir index: 1)
        Self::map_entry(boot_info.vram, 0x00400000, 0x00400000, KERNEL_BASE_ADDR);
        boot_info.set_addr_vram(0x00400000);

        // 0x00e80000 - 0x40a80000(Heap) ⇒ 0x00800000(dir index: 2) - 0x3fffffff(dir index: 255)
        Self::map_entry(0x00e80000, 0x00800000, 0x3f800000, KERNEL_BASE_ADDR);

        Ok(PageTableImpl {
            physical_base_virtual_address: KERNEL_BASE_ADDR,
            global_allocator: None,
        })
    }

    fn bury_zero() {
        let dir_start_address: *mut [u32; size_of::<u32>() * NUM_OF_ENTRY] = PAGE_DIR_BASE_ADDR as *mut [u32; size_of::<u32>() * NUM_OF_ENTRY];
        unsafe { *dir_start_address = [0x00000000; size_of::<u32>() * NUM_OF_ENTRY] };
        let mut table_start_address: *mut [u32; size_of::<u32>() * NUM_OF_ENTRY * 1024] = PAGE_TABLE_BASE_ADDR as *mut [u32; size_of::<u32>() * NUM_OF_ENTRY * 1024];
        unsafe { *table_start_address = [0x00000000; size_of::<u32>() * NUM_OF_ENTRY * 1024] };
    }

    // physical addressとvirtual addressのマッピングを行う
    // 引数はphysical & virtual addressの始点とバイト数(range)
    // page directoryのindexとpage tableの位置についてはvirtual addressから取得可能
    pub fn map_entry(start_phys_address: u32, start_vir_address: u32, range: usize, kernel_base_address: u32) -> Result<(), String> {
        let phys_address: usize = start_phys_address as usize;
        let vir_address: usize = start_vir_address as usize;
        let page_dir_tbl = asmfunc::load_cr3() as *mut Table<PageDirectory>;
        let flags = PTE_PRESENT | PTE_RW | PTE_USER;
        let page_size = size_of::<usize>() * NUM_OF_ENTRY;
        // let mut count: usize = 0;
        // 1 page毎なので、4096毎
        for i in 0..(range / page_size) {
            let phys_address = phys_address + (page_size * i);
            let vir_address = vir_address + (page_size * i);
            let dir_idx: usize = (vir_address >> 22) as usize;
            let tbl_idx: usize = (vir_address >> 12 & 0x3ff) as usize;
            let dir_entry: Entry = unsafe { (*page_dir_tbl).entries[dir_idx] };

            if dir_entry.present() {
                Graphic::putfont_asc(210, 350, 0, "OOOOKKKKKKK");
                let mut page_table = dir_entry.address() as *mut Table<PageTable>;
                unsafe { (*page_table).set_address(tbl_idx, phys_address as u32 & 0xfffff000, true); }
            } else {
                let page_table = match unsafe { (*page_dir_tbl).create_next_table(dir_idx, false, kernel_base_address) } {
                    Ok(table) => table,
                    Err(e) => {
                        panic!("Error in PageTableImpl.allocate_frame. {:?}", e)
                    },
                };
                unsafe { (*page_table).set_address(tbl_idx, phys_address as u32 & 0xfffff000, true); }
            }
        }
        Ok(())
    }

    pub fn set_allocator(&mut self, mut global_allocator: A) {
        self.global_allocator = Some(global_allocator);
    }

    pub fn get_physaddr(&self, vir_address: u32) -> u32 {
        let cr3 = asmfunc::load_cr3();
        let page_dir_tbl = cr3 as *mut Table<PageDirectory>;
        let position_in_dir: usize = (vir_address >> 22) as usize; // PageTable no.
        let pte_address = unsafe { (*page_dir_tbl).entries[position_in_dir].address() as *mut Table<PageTable> };
        let position_in_pte: usize = (vir_address >> 12 & 0x3ff) as usize;
        unsafe { (*pte_address).entries[position_in_pte].address() + vir_address & 0x00000fff }
    }

    pub fn allocate_frame(&mut self) -> Result<u32, String> {
        let layout = match Layout::from_size_align(SIZE_OF_PAGE, SIZE_OF_PAGE) {
            Ok(l) => l,
            Err(e) => panic!("Error in PageTableImpl.allocate_frame. {:?}", e),
        };
        if (&self.global_allocator).is_none() { panic!("global_allocator in PageTableImpl is not set."); }
        let ptr = self.global_allocator
            .as_mut()
            .unwrap()
            .allocate(layout)
            .or(Err("Error in PageTableImpl.allocate_frame when call self.table_allocator.alloc().".to_owned()))?
            .as_ptr();
        Ok(ptr as *const *mut [u8] as usize as u32)
    }

    pub fn map(&mut self, vir_address: u32) -> Result<(), String> {
        let page_dir = PAGE_DIR_BASE_ADDR as *mut Table<PageDirectory>;
        let position_in_dir: usize = (vir_address >> 22) as usize; // PageTable no.
        let dir_entry = unsafe { (*page_dir).entries[position_in_dir] };
        let table_idx = (vir_address >> 12 & 0x3ff) as usize;
        if dir_entry.present() {
            let page_table = dir_entry.address() as *mut Table<PageTable>;
            let mut table_entry = unsafe { (*page_table).entries[table_idx] };
            if table_entry.present() {
                return Err(format!("Already Exist in {:?}.", vir_address));
            }
            let pyhs_address = match self.allocate_frame() {
                Ok(addr) => addr,
                Err(e) => panic!(&format!("Error in PageTableImpl.map when call self.allocate_frame(). {:?}", e))
            };
            let flags = PTE_PRESENT | PTE_RW | PTE_USER;
            table_entry.set(pyhs_address & 0xfffff000, flags);
        } else {
            let page_table: *mut Table<PageTable> = unsafe {
                match (*page_dir).create_next_table(position_in_dir, false, self.physical_base_virtual_address) {
                    Ok(table) => table,
                    Err(e) => {
                        panic!(&format!("Error in map {:?}.", e));
                    }
                }
            };
            let mut table_entry = unsafe { (*page_table).entries[table_idx] };
            let pyhs_address = match self.allocate_frame() {
                Ok(addr) => addr,
                Err(e) => panic!(&format!("Error in PageTableImpl.map when call self.allocate_frame(). {:?}", e))
            };
            let flags = PTE_PRESENT | PTE_RW | PTE_USER;
            table_entry.set(pyhs_address & 0xfffff000, flags);
        }
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn page_fault_handler(esp: *const usize) {
    Graphic::putfont_asc(10, 345, 10, "Page Fault!!");
    loop {}
    let vir_address = asmfunc::load_cr2();
    {
        if let Some(ref mut table) = *KERNEL_TABLE.lock() {
                table.map(vir_address);
        } else {
                panic!("There is no Kernel Table.");
        }
    }
}