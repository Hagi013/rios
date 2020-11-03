use core::ptr;
use alloc::string::{String, ToString};
use core::fmt;
use core::borrow::{Borrow, BorrowMut};
use core::cell::RefCell;
use alloc::rc::Rc;
use super::asmfunc;

use super::super::queue::SimpleQueue;
use crate::util::linked_list::LinkedList;
use crate::spin::mutex::Mutex;

#[macro_use]
use crate::lazy_static;

lazy_static! {
    static ref TIMER_CTRL: Mutex<TimerCtrl> = {
        Mutex::new(TimerCtrl::init())
    };
    static ref COUNTER: Mutex<usize> = Mutex::new(0);
}

use super::graphic::Graphic;
use super::graphic::Printer;
use core::fmt::Write;

use super::pic::PIC1_IMR;
use super::pic::PIC0_OCW2;
use super::pic::PIC1_OCW2;

const PIT_CTRL: i32 = 0x0043;
const PIT_CNT0: i32 = 0x0040;
const PIT_CNT1: i32 = 0x0041;
const PIT_CNT2: i32 = 0x0042;

#[derive(Copy, Clone)]
enum TimerFlag {
    ALLOC, /* 確保した状態 */
    USING, /* タイマ作動中 */
}

impl PartialEq for TimerFlag {
    fn eq(&self, other: &TimerFlag) -> bool {
        self == other
    }
}

impl fmt::Debug for TimerFlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimerFlag::ALLOC => write!(f, "TimerFlag: ALLOC"),
            TimerFlag::USING => write!(f, "TimerFlag: USING"),
        }
    }
}

#[derive(Copy, Debug)]
struct Timer {
    timeout: u32,
    flags: TimerFlag,
    queue: SimpleQueue<i32>,
    data: i32,
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

impl Clone for Timer {
    fn clone(&self) -> Timer {
        Timer {
            timeout: self.timeout,
            flags: self.flags,
            queue: self.queue.clone(),
            data: self.data,
        }
    }
}

impl PartialEq for Timer {
    fn eq(&self, other: &Timer) -> bool {
        self.timeout == self.timeout
        && self.flags == self.flags
        && self.data == self.data
    }
}

impl Timer {
    fn new(timeout: u32, flags: TimerFlag, data: i32) -> Timer {
        Timer {
            timeout,
            flags,
            queue: SimpleQueue::new(),
            data,
        }
    }
}

struct TimerCtrl {
    count: u32,
    next_timeout: u32,
    linked_list: LinkedList<Timer>,
}

unsafe impl Sync for TimerCtrl {}
unsafe impl Send for TimerCtrl {}

impl TimerCtrl {
    fn new(next_timeout: u32) -> TimerCtrl {
        TimerCtrl {
            count: 0,
            next_timeout,
            linked_list: LinkedList::new(),
        }
    }

    pub fn init() -> TimerCtrl {
        TimerCtrl::new(0xffffffff)
    }

    pub fn init_add(&mut self, timer: *mut Timer) {
        self.linked_list.add(timer);
    }

    pub fn add(&mut self, timer: *mut Timer) {
        self.linked_list.push_front(timer);
    }

    pub fn get_count(&self) -> u32 { self.count }

}

pub fn timer_init() {
    asmfunc::io_out8(PIT_CTRL, 0x34);
    asmfunc::io_out8(PIT_CNT0, 0x9c); // 10msごとに割り込み(0x2e9c = 11932 = 100Hz)
    asmfunc::io_out8(PIT_CNT0, 0x2e);

    let guard_timer: Timer = Timer::new(0xffffffff, TimerFlag::USING, 0);
//    *TIMER_CTRL.lock().unwrap().add(guard_timer);
//    TIMER_CTRL.lock().add(guard_timer);
}

#[no_mangle]
pub extern "C" fn inthandler20(esp: *const usize) {
    // Graphic::putfont_asc(210, 330, 10, "Timer Started!!");
    asmfunc::io_out8(PIC0_OCW2, 0x60);
    // ひとまずuptimeを数える
    *COUNTER.lock() += 1;
}

pub fn get_uptime() -> usize {
    return *COUNTER.lock();
}
