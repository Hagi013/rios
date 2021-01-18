use super::asmfunc;
use super::pic;
use super::graphic::Graphic;

extern "C" {
    pub fn asm_inthandler02();
    pub fn asm_inthandler04();
    pub fn asm_inthandler05();
    pub fn asm_inthandler06();
    pub fn asm_inthandler07();
    pub fn asm_inthandler08();
    pub fn asm_inthandler0a();
    pub fn asm_inthandler0b();
    pub fn asm_inthandler0c();
    pub fn asm_inthandler0d();
    pub fn asm_inthandler0e();
    pub fn asm_inthandler0f();
    pub fn asm_inthandler10();
    pub fn asm_inthandler11();
    pub fn asm_inthandler12();
    pub fn asm_inthandler13();
    pub fn asm_inthandler20();
    pub fn asm_inthandler21();
    pub fn asm_inthandler27();
    pub fn asm_inthandler2c();
}


const ADR_GDT: u32 = 0x00270000;
const LIMIT_GDT: usize = 0x0000ffff;
const ADR_OSPAK: u32 = 0x00000000;
const LIMIT_OSPAK: u32 = 0x00280000 + 0x0007ffff;
const AR_DATA32_RW: u32 = 0x4092;
const AR_CODE32_ER: u32 = 0x409a;

const ADR_IDT: u32 = 0x0026f800;
const LIMIT_IDT: usize = 0x000007ff;
const AR_INTGATE32: u32 = 0x008e;

#[repr(C, packed)]
struct SegmentDescriptorEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access_right: u8,
    limit_high: u8,
    base_high: u8,
}

#[repr(C, packed)]
struct GateDescriptorEntry {
    offset_low: u16,
    selector: u16,
    dw_count: u8,
    access_right: u8,
    offset_high: u16,
}

pub struct DscTbl {
    segment_descriptor_table: [*mut SegmentDescriptorEntry; LIMIT_GDT / 8],
    gate_descriptor_table: [*mut GateDescriptorEntry; LIMIT_IDT / 8],
}

impl DscTbl {
    fn new() -> Self {
        DscTbl {
            segment_descriptor_table: DscTbl::init_gdt(),
            gate_descriptor_table: DscTbl::init_idt(),
        }
    }

    pub fn init_gdt_idt() -> Self {
        return DscTbl::new();
    }

    fn init_gdt() -> [*mut SegmentDescriptorEntry; LIMIT_GDT / 8] {
        let mut segment_descriptor_table: [*mut SegmentDescriptorEntry; LIMIT_GDT / 8] = [0 as *mut SegmentDescriptorEntry; LIMIT_GDT / 8];
        (0..(LIMIT_GDT / 8))
            .map(|idx| DscTbl::set_segmdesc(idx as u32, 0, 0, 0))
            .zip(segment_descriptor_table.iter_mut());
//            .for_each(|(setted_entry, init_table_entry)| *init_table_entry = setted_entry);

        segment_descriptor_table[1] = DscTbl::set_segmdesc(1, 0xffffffff, 0x00000000, AR_DATA32_RW);
        segment_descriptor_table[2] = DscTbl::set_segmdesc(2, LIMIT_OSPAK, ADR_OSPAK, AR_CODE32_ER);

        asmfunc::load_gdtr(LIMIT_GDT as u32, ADR_GDT);
        return segment_descriptor_table
    }

    fn set_segmdesc(idx: u32, mut limit: u32, base: u32, mut ar: u32) -> *mut SegmentDescriptorEntry {
        if limit > 0xfffff {
            ar |= 0x8000; /* G_bit = 1 */
            limit /= 0x1000;
        }

        let base_address: u32 = ADR_GDT + (idx * 8);
        let seg_dsc_entry: *mut SegmentDescriptorEntry = base_address as *mut SegmentDescriptorEntry;
        unsafe {
            (*seg_dsc_entry).limit_low = (limit & 0xffff) as u16;
            (*seg_dsc_entry).base_low = (base & 0xffff) as u16;
            (*seg_dsc_entry).base_mid = ((base >> 16) & 0xff) as u8;
            (*seg_dsc_entry).access_right = (ar & 0xff) as u8;
            (*seg_dsc_entry).limit_high = (((limit >> 16) & 0xff) | ((ar >> 8) & 0xf0)) as u8;
            (*seg_dsc_entry).base_high = ((base >> 24) & 0xff) as u8;
        }
        return seg_dsc_entry;
    }

    fn init_idt() -> [*mut GateDescriptorEntry; LIMIT_IDT / 8] {
        let mut gate_descriptor_table: [*mut GateDescriptorEntry; LIMIT_IDT / 8] = [0 as *mut GateDescriptorEntry; LIMIT_IDT / 8];
        (0..(LIMIT_IDT / 8))
            .map(|idx| DscTbl::set_gatedesc(idx as u32, 0, 0, 0))
            .zip(gate_descriptor_table.iter_mut())
            .for_each(|(b, df)| *df = b);

        asmfunc::load_idtr(LIMIT_IDT as u32, ADR_IDT);

        gate_descriptor_table[0x04] = DscTbl::set_fn_gatedesc(0x04 as u32, asm_inthandler04, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x05] = DscTbl::set_fn_gatedesc(0x05 as u32, asm_inthandler05, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x06] = DscTbl::set_fn_gatedesc(0x06 as u32, asm_inthandler06, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x07] = DscTbl::set_fn_gatedesc(0x07 as u32, asm_inthandler07, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x08] = DscTbl::set_fn_gatedesc(0x08 as u32, asm_inthandler08, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x0a] = DscTbl::set_fn_gatedesc(0x0a as u32, asm_inthandler0a, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x0b] = DscTbl::set_fn_gatedesc(0x0b as u32, asm_inthandler0b, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x0c] = DscTbl::set_fn_gatedesc(0x0c as u32, asm_inthandler0c, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x0d] = DscTbl::set_fn_gatedesc(0x0d as u32, asm_inthandler0d, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x0e] = DscTbl::set_fn_gatedesc(0x0e as u32, asm_inthandler0e, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x10] = DscTbl::set_fn_gatedesc(0x10 as u32, asm_inthandler10, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x11] = DscTbl::set_fn_gatedesc(0x11 as u32, asm_inthandler11, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x12] = DscTbl::set_fn_gatedesc(0x12 as u32, asm_inthandler12, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x13] = DscTbl::set_fn_gatedesc(0x13 as u32, asm_inthandler13, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x20] = DscTbl::set_fn_gatedesc(0x20 as u32, asm_inthandler20, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x21] = DscTbl::set_fn_gatedesc(0x21 as u32, asm_inthandler21, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x27] = DscTbl::set_fn_gatedesc(0x27 as u32, asm_inthandler27, 2 * 8, AR_INTGATE32);
        gate_descriptor_table[0x2c] = DscTbl::set_fn_gatedesc(0x2c as u32, asm_inthandler2c, 2 * 8, AR_INTGATE32);
        return gate_descriptor_table
    }

    fn set_gatedesc(idx: u32, offset: u32, selector: u16, ar: u32) -> *mut GateDescriptorEntry {
        let base_address: u32 = (ADR_IDT + (idx * 8));
        let gate_dsc_entry: *mut GateDescriptorEntry = base_address as *mut GateDescriptorEntry;
        unsafe {
            (*gate_dsc_entry).offset_low = (offset & 0xffff) as u16;
            (*gate_dsc_entry).selector = selector;
            (*gate_dsc_entry).dw_count = ((ar >> 8) & 0xff) as u8;
            (*gate_dsc_entry).access_right = (ar & 0xff) as u8;
            (*gate_dsc_entry).offset_high = ((offset >> 16) & 0xffff) as u16;
        }
        return gate_dsc_entry;
    }

    fn set_fn_gatedesc(idx: u32, func: unsafe extern fn(), selector: u16, ar: u32) -> *mut GateDescriptorEntry {
        let offset: u32 = func as u32;
        return DscTbl::set_gatedesc(idx, offset, selector, ar);
    }
}

