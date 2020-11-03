use core::slice::Iter;
use core::cell::Cell;
use core::fmt;
// use core::fmt::{ Write, Result };

//use alloc::vec::Vec;

use super::asmfunc;
use super::boot_info::BootInfo;
use super::hankaku;
#[macro_use]
use core::fmt::{ Write, Display };

const ADR_BOOTINFO: u32 = 0x00000ff0;
static mut scrnx: u16 = (ADR_BOOTINFO + 0x04) as u16;
static mut scrny: u16 = (ADR_BOOTINFO + 0x06) as u16;
static mut vram: u32 = (ADR_BOOTINFO + 0x08) as u32;

fn get_scrnx() -> &'static u16 {
    return unsafe { &*(scrnx as *const u16) }
}

fn get_scrny() -> &'static u16 {
    return unsafe { &*(scrny as *const u16) }
}

fn get_vram() -> &'static u32 {
    return unsafe { &*(vram as *const u32) }
}

static FONTS: [u8; 4096] = hankaku::FONTS;
static HANKAKU_TABLE: [&str; 0x80] = hankaku::HANKAKU_TABLE;
static HANKAKU_SHIFT_TABLE: [&str; 0x80] = hankaku::HANKAKU_SHIFT_TABLE;

pub struct Graphic {}
impl Graphic {
    pub fn new() -> Self {
        Graphic {}
    }

    pub fn init() {
        Graphic::set_palette();
        Graphic::init_screen();
    }

    fn set_palette() {
        // 割り込み許可フラグの値を記録する
        let eflags: u32 = asmfunc::io_load_eflags();
        asmfunc::io_cli(); // 割り込みを禁止する
        asmfunc::io_out8(0x03c8, 0);

        for rgb in RGB::iterator() {
            asmfunc::io_out8(0x03c9, (rgb.r() >> 2) as u8);
            asmfunc::io_out8(0x03c9, (rgb.b() >> 2) as u8);
            asmfunc::io_out8(0x03c9, (rgb.b() >> 2) as u8);
        }
         asmfunc::io_store_eflags(eflags);
    }

    pub fn init_screen() {
        let x: &u16 = get_scrnx();
        let y: &u16 = get_scrny();

        Graphic::boxfill(RGB::DarkLightBlue.palette_no(), 0, 0,      x - 1, y - 29);
        Graphic::boxfill(RGB::Gray.palette_no(),          0, y - 28, x - 1, y - 28);
        Graphic::boxfill(RGB::White.palette_no(),         0, y - 27, x - 1, y - 27);
        Graphic::boxfill(RGB::Gray.palette_no(),          0, y - 26, x - 1, y -  1);

        Graphic::boxfill(RGB::White.palette_no(),         3, y - 24,    59, y - 24);
        Graphic::boxfill(RGB::White.palette_no(),         2, y - 24,     2, y -  4);
        Graphic::boxfill(RGB::DarkGray.palette_no(),      3, y -  4,    59, y -  4);
        Graphic::boxfill(RGB::DarkGray.palette_no(),     59, y - 23,    59, y -  5);
        Graphic::boxfill(RGB::Black.palette_no(),         2, y -  3,    59, y -  3);
        Graphic::boxfill(RGB::Black.palette_no(),        60, y - 24,    60, y -  3);

        Graphic::boxfill(RGB::DarkGray.palette_no(), x - 47, y - 24, x - 4, y - 24);
        Graphic::boxfill(RGB::DarkGray.palette_no(), x - 47, y - 23, x -47, y -  4);
        Graphic::boxfill(RGB::White.palette_no(),    x - 47, y -  3, x - 4, y -  3);
        Graphic::boxfill(RGB::White.palette_no(),    x -  3, y - 24, x - 3, y -  3);
    }

    fn boxfill(color: u8, from_x: u16, from_y: u16, to_x: u16, to_y: u16) {
        for y in (from_y as u32)..(to_y as u32) {
            for x in (from_x as u32)..(to_x as u32) {
                 let address: *mut u8 = (get_vram() + (y * *get_scrnx() as u32) + x) as *mut u8;
                unsafe { *address = color; }
            }
        }
    }

    pub fn putfont_asc_from_keyboard(x: u32, y: u32, c: u8, data: i32) {
        if data >= 0x80 {
            return;
        }
        let font: &str = HANKAKU_TABLE[data as usize];
        Graphic::putfont_asc(x, y, c, font);
    }

    pub fn putfont_asc(x: u32, y: u32, c: u8, s: &str) {
        let mut idx: u32 = 0;
        for byte in s.bytes().into_iter() {
            Graphic::putfont_color((x + (8 * idx)), y, &c, ((byte as u16) * 16) as usize);
            idx += 1;
        }
    }

//    pub fn putfont_asc(&self, x: u16, y: u16, c: u8) {
//        // let mut idx: isize = 0x5a;
//        let mut idx: isize = 97;
//        self.putfont(&x, &y, &c, (&idx * 16) as usize);
//    }

    fn putfont_color(x: u32, y: u32, c: &u8, idx: usize) {
        for i in 0..16 {
            let mut address = (get_vram() + ((y + i) * *get_scrnx() as u32 + x)) as *mut u8;
            let d: u8 = FONTS[idx + i as usize];
            unsafe {
                if (d & 0x80) != 0 { *(address.offset(0)) = *c }
                if (d & 0x40) != 0 { *(address.offset(1)) = *c }
                if (d & 0x20) != 0 { *(address.offset(2)) = *c }
                if (d & 0x10) != 0 { *(address.offset(3)) = *c }
                if (d & 0x08) != 0 { *(address.offset(4)) = *c }
                if (d & 0x04) != 0 { *(address.offset(5)) = *c }
                if (d & 0x02) != 0 { *(address.offset(6)) = *c }
                if (d & 0x01) != 0 { *(address.offset(7)) = *c }
            }
        }
    }

    fn putblock(pxsize: u32, pysize: u32, px0: u32, py0: u32, block_buf: *const u8, bxsize: u32) {
        for y in 0..pysize {
            for x in 0..pxsize {
                let mut address = (get_vram() + ((py0 + y) * *get_scrnx() as u32 + (px0 + x)) as u32) as *mut u8;
                unsafe {
                    *address = *block_buf.offset((y * bxsize + x) as isize)
                }
            }
        }
    }
}

pub struct MouseGraphic {
    mouse_buf: Cell<[u8; 256]>
}

impl MouseGraphic {
    pub fn new() -> Self {
        MouseGraphic {
            mouse_buf: Cell::new([0x0; 256])
        }
    }

    pub fn init_mouse_cursor(&self, c: u8) -> (*mut u8, i32, i32, u16, u16) {
        static MOUSE_CURSOR: [[u8; 16]; 16] = [
            [1,1,1,1,1,1,1,1,1,1,1,1,1,1,2,2],  // 0 - 15
            [1,0,0,0,0,0,0,0,0,0,0,0,1,2,2,2],  // 16 - 31
            [1,0,0,0,0,0,0,0,0,0,0,1,2,2,2,2],  // 32 - 47
            [1,0,0,0,0,0,0,0,0,0,1,2,2,2,2,2],  // 48 - 63
            [1,0,0,0,0,0,0,0,0,1,2,2,2,2,2,2],  // 64 - 79
            [1,0,0,0,0,0,0,0,1,2,2,2,2,2,2,2],  // 80 - 96
            [1,0,0,0,0,0,0,0,1,2,2,2,2,2,2,2],  // 96 - 111
            [1,0,0,0,0,0,0,0,0,1,2,2,2,2,2,2],  // 112 - 127
            [1,0,0,0,0,1,1,0,0,0,1,2,2,2,2,2],  // 128 - 143
            // 132からは正常な値がセットされるっぽい
            // =============================
            [1,0,0,0,1,2,2,1,0,0,0,1,2,2,2,2],  // 144 - 159
            [1,0,0,1,2,2,2,2,1,0,0,0,1,2,2,2],  // 160 - 175
            [1,0,1,2,2,2,2,2,2,1,0,0,0,1,2,2],  // 176 - 191
            [1,1,2,2,2,2,2,2,2,2,1,0,0,0,1,2],  // 192 - 207
            [1,2,2,2,2,2,2,2,2,2,2,1,0,0,0,1],  // 208 - 223
            [2,2,2,2,2,2,2,2,2,2,2,2,1,0,0,1],  // 224 - 239
            [2,2,2,2,2,2,2,2,2,2,2,2,2,1,1,1]  // 240 - 255
        ];

        let mut mouse_buf: [u8; 256] = self.mouse_buf.get();
        for (y, cursor_array) in MOUSE_CURSOR.iter().enumerate() {
            for (x, cursor_char) in cursor_array.iter().enumerate() {
                if *cursor_char == 1 {
                    mouse_buf[y * 16 + x] = RGB::Black.palette_no();
                }
                if *cursor_char == 0 {
                    mouse_buf[y * 16 + x] = RGB::White.palette_no();
                }
                if *cursor_char == 2 {
                    mouse_buf[y * 16 + x] = c;
                }
            }
        }
        self.mouse_buf.set(mouse_buf);

        // ToDo ここは後で移動する
//        let mx = (get_scrnx() - 16) / 2;   /* 画面中央になるように座標計算 */
//        let my = (get_scrny() - 28 - 16) / 2;
        let mx: u32 = (*get_scrnx() as u32 - 16) / 2;   /* 画面中央になるように座標計算 */
        let my: u32 = (*get_scrny() as u32 - 28 - 16) / 2;
        Graphic::putblock(16, 16, mx, my, &mouse_buf as *const u8, 16);
//        return (&mouse_buf as *const u8 as *mut u8, mx as i32, my as i32, 16, 16);
        return (&mouse_buf as *const u8 as *mut u8, mx as i32, my as i32, 16, 16);
    }
}


enum RGB {
    Black,          /*  0:黒 */
    Red,            /*  1:明るい赤 */
    Green,          /*  2:明るい緑 */
    Yellow,         /*  3:明るい黄色 */
    Blue,           /*  4:明るい青 */
    Purple,         /*  5:明るい紫 */
    LightBlue,      /*  6:明るい水色 */
    White,          /*  7:白 */
    Gray,           /*  8:明るい灰色 */
    DarkRed,        /*  9:暗い赤 */
    DarkGreen,      /* 10:暗い緑 */
    DarkYellow,     /* 11:暗い黄色 */
    DarkBlue,       /* 12:暗い青 */
    DarkPurple,     /* 13:暗い紫 */
    DarkLightBlue,  /* 14:暗い水色 */
    DarkGray,       /* 15:暗い灰色 */
}

impl RGB {
    fn value(&self) -> RGBElement {
        match self {
            RGB::Black => RGBElement::new(0x00, 0x00, 0x00),
            RGB::Red => RGBElement::new(0xff, 0x00, 0x00),
            RGB::Green => RGBElement::new(0x00, 0xff, 0x00),
            RGB::Yellow => RGBElement::new(0xff, 0xff, 0x00),
            RGB::Blue => RGBElement::new(0x00, 0x00, 0xff),
            RGB::Purple => RGBElement::new(0xff, 0x00, 0xff),
            RGB::LightBlue => RGBElement::new(0x00, 0xff, 0xff),
            RGB::White => RGBElement::new(0xff, 0xff, 0xff),
            RGB::Gray => RGBElement::new(0xc6, 0xc6, 0xc6),
            RGB::DarkRed => RGBElement::new(0x84, 0x00, 0x00),
            RGB::DarkGreen => RGBElement::new(0x00, 0x84, 0x00),
            RGB::DarkYellow => RGBElement::new(0x84, 0x84, 0x00),
            RGB::DarkBlue => RGBElement::new(0x00, 0x00, 0x84),
            RGB::DarkPurple => RGBElement::new(0x84, 0x00, 0x84),
            RGB::DarkLightBlue => RGBElement::new(0x00, 0x84, 0x84),
            RGB::DarkGray => RGBElement::new(0x84, 0x84, 0x84),
        }
    }

    fn palette_no(&self) -> u8 {
        match self {
            RGB::Black => 0,
            RGB::Red => 1,
            RGB::Green => 2,
            RGB::Yellow => 3,
            RGB::Blue => 4,
            RGB::Purple => 5,
            RGB::LightBlue => 6,
            RGB::White => 7,
            RGB::Gray => 8,
            RGB::DarkRed => 9,
            RGB::DarkGreen => 10,
            RGB::DarkYellow => 11,
            RGB::DarkBlue => 12,
            RGB::DarkPurple => 13,
            RGB::DarkLightBlue =>14,
            RGB::DarkGray => 15,
        }
    }

    fn r(&self) -> u8 { self.value().r }

    fn g(&self) -> u8 { self.value().g }

    fn b(&self) -> u8 { self.value().b }

    fn iterator() -> Iter<'static, RGB> {
        static RGBS: [RGB; 16] = [
            RGB::Black,          /*  0:黒 */
            RGB::Red,            /*  1:明るい赤 */
            RGB::Green,          /*  2:明るい緑 */
            RGB::Yellow,         /*  3:明るい黄色 */
            RGB::Blue,           /*  4:明るい青 */
            RGB::Purple,         /*  5:明るい紫 */
            RGB::LightBlue,      /*  6:明るい水色 */
            RGB::White,          /*  7:白 */
            RGB::Gray,           /*  8:明るい灰色 */
            RGB::DarkRed,        /*  9:暗い赤 */
            RGB::DarkGreen,      /* 10:暗い緑 */
            RGB::DarkYellow,     /* 11:暗い黄色 */
            RGB::DarkBlue,       /* 12:暗い青 */
            RGB::DarkPurple,     /* 13:暗い紫 */
            RGB::DarkLightBlue,  /* 14:暗い水色 */
            RGB::DarkGray,       /* 15:暗い灰色 */
        ];
        return RGBS.into_iter();
    }
}

struct RGBElement {
    r: u8,
    g: u8,
    b: u8,
}

impl RGBElement {
    fn new(r: u8, g: u8, b: u8) -> Self {
        RGBElement {
            r,
            g,
            b,
        }
    }
}

pub struct Printer {
    width: u32,
    height: u32,
    color: u8,
}

impl Printer {
    pub fn new(width: u32, height: u32, color: u8) -> Self {
        Printer {
            width,
            height,
            color,
        }
    }
}

impl fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Graphic::putfont_asc(self.width, self.height, self.color, s);
        Ok(())
    }
}
