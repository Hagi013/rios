use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed;

use core::mem::replace;

use super::arch::boot_info::BootInfo;

use super::Printer;
use core::fmt;
use core::fmt::Write;

use super::util::linked_list::LinkedList;
use core::borrow::{Borrow, BorrowMut};
use core::ptr;
use alloc::borrow::ToOwned;

use super::get_uptime;


/* ToDo
    0. windowを受け取った時のrendering
        - 実際の色付けや位置決めはwindowの管理外で、window.bufに含める
        - buf配列要素に対する添字(アドレス)は、それぞれbase_x, base_y, xsize, ysizeで決まる
        - 受け取った段階で一番上にrenderingさせる
    1. windowの追加時のrendering
    2. slide時のrendering
    3. top down時のrendering
        - どうやって順番を管理するか
*/
pub struct WindowsManager {
    linked_list: LinkedList<Window>,
    windows_map: Vec<usize>,
    scrnx: u16,
    scrny: u16,
    vram: u32,
}

impl WindowsManager {
    pub fn new() -> Self {
        let mut b: BootInfo = BootInfo::new();
        let mut window_size: usize = 0;
        window_size = (b.scrnx as usize) * (b.scrny as usize) + (b.scrnx as usize);

        let mut initial_buf: Vec<u8> = Vec::new();

        for i in 0..(b.scrnx as usize) * (b.scrny as usize) {
            let address = (b.vram + i as u32) as *mut u8;
            initial_buf.push(unsafe { *address });
        }
        let buf = (boxed::Box::into_raw(initial_buf.into_boxed_slice())) as *mut u8;
        let mut base_window: Window = Window::new(0, 0, b.scrnx, b.scrny, buf);
        base_window.id = 0;

        let mut linked_list: LinkedList<Window> = LinkedList::new();
        linked_list.add(boxed::Box::into_raw(boxed::Box::new(base_window)));
        let mut windows_map: Vec<usize> = vec![base_window.id; window_size];
        WindowsManager {
            linked_list,
            windows_map,
            scrnx: b.scrnx,
            scrny: b.scrny,
            vram: b.vram,
        }
    }

pub fn create_window(&mut self, base_x: i32, base_y: i32, xsize: u16, ysize: u16, buf: *mut u8) -> Result<*mut Window, String> {
    let mut n_w = Window::new(base_x, base_y, xsize, ysize, buf);
    let n_w_address = boxed::Box::into_raw(boxed::Box::new(n_w)) as *mut Window;
    self.add(n_w_address);
    let height=  self.linked_list.get_position_from_data(n_w_address).ok_or("add method is not executed properly in create_window".to_owned())?;
    self.refresh_map(base_x, base_y, n_w_address, height, 66);
    return Ok(n_w_address);
    }

//    pub fn close_window(&mut self, buf: *mut u8) -> Result<(), String> {
//    }

    // addするときは一番最後に入れる
    fn add(&mut self, n_w: *mut Window) -> Result<(), String> {
        return self.linked_list.add(n_w);
    }

    pub fn remove(&mut self, t_w: *mut Window) -> Result<(), String> {
        return self.linked_list.remove(t_w);
    }

    pub fn refresh_map(&mut self, base_x: i32, base_y: i32, w_pointer: *mut Window, height: usize, num: usize) -> Result<(), String> {
        let w_r = unsafe { *w_pointer };
        let (mut from_x, mut from_y, mut to_x, mut to_y) = self.get_adjusted_position(base_x, base_y, w_r.xsize, w_r.ysize);

        for h in height..self.linked_list.len() {
            let w_p = self.linked_list.get_data_from_position(h).ok_or("".to_owned()).clone()?;
            let w = unsafe { *w_p };

            let win_from_x = if (from_x as isize) - (w.get_base_x() as isize) < 0 { 0 } else { (from_x as isize) - (w.get_base_x() as isize) };
            let win_from_y = if (from_y as isize) - (w.get_base_y() as isize) < 0 { 0 } else { (from_y as isize) - (w.get_base_y() as isize) };
            let win_to_x = unsafe {
                if (to_x as isize) - (w.get_base_x() as isize) > (w.xsize as isize) {
                    w.xsize as isize
                } else {
                    (to_x as isize - w.get_base_x() as isize)
                }
            };
            let win_to_y = unsafe {
                if (to_y as isize) - (w.get_base_y() as isize) > (w.ysize as isize) {
                    w.ysize as isize
                } else {
                    (to_y as isize - w.get_base_y() as isize)
                }
            };

            for y in win_from_y..win_to_y {
                let vy = w.get_base_y() as usize + y as usize;
                for x in win_from_x..win_to_x {
                    let vx = w.get_base_x() as usize + x as usize;
                    self.windows_map[vy * (self.scrnx as usize) + vx] = w.id;
                }
            }
        }
        Ok(())
    }

    pub fn refresh_windows(&mut self, base_x: i32, base_y: i32, w_pointer: *mut Window, from_height: usize, to_height: usize) -> Result<(), String> {
        let w = unsafe { *w_pointer };
        let (from_x, from_y, to_x, to_y) = unsafe { self.get_adjusted_position(base_x, base_y, w.xsize, w.ysize) };
        let linked_list_len: usize = self.linked_list.len();

        let height: usize = if to_height > linked_list_len { linked_list_len } else { to_height };
        for h in from_height..(height + 1) {
            let target_w_pointer = self.linked_list.get_data_from_position(h).ok_or("get_data_from_position is error in refresh_windows")?;
            let target_w = unsafe { *target_w_pointer };
            let win_from_x = if (from_x as isize) - (target_w.get_base_x() as isize) < 0 { 0 } else { (from_x as isize) - (target_w.get_base_x() as isize) };
            let win_from_y = if (from_y as isize) - (target_w.get_base_y() as isize) < 0 { 0 } else { (from_y as isize) - (target_w.get_base_y() as isize) };
            let win_to_x = unsafe {
                if (to_x as i16) - (target_w.get_base_x() as i16) > (target_w.xsize as i16) {
                    target_w.xsize as isize
                } else {
                    (to_x as isize - target_w.get_base_x() as isize)
                }
            };
            let win_to_y = unsafe {
                if (to_y as i16) - (target_w.get_base_y() as i16) > (target_w.ysize as i16) {
                    target_w.ysize as isize
                } else {
                    (to_y as isize - target_w.get_base_y() as isize)
                }
            };

            for y in win_from_y..win_to_y {
                let vy = (target_w.get_base_y() as usize + y as usize);
                for x in win_from_x..win_to_x {
                    let vx = (target_w.get_base_x() as usize + x as usize);
                    unsafe {
                        let win_id: usize = self.windows_map[vy * (self.scrnx as usize) + vx];
                        if target_w.id == win_id {
                            let mut address = ((self.vram) + (vy as u32) * (self.scrnx as u32) + (vx as u32)) as *mut u8;
                            *address = *target_w.buf.offset((y as isize) * (target_w.xsize as isize) + (x as isize));
                        }
                    }
                }
            }
        }
        return Ok(());
    }

    pub fn move_window(&mut self, w_pointer: *mut Window, mut value_x: i32, value_y: i32) -> Result<*mut Window, String> {
        let old_base_x: i32 = unsafe { (*w_pointer).base_x.clone() };
        let old_base_y: i32 = unsafe { (*w_pointer).base_y.clone() };

        // i32で保持されているデータだが、なぜかマイナスとして保持されておらず、16bit分だけ
        // コピーして、しっかりマナスとして判定させる
        // ToDo 今後根本となる原因を修正する必要はあるが(おそらくio_in8の返却がi32で返すところでおかしくなってる？)、
        //      一旦画面の大きさ的にx軸とy軸とで16bitの最大値を超えることはないため、こうする
        let x: i32 = 0x0000 + (value_x as i16) as i32;
        let y: i32 = 0x0000 + (value_y as i16) as i32;

        let mut mx: i32 = unsafe { (*w_pointer).base_x + x };
        mx = if mx < 0 {
            0
        } else if mx > unsafe { self.scrnx as i32 - 1 } {
            unsafe { self.scrnx as i32 - 1 }
        } else {
            unsafe { x + (*w_pointer).base_x }
        };

        let mut my: i32 = unsafe { (*w_pointer).base_y + y };
        my = if my < 0 {
            0
        } else if my > unsafe { self.scrny as i32 - 1 } {
            unsafe { self.scrny as i32 - 1 }
        } else {
            unsafe { y + (*w_pointer).base_y }
        };

        unsafe { (*w_pointer).set_base_x(mx) };
        unsafe { (*w_pointer).set_base_y(my) };
        let height = self.linked_list.get_position_from_data(w_pointer).ok_or("In move_window, window is not existing.".to_owned())?;

        self.refresh_map(old_base_x, old_base_y, w_pointer, 0, 15);
        self.refresh_map(mx, my, w_pointer, height, 30);

        let to_height = if height == 0 { 0 } else { height - 1 };
        self.refresh_windows(old_base_x, old_base_y, w_pointer, 0, to_height);
        self.refresh_windows(mx, my, w_pointer, height, height);
        return Ok(w_pointer);
    }

    fn get_adjusted_position(&self, base_x: i32, base_y: i32, xsize: u16, ysize: u16) -> (u16, u16, u16, u16) {
        let from_x = if base_x < 0 { 0 as u16 } else { base_x as u16 };
        let from_y = if base_y < 0 { 0 as u16 } else { base_y as u16};
        unsafe {
            let to_x: u16 = if from_x + xsize > self.scrnx { self.scrnx } else { from_x + xsize };
            let to_y: u16 = if from_y + ysize > self.scrny { self.scrny } else { from_y + ysize };
            return (from_x, from_y, to_x, to_y);
        }
    }
}


#[derive(Copy, Clone)]
pub struct Window {
    pub id: usize,
    pub base_x: i32,
    pub base_y: i32,
    pub xsize: u16,
    pub ysize: u16,
    pub buf: *mut u8,
}

impl Window {
    pub fn new(base_x: i32, base_y: i32, xsize: u16, ysize: u16, mut buf: *mut u8) -> Window {
        Window {
            id: get_uptime(),
            base_x,
            base_y,
            xsize,
            ysize,
            buf,
        }
    }

    fn set_base_x(&mut self, x: i32) {
        self.base_x = x;
    }

    pub fn get_base_x(&self) -> i32 {
        self.base_x
    }

    fn set_base_y(&mut self, y: i32) {
        self.base_y = y;
    }

    fn get_base_y(&self) -> i32 {
        self.base_y
    }

    fn equal(&self, other: &Window) -> bool {
        self.id == other.id
    }
}

impl core::cmp::PartialEq<Window> for Window {
    // ToDo ここの条件はしっかり考える
    #[inline]
    fn eq(&self, other: &Window) -> bool {
        self.id == other.id
    }

    #[inline]
    fn ne(&self, other: &Window) -> bool {
        self.id != other.id
    }
}

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "base_x: {:?}, base_y: {:?}, xsize: {:?}, ysize: {:?} buf: {:?}", self.base_x, self.base_y, self.xsize, self.ysize, self.buf as *const u8)
    }
}