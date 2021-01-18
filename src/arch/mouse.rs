use core::ptr;
use alloc::string::{String, ToString};

use super::asmfunc;

use super::pic::PIC1_IMR;
use super::pic::PIC0_OCW2;
use super::pic::PIC1_OCW2;
use super::pic::PORT_KEYDAT;
use super::pic::KEYCMD_SENDTO_MOUSE;
use super::pic::MOUSECMD_ENABLE;
use super::pic::wait_kbc_sendready;
use super::super::queue::SimpleQueue;
use super::super::spin::mutex::Mutex;
use super::super::lazy_static;

use super::graphic::Graphic;
use super::graphic::Printer;
use core::fmt::Write;
use crate::arch::pic::PORT_KEYCMD;
use core::borrow::Borrow;

static mut MOUSE_QUEUE: Option<SimpleQueue<i32>> = None;
static mut MOUSE_BUF: Option<MouseBuf> = None;
static mut INIT_FLAG: Option<bool> = Some(false);

#[derive(Copy, Clone)]
struct MouseBuf {
    phase: usize,
    buf: [i32; 3],
    x: i32,
    y: i32,
    btn: i32,
}

impl MouseBuf {
    pub fn new() -> Self {
        MouseBuf {
            phase: 0,
            buf: [0x00; 3],
            x: 0x00,
            y: 0x00,
            btn: 0x00,
        }
    }

    pub fn get_from_mouse_dev(&mut self, data: i32) -> Result<i32, ()> {
        /* マウスの0xfaを待っている段階 */
        if self.phase == 0 && data == 0xfa {
            self.phase = 1;
            return Ok(0);
        }

        if self.phase == 1 { /* マウスの1バイト目を待っている段階 */
            /*  正しい1バイト目だった */
            if (data & 0xc8) == 0x08 {
                self.buf[0] = data;
                self.phase = 2;
            }
            return Ok(0);
        }
        if self.phase == 2 { /* マウスの2バイト目を待っている段階 */
            self.buf[1] = data;
            self.phase = 3;
            return Ok(0);
        }
        if self.phase == 3 { /* マウスの3バイト目を待っている段階 */
            self.buf[2] = data;
            self.phase = 1;
            self.btn = self.buf[0] & 0x07;
            self.x = self.buf[1];
            self.y = self.buf[2];
            if (self.buf[0] & 0x10) != 0x00 {
                self.x |= 0x7fffff00;
            }
            if (self.buf[0] & 0x20) != 0x00 {
                self.y |= 0x7fffff00;
            }
            /* マウスではy方向の符号が画面と反対 */
            self.y = -self.y;
            return Ok(1);
        }
        return Err(());
    }

    pub fn get_status(&self) -> (i32, i32, i32) {
        (self.btn, self.x, self.y)
    }
}

pub fn allow_mouse_int() {
    asmfunc::io_out8(PIC1_IMR, 0xef);
    wait_kbc_sendready();
    asmfunc::io_out8(PORT_KEYCMD, KEYCMD_SENDTO_MOUSE);
    wait_kbc_sendready();
    asmfunc::io_out8(PORT_KEYDAT, MOUSECMD_ENABLE);
    let queue: SimpleQueue<i32> = SimpleQueue::new();
    let mouse_buf: MouseBuf = MouseBuf::new();
    unsafe {
        MOUSE_QUEUE = Some(queue);
        MOUSE_BUF = Some(mouse_buf);
    }
}

#[no_mangle]
pub extern "C" fn inthandler2c(esp: *const usize) {
    asmfunc::io_out8(PIC1_OCW2, 0x64);
    asmfunc::io_out8(PIC0_OCW2, 0x62);
    unsafe {
        // 初回に謎の250という値が飛んでくるので無視する
        if !INIT_FLAG.unwrap() {
            INIT_FLAG = Some(true);
            return;
        } else {
            let data: i32 = asmfunc::io_in8(PORT_KEYDAT);
            match ptr::read_volatile(&MOUSE_QUEUE) {
                Some(mut queue) => {
                    queue.enqueue(data);
                    MOUSE_QUEUE = Some(queue);
                },
                None => {
                    let mut queue: SimpleQueue<i32> = SimpleQueue::new();
                    queue.enqueue(data);
                    MOUSE_QUEUE = Some(queue);
                }
            }
        }
    }
}

pub fn is_existing() -> bool {
    unsafe {
        match ptr::read_volatile(&MOUSE_QUEUE) {
            Some(queue) => queue.is_existing(),
            None => false,
        }
    }
}

pub fn get_data() -> Result<Option<(i32, i32, i32)>, String> {
    unsafe {
        if let Some(mut queue) = ptr::read(&MOUSE_QUEUE) {
            let data = queue.dequeue().ok_or("dequeue is error at mouse.".to_string())?;
            MOUSE_QUEUE = Some(queue);
            if let Some(mut buf) = ptr::read(&MOUSE_BUF) {
                let code = buf.get_from_mouse_dev(data).or(Err("get_from_mouse_dev is Error.".to_string()))?;
                MOUSE_BUF = Some(buf);
                if code == 0 {
                    return Ok(None)
                } else {
                    let mouse_status_tuple = buf.get_status();
                    Ok(Some(mouse_status_tuple))
                }
            } else {
                MOUSE_BUF = Some(MouseBuf::new());
                Err("MOUSE_BUF is initialized.".to_string())
            }
        } else {
            MOUSE_QUEUE = Some(SimpleQueue::new());
            Err("MOUSE_QUEUE is initialized.".to_string())
        }
    }
}
