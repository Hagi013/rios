use core::alloc::{Allocator, Layout};
use core::ptr::{self, NonNull};
use core::pin::Pin;
use core::marker::PhantomData;
use core::mem;
use core::hash::Hasher;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::borrow;
use core::iter::FusedIterator;
use core::slice;

use alloc::alloc::handle_alloc_error;
use alloc::string::{String, ToString};
use alloc::borrow::ToOwned;

use crate::allocator::LockedHeap;
use crate::spin::mutex::Mutex;

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;

const DMA_START: usize = 0xb0000000;
const DMA_END: usize = 0xb1000000;

pub static mut DMA_ALLOCATOR: LockedHeap = LockedHeap {
    heap: Mutex::new(None)
};

pub fn init_dma() {
    let heap_start = DMA_START;
    let heap_size: usize = DMA_END - DMA_START;
    // unsafe { DMA_ALLOCATOR.init(heap_start, heap_size) };
    unsafe { DMA_ALLOCATOR.init_test(heap_start, heap_size) };
}

// ref: https://github.com/glandium/allocator_api/blob/master/src/liballoc/boxed.rs
pub struct DmaBox<T: ?Sized> {
    ptr: NonNull<T>,
    marker: PhantomData<T>,
    pub(crate) a: &'static LockedHeap,
}

impl<T> DmaBox<T> {
    #[inline(always)]
    pub fn new_in(x: T, a: &'static LockedHeap) -> DmaBox<T> {
        let mut a = a;
        let layout = Layout::for_value(&x);
        let size = layout.size();
        let p = if size == 0 {
            NonNull::dangling()
        } else {
            unsafe {
                let ptr = a.allocate(layout).unwrap_or_else(|_| { handle_alloc_error(layout) });
                ptr.cast()
            }
        };
        unsafe {
            ptr::write(p.as_ptr() as *mut T, x);
        }
        DmaBox {
            ptr: p,
            marker: PhantomData,
            a,
        }
    }

    #[inline(always)]
    pub fn pin_in(x: T, a: &'static LockedHeap) -> Pin<DmaBox<T>> {
        DmaBox::new_in(x, a).into()
    }

    #[inline(always)]
    pub fn new(x: T) -> DmaBox<T> {
        DmaBox::new_in(x, unsafe { &DMA_ALLOCATOR })
    }
}

// impl<T> DmaBox<T> {
//     #[inline(always)]
//     pub fn new(x: T) -> DmaBox<T> {
//         DmaBox::new_in(x,unsafe { &DMA_ALLOCATOR })
//     }
// }

// fn get_dma_box<T>(x: T) -> DmaBox<T> {
//     DmaBox::new_in(x,unsafe { &DMA_ALLOCATOR })
// }

impl<T: ?Sized> DmaBox<T> {
    #[inline]
    pub unsafe fn from_raw_in(raw: *mut T, a: &'static LockedHeap) -> Self {
        DmaBox {
            ptr: NonNull::new_unchecked(raw),
            marker: PhantomData,
            a,
        }
    }

    #[inline]
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        DmaBox::from_raw_in(raw, &DMA_ALLOCATOR)
    }

    #[inline]
    pub fn into_raw(b: DmaBox<T>) -> *mut T {
        DmaBox::into_raw_non_null(b).as_ptr()
    }

    #[inline]
    pub fn into_raw_non_null(b: DmaBox<T>) -> NonNull<T> {
        let ptr = b.ptr;
        mem::forget(ptr);
        ptr
    }

    #[inline]
    pub fn leak<'a>(b: DmaBox<T>) -> &'a mut T
    where
        T: 'a
    {
        unsafe { &mut *DmaBox::into_raw(b) }
    }

    pub fn into_pin(boxed: DmaBox<T>) -> Pin<DmaBox<T>> {
        unsafe { Pin::new_unchecked(boxed) }
    }
}

impl<T: ?Sized> Drop for DmaBox<T> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::for_value(self.ptr.as_ref());
            ptr::drop_in_place(self.ptr.as_ptr());
            if layout.size() != 0 {
                self.a.deallocate(self.ptr.cast(), layout);
            }
        }
    }
}

impl<T: Default> Default for DmaBox<T> {
    fn default() -> DmaBox<T> {
        DmaBox::new_in(Default::default(), unsafe { &DMA_ALLOCATOR })
    }
}

impl<T> Default for DmaBox<[T]> {
    fn default() -> DmaBox<[T]> {
        let a = unsafe { &DMA_ALLOCATOR };
        let b = DmaBox::<[T; 0]>::new_in([], a);
        let raw = b.ptr.as_ptr();
        let a = unsafe { ptr::read(&b.a) };
        mem::forget(b);
        unsafe { DmaBox::from_raw_in(raw as *mut [T], a) }
    }
}

#[inline]
pub unsafe fn from_boxed_utf8_unchecked(v: DmaBox<[u8]>) -> DmaBox<str> {
    let a = ptr::read(&v.a);
    DmaBox::from_raw_in(DmaBox::into_raw(v) as *mut str, a)
}

impl Default for DmaBox<str> {
    fn default() -> DmaBox<str> {
        unsafe { from_boxed_utf8_unchecked(Default::default()) }
    }
}

impl<T: Clone + Clone> Clone for DmaBox<T> {
    #[inline]
    fn clone(&self) -> DmaBox<T> {
        DmaBox::new_in((**self).clone(), &self.a)
    }

    #[inline]
    fn clone_from(&mut self, source: &DmaBox<T>) {
        (**self).clone_from(&(**source));
    }
}

// impl Clone for DmaBox<str> {
//     fn clone(&self) -> Self {
//         let len = self.len();
//         let buf = RawVec::with_capacity_in(len, self.a.clone());
//         unsafe {
//             ptr::copy_nonoverlapping(self.as_ptr(), buf.ptr(), len);
//             from_boxed_utf8_unchecked(buf.into_box());
//         }
//     }
// }

impl<T: ?Sized + Hasher> Hasher for DmaBox<T> {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes);
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i);
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i);
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i);
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i);
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i);
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i);
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i);
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i);
    }
}

impl<T> From<T> for DmaBox<T> {
    /// Converts a generic type `T` into a `Box<T>`
    fn from(t: T) -> Self {
        unsafe { DmaBox::new_in(t, &DMA_ALLOCATOR) }
    }
}

impl<T: ?Sized> From<DmaBox<T>> for Pin<DmaBox<T>> {
    /// Converts a `Box<T>` into a `Pin<Box<T>>`
    fn from(boxed: DmaBox<T>) -> Self {
        DmaBox::into_pin(boxed)
    }
}

impl<'a, T: Copy> From<&'a [T]> for DmaBox<[T]> {
    /// Converts a `&[T]` into a `Box<[T]>`
    fn from(slice: &'a [T]) -> DmaBox<[T]> {
        let a = unsafe { &DMA_ALLOCATOR };
        let mut boxed = unsafe { DmaRawVec::with_capacity_in(slice.len(), a).into_box() };
        boxed.copy_from_slice(slice);
        boxed
    }
}

impl<'a, T: Copy> DmaBox<[T]> {
    pub fn from_test(slice: &'a [T]) -> DmaBox<[T]> {
        let a = unsafe { &DMA_ALLOCATOR };
        let mut boxed = unsafe { DmaRawVec::with_capacity_in_test(slice.len(), a).into_box() };
        let mut printer = Printer::new(600, 620, 0);
        write!(printer, "{:?}", "hhhhhhhhh").unwrap();
        boxed.copy_from_slice(slice);
        let mut printer = Printer::new(600, 650, 0);
        write!(printer, "{:?}", "jjjjjjjjjjj").unwrap();
        boxed
    }
}

impl<'a> From<&'a str> for DmaBox<str> {
    /// Converts a `&str` into a `Box<str>`
    #[inline]
    fn from(s: &'a str) -> DmaBox<str> {
        unsafe { from_boxed_utf8_unchecked(DmaBox::from(s.as_bytes())) }
    }
}

impl From<DmaBox<str>> for DmaBox<[u8]> {
    /// Converts a `Box<str>` into a `Box<[u8]>`
    fn from(s: DmaBox<str>) -> Self {
        unsafe {
            let a = ptr::read(&s.a);
            DmaBox::from_raw_in(DmaBox::into_raw(s) as *mut [u8], a)
        }
    }
}

impl<T: fmt::Display + ?Sized> fmt::Display for DmaBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for DmaBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized> fmt::Pointer for DmaBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // It's not possible to extract the inner Uniq directly from the Box,
        // instead we cast it to a *const which aliases the Unique
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl<T: ?Sized> Deref for DmaBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for DmaBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<I: Iterator + ?Sized> Iterator for DmaBox<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
    fn nth(&mut self, nth: usize) -> Option<I::Item> {
        (**self).nth(nth)
    }
}

impl<I: DoubleEndedIterator + ?Sized> DoubleEndedIterator for DmaBox<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        (**self).next_back()
    }
}

impl<I:ExactSizeIterator + ?Sized> ExactSizeIterator for DmaBox<I> {
    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<I: FusedIterator + ?Sized> FusedIterator for DmaBox<I> {}

impl<T: Clone> Clone for DmaBox<[T]> {
    fn clone(&self) -> Self {
        let mut new = DmaBoxBuilder {
            data: DmaRawVec::with_capacity_in(self.len(), &self.a),
            len: 0,
        };

        let mut target = new.data.ptr();

        for item in self.iter() {
            unsafe {
                ptr::write(target, item.clone());
                target = target.offset(1);
            };
            new.len += 1;
        }
        return unsafe { new.into_box() };

        struct DmaBoxBuilder<T> {
            data: DmaRawVec<T>,
            len: usize,
        }

        impl<T> DmaBoxBuilder<T> {
            unsafe fn into_box(self) -> DmaBox<[T]> {
                let raw = ptr::read(&self.data);
                mem::forget(self);
                raw.into_box()
            }
        }

        impl<T> Drop for DmaBoxBuilder<T> {
            fn drop(&mut self) {
                let mut data = self.data.ptr();
                let max = unsafe { data.add(self.len) };
                while data != max {
                    unsafe {
                        ptr::read(data);
                        data = data.offset(1);
                    }
                }
            }
        }
    }
}

impl<T: ?Sized> borrow::Borrow<T> for DmaBox<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> borrow::BorrowMut<T> for DmaBox<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized> AsRef<T> for DmaBox<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> AsMut<T> for DmaBox<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

/* Nota bene
 *
 *  We could have chosen not to add this impl, and instead have written a
 *  function of Pin<Box<T>> to Pin<T>. Such a function would not be sound,
 *  because Box<T> implements Unpin even when T does not, as a result of
 *  this impl.
 *
 *  We chose this API instead of the alternative for a few reasons:
 *      - Logically, it is helpful to understand pinning in regard to the
 *        memory region being pointed to. For this reason none of the
 *        standard library pointer types support projecting through a pin
 *        (Box<T> is the only pointer type in std for which this would be
 *        safe.)
 *      - It is in practice very useful to have Box<T> be unconditionally
 *        Unpin because of trait objects, for which the structural auto
 *        trait functionality does not apply (e.g., Box<dyn Foo> would
 *        otherwise not be Unpin).
 *
 *  Another type with the same semantics as Box but only a conditional
 *  implementation of `Unpin` (where `T: Unpin`) would be valid/safe, and
 *  could have a method to project a Pin<T> from it.
 */
impl<T: ?Sized> Unpin for DmaBox<T> { }



pub struct DmaRawVec<T> {
    ptr: NonNull<T>,
    marker: PhantomData<T>,
    cap: usize,
    a: &'static LockedHeap,
}

impl<T> DmaRawVec<T> {
    pub fn new_in(a: &'static LockedHeap) -> Self {
        DmaRawVec {
            ptr: NonNull::dangling(),
            marker: PhantomData,
            cap: [0, !0][(mem::size_of::<T>() == 0) as usize],
            a
        }
    }

    #[inline]
    pub fn with_capacity_in(cap: usize, a: &'static LockedHeap) -> Self {
        DmaRawVec::allocate_in(cap, false, a)
    }

    #[inline]
    pub fn with_capacity_in_test(cap: usize, a: &'static LockedHeap) -> Self {
        DmaRawVec::allocate_in_test(cap, false, a)
    }

    #[inline]
    pub fn with_capacity_zeroed_in(cap: usize, a: &'static LockedHeap) -> Self {
        DmaRawVec::allocate_in(cap, true, a)
    }

    fn allocate_in(cap: usize, zeroed: bool, mut a: &'static LockedHeap) -> Self {
        unsafe {
            let elem_size = mem::size_of::<T>();

            let alloc_size = cap.checked_mul(elem_size).unwrap_or_else(|| capacity_overflow());
            alloc_guard(alloc_size).unwrap_or_else(|_| capacity_overflow());

            let ptr = if alloc_size == 0 {
                NonNull::<T>::dangling()
            } else {
                let align = mem::align_of::<T>();
                let layout = Layout::from_size_align(alloc_size, align).unwrap();
                let result = if zeroed {
                    a.allocate_zeroed(layout)
                } else {
                    a.allocate(layout)
                };
                match result {
                    Ok(ptr) => ptr.cast(),
                    Err(e) => {
                        // let mut printer = Printer::new(100, 700, 0);
                        // write!(printer, "{:?}", e.to_string()).unwrap();
                        handle_alloc_error(layout)
                    },
                }
            };
            DmaRawVec {
                ptr: ptr.into(),
                marker: PhantomData,
                cap,
                a
            }
        }
    }

    fn allocate_in_test(cap: usize, zeroed: bool, mut a: &'static LockedHeap) -> Self {
        unsafe {
            let elem_size = mem::size_of::<T>();

            let alloc_size = cap.checked_mul(elem_size).unwrap_or_else(|| capacity_overflow());
            alloc_guard(alloc_size).unwrap_or_else(|_| capacity_overflow());

            let ptr = if alloc_size == 0 {
                NonNull::<T>::dangling()
            } else {
                let mut printer = Printer::new(600, 635, 0);
                write!(printer, "{:x}", alloc_size).unwrap();
                let align = mem::align_of::<T>();
                let layout = Layout::from_size_align(alloc_size, align).unwrap();

                let mut printer = Printer::new(500, 620, 0);
                write!(printer, "{:?}", elem_size).unwrap();
                let mut printer = Printer::new(500, 635, 0);
                write!(printer, "{:?}", alloc_size).unwrap();
                let mut printer = Printer::new(500, 650, 0);
                write!(printer, "{:?}", align).unwrap();

                let result = if zeroed {
                    a.allocate_zeroed(layout)
                } else {
                    a.allocate_test(layout)
                };
                let mut printer = Printer::new(600, 680, 0);
                write!(printer, "{:?}", "iiiiiiiiii").unwrap();
                match result {
                    Ok(ptr) => ptr.cast(),
                    Err(e) => {
                        // let mut printer = Printer::new(100, 700, 0);
                        // write!(printer, "{:?}", e.to_string()).unwrap();
                        handle_alloc_error(layout)
                    },
                }
            };
            let mut printer = Printer::new(600, 680, 0);
            write!(printer, "{:x}", &ptr as *const NonNull<T> as u8).unwrap();
            let mut printer = Printer::new(600, 695, 0);
            write!(printer, "{:?}", cap).unwrap();
            DmaRawVec {
                ptr: ptr.into(),
                marker: PhantomData,
                cap,
                a
            }
        }
    }
}

impl<T> DmaRawVec<T> {
    pub fn new() -> Self {
        Self::new_in(unsafe { &DMA_ALLOCATOR })
    }

    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        DmaRawVec::allocate_in(cap, false, unsafe { &DMA_ALLOCATOR })
    }

    #[inline]
    pub fn with_capacity_test(cap: usize) -> Self {
        DmaRawVec::allocate_in_test(cap, false, unsafe { &DMA_ALLOCATOR })
    }

    #[inline]
    pub fn with_capacity_zeroed(cap: usize) -> Self {
        DmaRawVec::allocate_in(cap, true, unsafe { &DMA_ALLOCATOR })
    }

    /// Reconstitutes a DmaRawVec from a pointer, capacity, and allocator.
    pub unsafe fn from_raw_parts_in(ptr: *mut T, cap: usize, a: &'static LockedHeap) -> Self {
        DmaRawVec {
            ptr: NonNull::new_unchecked(ptr),
            marker: PhantomData,
            cap,
            a,
        }
    }

    pub unsafe fn from_raw_parts(ptr: *mut T, cap: usize) -> Self {
        Self::from_raw_parts_in(ptr, cap, &DMA_ALLOCATOR)
    }

    pub fn from_box(mut slice: DmaBox<[T]>) -> Self {
        unsafe {
            let a = ptr::read(&slice.a);
            let result = DmaRawVec::from_raw_parts_in(slice.as_mut_ptr(), slice.len(), a);
            mem::forget(slice);
            result
        }
    }

    pub fn ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    #[inline(always)]
    pub fn cap(&self) -> usize {
        if mem::size_of::<T>() == 0 {
            !0
        } else {
            self.cap
        }
    }

    pub fn alloc(&self) -> &LockedHeap {
        self.a
    }

    // pub fn alloc_mut(&mut self) -> &mut LockedHeap {
    //     &mut self.a
    // }

    fn current_layout(&self) -> Option<Layout> {
        if self.cap == 0 {
            None
        } else {
            unsafe {
                let align = mem::align_of::<T>();
                let size = mem::size_of::<T>() * self.cap;
                Some(Layout::from_size_align_unchecked(size, align))
            }
        }
    }

    pub unsafe fn into_box(self) -> DmaBox<[T]> {
        let slice = slice::from_raw_parts_mut(self.ptr(), self.cap);
        let a = ptr::read(&self.a);
        let output: DmaBox<[T]> = DmaBox::from_raw_in(slice, a);
        mem::forget(self);
        output
    }

    pub unsafe fn dealloc_buffer(&mut self) {
        let elem_size = mem::size_of::<T>();
        if elem_size != 0 {
            if let Some(layout) = self.current_layout() {
                self.a.deallocate(NonNull::from(self.ptr).cast(), layout);
            }
        }
    }

    fn drop(&mut self) {
        unsafe { self.dealloc_buffer() }
    }
}



#[inline]
fn alloc_guard(alloc_size: usize) -> Result<(), String> {
    if mem::size_of::<usize>() < 8 && alloc_size > core::isize::MAX as usize {
        Err("Capacity Overflow".to_owned())
    } else {
        Ok(())
    }
}

fn capacity_overflow() -> ! {
    panic!("DmaRawVec capacity overflow")
}

