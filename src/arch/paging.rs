#![feature(allocator_api)]
use core::alloc::{Layout, Allocator as Alloc};
use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr;

use alloc::boxed;
use alloc::string::String;
use alloc::alloc::Global;
use alloc::borrow::ToOwned;

use super::asmfunc;
use core::borrow::{Borrow, BorrowMut};

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

const PAGE_DIR_BASE_ADDR: u32 = 0x00400000;
const PAGE_TABLE_BASE_ADDR: u32 = PAGE_DIR_BASE_ADDR + ((size_of::<u32>() * (NUM_OF_ENTRY + 1)) as u32);
const KERNEL_BASE_ADDR: u32 = 0x0000_0000;
const NUM_OF_ENTRY: usize = 1024;    // 0x10_0000_0000
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
                // set_pg_flag();

                // Graphic::putfont_asc(10, 330, 10, "Paging On!!");
                // let cr0 = load_cr0();
                // let mut printer = Printer::new(100, 230, 10);
                // write!(printer, "{:b}", new_cr0).unwrap();

                let cr3 = load_cr3();
                let mut printer = Printer::new(100, 500, 10);
                write!(printer, "{:x}", cr3).unwrap();
            },
            None => panic!("Error init_paging."),
        };
    }
}

pub fn paging_on() {
    store_cr3(PAGE_DIR_BASE_ADDR);
    set_pg_flag();
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
}

impl<L> Table<L>
where
    L: TableLevel,
{
    pub fn create_next_table(&mut self, index: usize, user_accessible: bool, physical_base_virtual_address: u32) -> Result<*mut Table<L::NextLevel>, String>
    {
        let start_address: *mut [u32; size_of::<u32>() * NUM_OF_ENTRY] = (PAGE_TABLE_BASE_ADDR + (index * size_of::<u32>()) as u32) as *mut [u32; size_of::<u32>() * NUM_OF_ENTRY];
        unsafe { *start_address = [0x00000000; size_of::<u32>() * NUM_OF_ENTRY] };
        let flags = PTE_PRESENT | PTE_RW | if user_accessible { PTE_USER } else { 0 };
        self.entries[index].set(start_address as u32, flags);
        let virtual_table_address = self.entries[index].address() + physical_base_virtual_address;
        unsafe { Ok(virtual_table_address as *mut Table<L::NextLevel>) }
    }
}


pub struct PageTableImpl<A: 'static + Alloc> {
    page_dir_start_address: u32,
    physical_base_virtual_address: u32,
    global_allocator: Option<A>,
}

impl<A> PageTableImpl<A>
where
    A: 'static + Alloc,
{
    pub fn initialize() -> Result<Self, String> {
        Graphic::putfont_asc(210, 175, 10, "1111");
        let start_address: *mut [u32; size_of::<u32>() * NUM_OF_ENTRY] = PAGE_DIR_BASE_ADDR as *mut [u32; size_of::<u32>() * NUM_OF_ENTRY];
        unsafe { *start_address = [0x00000000; size_of::<u32>() * NUM_OF_ENTRY] };

        Graphic::putfont_asc(210, 190, 10, "2222");
        let mut printer = Printer::new(100, 345, 10);
        write!(printer, "{:?}", start_address).unwrap();

        let mut page_dir_base: *mut Table<PageDirectory> = start_address as *mut Table<PageDirectory>;
        unsafe {
            *page_dir_base = Table::new() as Table<PageDirectory>;
        }
        let mut printer = Printer::new(100, 360, 10);
        write!(printer, "{:?}", page_dir_base).unwrap();

        store_cr3(start_address as u32);
        Ok(PageTableImpl {
            page_dir_start_address: start_address as u32,
            physical_base_virtual_address: KERNEL_BASE_ADDR,
            global_allocator: None,
        })
    }

    pub fn set_allocator(mut self, mut global_allocator: A) -> Self {
        self.global_allocator = Some(global_allocator);
        self
    }

    pub fn get_page_directory_table_start_address(&self) -> u32 { self.page_dir_start_address }

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
        let page_dir = self.page_dir_start_address as *mut Table<PageDirectory>;
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
    let vir_address = asmfunc::load_cr2();
    {
        if let Some(ref mut table) = *KERNEL_TABLE.lock() {
                table.map(vir_address);
        } else {
                panic!("There is no Kernel Table.");
        }
    }
    // loop {
    //     asmfunc::io_hlt();
    // }
}