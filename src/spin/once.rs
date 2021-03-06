use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering, spin_loop_hint as cpu_relax};
use core::fmt;

pub struct Once<T> {
    state: AtomicUsize,
    data: UnsafeCell<Option<T>> // TODO remove option and use mem::uninitialized
}

impl<T: fmt::Debug> fmt::Debug for Once<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_exec() {
            Some(s) => write!(f, "Once {{ data: ")
                .and_then(|()| s.fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Once {{ <uninitialized> }}")
        }
    }
}

unsafe impl<T: Send + Sync> Sync for Once<T> {}
unsafe impl<T: Send> Send for Once<T> {}

const INCOMPLETE: usize = 0x0;
const RUNNING: usize = 0x1;
const COMPLETE: usize = 0x2;
const PANICKED: usize = 0x3;

use core::hint::unreachable_unchecked as unreachable;

impl<T> Once<T> {
    pub const INIT: Self = Once {
        state: AtomicUsize::new(INCOMPLETE),
        data: UnsafeCell::new(None),
    };

    pub const fn new() -> Once<T> {
        Self::INIT
    }

    fn force_get<'a>(&'a self) -> &'a T {
        match unsafe { &*self.data.get() }.as_ref() {
            None => unsafe { unreachable() },
            Some(p) => p,
        }
    }

    pub fn call_once<'a, F>(&'a self, builder: F) -> &'a T
        where F: FnOnce() -> T
    {

        let mut status = self.state.load(Ordering::SeqCst);

        if status == INCOMPLETE {
            status = self.state.compare_and_swap(
                INCOMPLETE,
                RUNNING,
                Ordering::SeqCst,
            );
            if status == INCOMPLETE {
                let mut finish = Finish { state: &self.state, panicked: true };
                unsafe { *self.data.get() = Some(builder()) };
                finish.panicked = false;

                status = COMPLETE;
                self.state.store(status, Ordering::SeqCst);
                return self.force_get();
            }
        }

        loop {
            match status {
                INCOMPLETE => unreachable!(),
                RUNNING => {
                    cpu_relax();
                    status = self.state.load(Ordering::SeqCst)
                },
                PANICKED => panic!("Once has panicked."),
                COMPLETE => return self.force_get(),
                _ => unsafe { unreachable() },
            }
        }
    }

    pub fn try_exec<'a>(&'a self) -> Option<&'a T> {
        match self.state.load(Ordering::SeqCst) {
            COMPLETE => Some(self.force_get()),
            _ => None,
        }
    }

    pub fn wait<'a>(&'a self) -> Option<&'a T> {
        loop {
            match self.state.load(Ordering::SeqCst) {
                INCOMPLETE => return None,
                RUNNING => cpu_relax(),
                COMPLETE => return Some(self.force_get()),
                PANICKED => panic!("Once has panicked."),
                _ => unsafe { unreachable() },
            }
        }
    }
}

struct Finish<'a> {
    state: &'a AtomicUsize,
    panicked: bool,
}

impl<'a> Drop for Finish<'a> {
    fn drop(&mut self) {
        if self.panicked {
            self.state.store(PANICKED, Ordering::SeqCst);
        }
    }
}