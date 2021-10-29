use core::ptr::{write_volatile, read_volatile};

#[macro_export]
macro_rules! write_mem {
    ($addr: expr, $val: expr) => {
        unsafe { core::ptr::write_volatile($addr, $val) }
    };
}

#[macro_export]
macro_rules! read_mem {
    ($addr: expr) => { unsafe { core::ptr::read_volatile($addr) } };
}

pub(crate) use write_mem;
pub(crate) use read_mem;