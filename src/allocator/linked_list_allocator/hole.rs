use alloc::alloc::{AllocError as AllocErr, Layout};
use core::mem::size_of;
use core::ptr::NonNull;

use super::align_up;
// use core::alloc::Alloc;

/// A sorted list of holes. It uses the the holes itself to store its nodes.
pub struct HoleList {
    first: Hole, // dummy
}

impl HoleList {
    pub const fn empty() -> HoleList {
        HoleList {
            first: Hole {
                size: 0,
                next: None,
            }
        }
    }

    pub unsafe fn new(hole_addr: usize, hole_size: usize) -> HoleList {
        assert_eq!(size_of::<Hole>(), Self::min_size());

        let ptr = hole_addr as *mut Hole;
        ptr.write(Hole {
            size: hole_size,
            next: None,
        });
        HoleList {
           first: Hole {
               size: 0,
               next: Some(&mut *ptr),
           }
        }
    }

    pub fn allocate_first_fit(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocErr> {
        assert!(layout.size() >= Self::min_size());
        allocate_first_fit(&mut self.first, layout).map(|allocation| {
            if let Some(padding) = allocation.front_padding {
                deallocate(&mut self.first, padding.addr, padding.size);
            }
            if let Some(padding) = allocation.back_padding {
                deallocate(&mut self.first, padding.addr, padding.size);
            }
            // MemoryBlock {
            //     ptr: NonNull::new(allocation.info.addr as *mut u8).unwrap(),
            //     size: layout.size(),
            // }
            unsafe {
                NonNull::slice_from_raw_parts(NonNull::new_unchecked(allocation.info.addr as *mut u8), layout.size())
            }
        })
    }

    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        deallocate(&mut self.first, ptr.as_ptr() as usize, layout.size())
    }

    /// Returns the minimal allocation size. Smaller allocations or deallocations are not allowed.
    pub fn min_size() -> usize {
        size_of::<usize>() * 2
    }
}

pub struct Hole {
    size: usize,
    next: Option<&'static mut Hole>,
}

impl Hole {
    fn info(&self) -> HoleInfo {
        HoleInfo {
            addr: self as *const _ as usize,
            size: self.size
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct HoleInfo {
    addr: usize,
    size: usize,
}

struct Allocation {
    info: HoleInfo,
    front_padding: Option<HoleInfo>,
    back_padding: Option<HoleInfo>,
}

fn split_hole(hole: HoleInfo, required_layout: Layout) -> Option<Allocation> {
    let required_size = required_layout.size();
    let required_align = required_layout.align();

    let (align_addr, front_padding) = if hole.addr == align_up(hole.addr, required_align) {
        // hole has already the required alignment
        (hole.addr, None)
    } else {
        // the required alignment causes some padding before the allocation
        let align_addr = align_up(hole.addr + HoleList::min_size(), required_align);
        (
            align_addr,
            Some(
                HoleInfo {
                    addr: hole.addr,
                    size: align_addr - hole.addr,
                }),
        )
    };

    let aligned_hole = {
        if align_addr + required_size > hole.addr + hole.size {
            // Hole is too small
            return None;
        }
        HoleInfo {
            addr: align_addr,
            size: hole.size - (align_addr - hole.addr),
        }
    };

    let back_padding = if aligned_hole.size == required_size {
        None
    } else if aligned_hole.size - required_size < HoleList::min_size() {
        None
    } else {
        // the hole is bigger than necessary, so there is some padding behind the allocation
        Some(HoleInfo {
            addr: aligned_hole.addr + required_size,
            size: aligned_hole.size - required_size,
        })
    };
    Some(Allocation {
        info: HoleInfo {
            addr: aligned_hole.addr,
            size: required_size,
        },
        front_padding,
        back_padding
    })
}

fn allocate_first_fit(mut previous: &mut Hole, layout: Layout) -> Result<Allocation, AllocErr> {
    loop {
        let allocation: Option<Allocation> = previous
            .next
            .as_mut()
            .and_then(|current| split_hole(current.info(), layout.clone()));
        match allocation {
            Some(allocation) => {
                previous.next = previous.next.as_mut().unwrap().next.take();
                return Ok(allocation);
            },
            None if previous.next.is_some() => {
                previous = move_helper(previous).next.as_mut().unwrap();
            },
            None => {
                return Err(AllocErr);
            },
        }
    }
}

fn deallocate(mut hole: &mut Hole, addr: usize, mut size: usize) {
    loop {
        assert!(size >= HoleList::min_size());

        let hole_addr = if hole.size == 0 {
            0
        } else {
            hole as *mut _ as usize
        };

        assert!(
            hole_addr + hole.size <= addr,
            "invalid deallocation (probably a double free)"
        );

        let next_hole_info = hole.next.as_ref().map(|next| next.info());

        match next_hole_info {
            Some(next) if hole_addr + hole.size == addr && addr + size == next.addr => {
                // block fills the gap between this hole and the next hole
                // before:  ___XXX____YYYYY____    where X is this hole and Y the next hole
                // after:   ___XXXFFFFYYYYY____    where F is the freed block
                hole.size += size + next.size; // merge the F and Y blocks to this X block
                hole.next = hole.next.as_mut().unwrap().next.take(); // remove the Y block
            },
            _ if hole_addr + hole.size == addr => {
                // block is right behind this hole but there is used memory after it
                // before:  ___XXX______YYYYY____    where X is this hole and Y the next hole
                // after:   ___XXXFFFF__YYYYY____    where F is the freed block

                // or: block is right behind this hole and this is the last hole
                // before:  ___XXX_______________    where X is this hole and Y the next hole
                // after:   ___XXXFFFF___________    where F is the freed block
                hole.size += size; // merge the F block to this X block
            },
            Some(next) if addr + size == next.addr => {
                // block is right before the next hole but there is used memory before it
                // before:  ___XXX______YYYYY____    where X is this hole and Y the next hole
                // after:   ___XXX__FFFFYYYYY____    where F is the freed block
                hole.next = hole.next.as_mut().unwrap().next.take(); // remove the Y block
                size += next.size; // free the merged F/Y block in next iteration
                continue;
            },
            Some(next) if next.addr <= addr => {
                // block is behind the next hole, so we delegate it to the next hole
                // before:  ___XXX__YYYYY________    where X is this hole and Y the next hole
                // after:   ___XXX__YYYYY__FFFF__    where F is the freed block
                hole = move_helper(hole).next.as_mut().unwrap(); // start next iteration at next hole
                continue;
            },
            _ => {
                // block is between this and the next hole
                // before:  ___XXX________YYYYY_    where X is this hole and Y the next hole
                // after:   ___XXX__FFFF__YYYYY_    where F is the freed block

                // or: this is the last hole
                // before:  ___XXX_________    where X is this hole
                // after:   ___XXX__FFFF___    where F is the freed block
                let new_hole = Hole {
                    size,
                    next: hole.next.take(), // the reference to the Y block (if it exists)
                };
                // write the new hole to the freed memory
                let ptr = addr as *mut Hole;
                unsafe { ptr.write(new_hole) };
                // add the F block as the next block of the X block
                hole.next = Some(unsafe { &mut *ptr });
            }
        }
        break;
    }
}

/// Identity function to ease moving of references.
///
/// By default, references are reborrowed instead of moved (equivalent to `&mut *reference`). This
/// function forces a move.
///
/// for more information, see section “id Forces References To Move” in:
/// https://bluss.github.io/rust/fun/2015/10/11/stuff-the-identity-function-does/
fn move_helper<T>(x: T) -> T {
    x
}