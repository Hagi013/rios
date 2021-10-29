use core::slice::from_raw_parts;
use alloc::vec::Vec;
use core::mem::size_of;

pub fn switch_endian32(src: u32) -> u32 {
    // 0xee11ff22 => 0x22ff11ee
    (src) >> 24 & 0x000000ff |
    (src) << 8  & 0x00ff0000 |
    (src) >> 8  & 0x0000ff00 |
    (src) << 24 & 0xff000000
}

pub fn switch_endian16(src: u16) -> u16 {
    // ff22 => 0x22ff
    (src) << 8 | (src) >> 8
}

pub fn any_as_u8_vec<T: Sized>(p: &T) -> Vec<u8> {
    // let slice = unsafe {
    //     from_raw_parts(
    //         (p as *const T) as *const u8,
    //         size_of::<T>(),
    //     )
    // };
    // slice.to_vec()
    any_as_u8_slice(p).to_vec()
}

pub fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe {
        from_raw_parts(
            (p as *const T) as *const u8,
            size_of::<T>(),
        )
    }
}

pub fn push_to_vec(mut src: Vec<u8>, mut dst: Vec<u8>) -> Vec<u8> {
    for b in src.as_slice() {
        unsafe {
            dst.push(*b);
        }
    }
    dst
}
