use crate::lazy_static;
use crate::spin::mutex::Mutex;
use super::asmfunc::{io_out8, io_in8 };

const IO_PORT_COM1: i32 = 0x3F8;
const IO_PORT_COM2: i32 = 0x2F8;
const IO_PORT_COM3: i32 = 0x3E8;
const IO_PORT_COM4: i32 = 0x2E8;
const IO_PORT_COM5: i32 = 0x5F8;
const IO_PORT_COM6: i32 = 0x4F8;
const IO_PORT_COM7: i32 = 0x5E8;
const IO_PORT_COM8: i32 = 0x4E8;

pub struct SerialPort {
  io_port: i32,
  is_init: bool,
}

impl SerialPort {
  pub fn new(io_port: i32) -> Self {
    Self { io_port, is_init: false }
  }

  pub fn init(&mut self) {
    io_out8(self.io_port + 1, 0x00); // IER Disable all interrupts
    io_out8(self.io_port + 3, 0x80); // LCR Set DLAB on
    io_out8(self.io_port + 0, 0x03); // DLL - set baud late 38400 bps
    io_out8(self.io_port + 1, 0x00); // DLM - set baud rate 38400 bps
    io_out8(self.io_port + 3, 0x03); // LCR - disable DLAB, 8bit, no parity, 1 stop bit
    io_out8(self.io_port + 2, 0xC7); // FCR - enable FIFO, clear TX/RX queues, 14byte threshold
    io_out8(self.io_port + 4, 0x0B); // MCR - IRQs enabled, RTS/DSR set
    io_out8(self.io_port + 4, 0x1E); // MCR - set loopback mode, test the serial chip
    io_out8(self.io_port + 0, 0xAE); // RBR - test the serial chip (send 0xae)

    if io_in8(self.io_port + 0) != 0xAE {
      panic!("Serial port test failed");
    }

    // if serial isn't faulty, set normal mode
    io_out8(self.io_port + 4, 0x0F); // MCR - set normal operation mode
    self.is_init = true;
  }

  pub fn receive_data(&mut self) -> Result<i32, &'static str> {
    if !self.is_init {
      return Err("Serial port wasn't initialized");
    }
    let res = io_in8(self.io_port + 5) & 1;
    if res == 0 {
      return Err("Hasn't received data");
    }
    return Ok(io_in8(self.io_port));
  }

  pub fn send_data(&mut self, data: u8) -> Result<(), &'static str> {
    if !self.is_init {
      return Err("Serial port wasn't initialized");
    }

    while self.is_transmit_empty() == 0 {}
    io_out8(self.io_port, data);
    return Ok(());
  }
  pub fn is_transmit_empty(&self) -> i32 {
    io_in8(self.io_port + 5) & 0x20
  }
}

impl core::fmt::Write for SerialPort {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    for s in s.bytes() {
      self.send_data(s as u8).map_err(|_| core::fmt::Error)?;
    }
    Ok(())
  }
}

pub fn serial_print_char(c: char) {
  let mut serial = SerialPort::new(IO_PORT_COM1);
  serial.init();
  serial.send_data(c as u8).unwrap();
}

lazy_static! {
  static ref COM1: Mutex<SerialPort> = {
    let mut serial = SerialPort::new(IO_PORT_COM1);
    serial.init();
    Mutex::new(serial)
  };
}

pub fn _print(args: core::fmt::Arguments) {
  use core::fmt::Write;
  COM1.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::arch::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
  () => ($crate::serial_print!("\n"));
  ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
  ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}
