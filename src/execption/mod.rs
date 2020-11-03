use core::fmt::Write;

use super::Graphic;
use super::Printer;
use super::asmfunc;

#[no_mangle]
pub extern "C" fn non_maskable_interrupt_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "non_maskable_interrupt_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn overflow_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "overflow_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn bounds_check_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "bounds_check_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn undefined_operation_code_instruction_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "undefined_operation_code_instruction_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn no_coprocessor_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "undefined_operation_code_instruction_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn double_fault_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "double_fault_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn invalid_tss_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "invalid_tss_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn segment_not_present_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "segment_not_present_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn stack_segment_fault_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "stack_segment_fault_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn general_protection_error_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "general_protection_error_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn coprocessor_error_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "coprocessor_error_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn alignment_check_error_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "alignment_check_error_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn machine_check_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "machine_check_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}

#[no_mangle]
pub extern "C" fn simd_fpu_exception_handler(esp: *const usize) {
    Graphic::putfont_asc(0, 180, 10, "simd_fpu_exception_handler!!!!!");
    let mut printer = Printer::new(0, 200, 10);
    write!(printer, "{:?}", unsafe { esp.offset(11) }).unwrap();
    loop {
        asmfunc::io_hlt();
    }
}
