use core::fmt::Write;
use core::mem::{size_of, transmute};
// use core::ptr::{read_volatile, write_volatile};
use alloc::vec::Vec;
use alloc::borrow::ToOwned;

use super::super::net::e1000::{get_mac_addr};
use crate::asmfunc::{io_in32, io_out32};
use crate::arch::graphic::{Graphic, Printer, print_str};
use crate::memory::dma::DmaBox;

#[macro_use]
use crate::memory::volatile::{write_mem, read_mem};

#[macro_use]
use crate::lazy_static;
use crate::spin::mutex::{Mutex, MutexGuard};


const CONFIG_ADDR: i32 = 0x0cf8;
const CONFIG_DATA: i32 = 0x0cfc;
const PCI_CONF_DID_VID: u8 = 0x00;
const PCI_CONF_BAR: u8 = 0x10;

const NIC_BUS_NUM: u8 = 0x00;
// const NIC_DEV_NUM: u8 = 0x04;
const NIC_DEV_NUM: u8 = 0x03;
const NIC_FN_NUM: u8 = 0x0;
const NIC_REG_IMS: u16 = 0x00d0;
const NIC_REG_IMC: u16 = 0x00d8;
const NIC_REG_RCTL: u16 = 0x0100;
const NIC_REG_RDBAL: u16 = 0x2800;
const NIC_REG_RDBAH: u16 = 0x2804;
const NIC_REG_RDLEN: u16 = 0x2808;
const NIC_REG_RDH: u16 = 0x2810;
const NIC_REG_RDT: u16 = 0x2818;
const NIC_REG_TCTL: u16 = 0x0400;
const NIC_REG_TDBAL: u16 = 0x3800;
const NIC_REG_TDBAH: u16 = 0x3804;
const NIC_REG_TDLEN: u16 = 0x3808;
const NIC_REG_TDH: u16 = 0x3810;
const NIC_REG_TDT: u16 = 0x3818;

const PCI_CONF_STATUS_COMMAND: u8 = 0x04;

const PCI_COM_IO_EN: u32 = 0x01 << 0;
const PCI_COM_MEM_EN: u32 = 0x01 << 1;
const PCI_COM_BUS_MASTER_EN: u32 = 0x01 << 2;
const PCI_COM_SPECIAL_CYCLE: u32 = 0x01 << 3;
const PCI_COM_MEMV_INV_EN: u32 = 0x01 << 4;
const PCI_COM_VGA_PAL_SNP: u32 = 0x01 << 5;
const PCI_COM_PARITY_ERR_RES: u32 = 0x01 << 6;
const PCI_COM_SERR_EN: u32 = 0x01 << 8;
const PCI_COM_FAST_BACK2BACK_EN: u32 = 0x01 << 9;
const PCI_COM_INTR_DIS: u32 = 0x01 << 10;

const PCI_STAT_INTR: u32 = 0x01 << 3;
const PCI_STAT_MULT_FUNC: u32 = 0x01 << 4;
const PCI_STAT_66MHZ: u32 = 0x01 << 5;
const PCI_STAT_FAST_BACK2BACK: u32 = 0x01 << 7;
const PCI_STAT_DATA_PARITY_ERR: u32 = 0x01 << 8;
const PCI_STAT_DEVSEL_MASK: u32 = 0x03 << 9;
const PCI_STAT_DEVSEL_FAST: u32 = 0b00 << 9;
const PCI_STAT_DEVSEL_MID: u32 = 0b01 << 9;
const PCI_STAT_DEVSEL_LOW: u32 = 0b10 << 9;
const PCI_STAT_SND_TARGET_ABORT: u32 = 0x01 << 11;
const PCI_STAT_RCV_TARGET_ABORT: u32 = 0x01 << 12;
const PCI_STAT_RCV_MASTER_ABORT: u32 = 0x01 << 13;
const PCI_STAT_SYS_ERR: u32 = 0x01 << 14;
const PCI_STAT_PARITY_ERR: u32 = 0x01 << 15;

const PCI_BAR_MASK_IO: u32 = 0x00000001;
const PCI_BAR_MASK_MEM_TYPE: u32 = 0x00000006;
const PCI_BAR_MEM_TYPE_32BIT: u32 = 0x00000000;
const PCI_BAR_MEM_TYPE_1M: u32 = 0x00000002;
const PCI_BAR_MEM_TYPE_64BIT: u32 = 0x00000004;
const PCI_BAR_MASK_MEM_PREFETCHABLE: u32 = 0x00000008;
const PCI_BAR_MASK_MEM_ADDR: u32 = 0xfffffff0;
const PCI_BAR_MASK_IO_ADDR: u32 = 0xfffffffc;

const NIC_RCTL_EN: u32 = 1 << 1;
const NIC_RCTL_SBP: u32 = 1 << 2;
const NIC_RCTL_UPE: u32 = 1 << 3;
const NIC_RCTL_MPE: u32 = 1 << 4;
const NIC_RCTL_LPE: u32 = 1 << 5;
const NIC_RCTL_BAM: u32 = 1 << 15;
const NIC_RCTL_BSIZE_1024B: u32 = 0b01 << 16;
const PACKET_RBSIZE_BIT: u32 = 0b01 << 16;

const NIC_TCTL_EN: u32 = 1 << 1;
const NIC_TCTL_PSP: u32 = 1 << 3;
const NIC_TCTL_CT_SHIFT: u32 = 4;
const NIC_TCTL_COLD_SHIFT: u32 = 12;
const NIC_TCTL_SWXOFF: u32 = 1 << 22;
const NIC_TCTL_RTLC: u32 = 1 << 24;
const NIC_TCTL_NRTU: u32 = 1 << 25;


const NIC_RDESC_STAT_DD: u8 = 1 << 0;
const NIC_RDESC_STAT_EOP: u8 = 1 << 1;
const NIC_RDESC_STAT_IXSM: u8 = 1 << 2;
const NIC_RDESC_STAT_VP: u8 = 1 << 3;
const NIC_RDESC_STAT_TCPCS: u8 = 1 << 5;
const NIC_RDESC_STAT_IPCS: u8 = 1 << 6;
const NIC_RDESC_STAT_PIF: u8 = 1 << 7;

const NIC_TDESC_CMD_EOP: u8 = 1 << 0;
const NIC_TDESC_CMD_IFCS: u8 = 1 << 1;
const NIC_TDESC_CMD_IC: u8 = 1 << 2;
const NIC_TDESC_CMD_RS: u8 = 1 << 3;
const NIC_TDESC_CMD_RPS: u8 = 1 << 4;
const NIC_TDESC_CMD_DEXT: u8 = 1 << 5;
const NIC_TDESC_CMD_VLE: u8 = 1 << 6;
const NIC_TDESC_CMD_IDE: u8 = 1 << 7;


struct PciConfiguration(u32);

impl PciConfiguration {
    pub fn new(enable: bool, bus_num: u8, device_num: u8, fn_num: u8, reg_addr: u8) -> Self {
        let enable_bit: u32 = if enable { 0x80000000 } else { 0x00000000 };
        PciConfiguration(
            0x00000000 | enable_bit | ((bus_num as u32) << 16) | ((device_num as u32) << 11) | ((fn_num as u32) << 8) | (reg_addr as u32)
        )
    }

    fn enabled(&self) -> bool {
        self.0 & 0x80000000 > 0
    }

    fn set_enable(&mut self) {
        self.0 = self.0 & 0xffffffff;
    }

    fn set_disable(&mut self) {
        self.0 = self.0 & 0x7fffffff;
    }

    fn set_bus_num(&mut self, bus_num: u8) {
        self.0 = self.0 & 0xff00ffff;
        self.0 = self.0 | ((bus_num as u32) << 16);
    }

    fn get_bus_num(&self) -> u8 {
        ((self.0 & 0x00ff0000) >> 16) as u8
    }

    fn set_device_num(&mut self, device_num: u8) {
        self.0 = self.0 & 0xffff07ff;
        self.0 = self.0 | ((device_num as u32) << 11);
    }

    fn get_device_num(&self) -> u8 {
        (((self.0 & 0x0000f800) >> 11) & 0x1f) as u8
    }

    fn set_fn_num(&mut self, fn_num: u8) {
        self.0 = self.0 & 0xfffff8ff;
        self.0 = self.0 | ((fn_num as u32) << 8);
    }

    fn get_fn_num(&self) -> u8 {
        (((self.0 & 0x00000700) >> 8) & 0x08) as u8
    }

    fn set_reg_addr(&mut self, reg_addr: u8) {
        self.0 = self.0 & 0xffffff00;
        self.0 = self.0 | (reg_addr as u32);
    }

    fn get_reg_addr(&self) -> u8 {
        (self.0 & 0x000000ff) as u8
    }
}

struct BaseAddressRegister(u32);
impl BaseAddressRegister {
    fn new(addr: i32) -> Self {
        BaseAddressRegister(addr as u32)
    }

    fn prefetchable(&self) -> bool {
        self.0 & 0x08 > 0
    }

    fn memory_32bit_type(&self) -> bool {
        if self.io_address() { return false; }
        !self.memory_1m_type() && !self.memory_64bit_type()
    }

    fn memory_1m_type(&self) -> bool {
        if self.io_address() { return false; }
        self.0 & 0x02 > 0
    }

    fn memory_64bit_type(&self) -> bool {
        if self.io_address() { return false; }
        self.0 & 0x04 > 0
    }

    fn io_address(&self) -> bool {
        self.0 & 0x01 > 0
    }

    fn base_addr(&self) -> u32 {
        if self.io_address() {
            return self.0 & 0xfffffffc;
        }
        self.0 & 0xfffffff8
    }
}

pub fn get_pci_conf_reg(nic_bus_num: u8, nic_dev_num: u8, nic_fn_num: u8, reg: u8) -> i32 {
    io_out32(CONFIG_ADDR, PciConfiguration::new(true, nic_bus_num, nic_dev_num, nic_fn_num, reg).0);
    io_in32(CONFIG_DATA)
}

pub fn set_pci_conf_reg(nic_bus_num: u8, nic_dev_num: u8, nic_fn_num: u8, reg: u8, val: u32) {
    io_out32(CONFIG_ADDR, PciConfiguration::new(true, nic_bus_num, nic_dev_num, nic_fn_num, reg).0);
    io_out32(CONFIG_DATA, val);
}

pub fn get_nic_reg_base() -> u32 {
    BaseAddressRegister::new(get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_BAR)).base_addr()
}

pub fn get_nic_reg(reg: u16) -> u32 {
    let mut base_addr = get_nic_reg_base();
    // let mut pointer = unsafe { *(&mut base_addr as *mut u32).offset(0 as isize) as *mut u32 as u32 };
    // let mut printer = Printer::new(900, 305, 0);
    // write!(printer, "{:?}", unsafe { *(*&mut pointer as *mut u32) as *mut u32 }).unwrap(); // 0xffef3 => 0xfebc0000
    unsafe { *(*&mut (base_addr + reg as u32) as *mut u32) }
}

pub fn set_nic_reg(reg: u16, val: u32) {
    let mut base_addr = get_nic_reg_base();
    unsafe { *(*&mut (base_addr + reg as u32) as *mut u32) = val };
}

pub fn dump_vid_did() {
    let conf_data: i32 = get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_DID_VID);
    let vendor_id: u16 = (conf_data & 0x0000ffff) as u16;
    let device_id: u16 = (conf_data >> 16) as u16;
    let mut printer = Printer::new(10, 200, 0);
    write!(printer, "{:x}", vendor_id).unwrap();

    let mut printer = Printer::new(10, 215, 0);
    write!(printer, "{:x}", device_id).unwrap();
}

pub fn dump_command_status() {
    let conf_data: i32 = get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_STATUS_COMMAND);

    let command: u32 = (conf_data & 0x0000ffff) as u32;
    let status: u32 = (conf_data >> 16) as u32;

    let mut height: u32 = 215;
    if command & PCI_COM_IO_EN > 0 {
        print_str(400, height, "IO_EN", 0);
        height += 15;
    }
    if command & PCI_COM_MEM_EN > 0 {
        print_str(400, height, "MEM_EN", 0);
        height += 15;
    }
    if command & PCI_COM_BUS_MASTER_EN > 0 {
        print_str(400, height, "BUS_MASTER_EN", 0);
        height += 15;
    }
    if command & PCI_COM_SPECIAL_CYCLE > 0 {
        print_str(400, height, "SPECIAL_CYCLE", 0);
        height += 15;
    }
    if command & PCI_COM_MEMV_INV_EN > 0 {
        print_str(400, height, "MEMW_INV_EN", 0);
        height += 15;
    }
    if command & PCI_COM_VGA_PAL_SNP > 0 {
        print_str(400, height, "VGA_PAL_SNP", 0);
        height += 15;
    }
    if command & PCI_COM_PARITY_ERR_RES > 0 {
        print_str(400, height, "PARITY_ERR_RES", 0);
        height += 15;
    }
    if command & PCI_COM_SERR_EN > 0 {
        print_str(400, height, "SERR_EN", 0);
        height += 15;
    }
    if command & PCI_COM_FAST_BACK2BACK_EN > 0 {
        print_str(400, height, "FAST_BACK2BACK_EN", 0);
        height += 15;
    }
    if command & PCI_COM_INTR_DIS > 0 {
        print_str(400, height, "INTR_DIS", 0);
        height += 15;
    }

    let mut height: u32 = 215;
    if status & PCI_STAT_INTR > 0 {
        print_str(500, height, "INTR", 0);
        height += 15;
    }
    if status & PCI_STAT_MULT_FUNC > 0 {
        print_str(500, height, "MULT_FUNC", 0);
        height += 15;
    }
    if status & PCI_STAT_66MHZ > 0 {
        print_str(500, height, "66MHZ", 0);
        height += 15;
    }
    if status & PCI_STAT_FAST_BACK2BACK > 0 {
        print_str(500, height, "FAST_BACK2BACK", 0);
        height += 15;
    }
    if status & PCI_STAT_DATA_PARITY_ERR > 0 {
        print_str(500, height, "DATA_PARITY_ERR", 0);
        height += 15;
    }
    if (status as u32 & PCI_STAT_DEVSEL_MASK) == PCI_STAT_DEVSEL_FAST {
        print_str(500, height, "DEVSEL_FAST", 0);
        height += 15;
    }
    if (status as u32 & PCI_STAT_DEVSEL_MASK) == PCI_STAT_DEVSEL_MID {
        print_str(500, height, "DEVSEL_MID", 0);
        height += 15;
    }
    if (status as u32 & PCI_STAT_DEVSEL_MASK) == PCI_STAT_DEVSEL_LOW {
        print_str(500, height, "DEVSEL_LOW", 0);
        height += 15;
    }

    if status & PCI_STAT_SND_TARGET_ABORT > 0 {
        print_str(500, height, "SND_TARGET_ABORT", 0);
        height += 15;
    }
    if status & PCI_STAT_RCV_TARGET_ABORT > 0 {
        print_str(500, height, "RCV_TARGET_ABORT", 0);
        height += 15;
    }
    if status & PCI_STAT_RCV_MASTER_ABORT > 0 {
        print_str(500, height, "RCV_MASTER_ABORT", 0);
        height += 15;
    }
    if status & PCI_STAT_SYS_ERR > 0 {
        print_str(500, height, "SYS_ERR", 0);
        height += 15;
    }
    if status & PCI_STAT_PARITY_ERR > 0 {
        print_str(500, height, "PARITY_ERR", 0);
        height += 15;
    }
}

pub fn set_pci_intr_disable() {
    let mut conf_data = get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_STATUS_COMMAND) as u32;
    conf_data = conf_data & 0x0000ffff;
    conf_data = conf_data | PCI_COM_INTR_DIS;
    set_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_STATUS_COMMAND, conf_data);
    // dump_command_status();
}

pub fn set_bus_master_en() {
    let mut conf_data = get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_STATUS_COMMAND) as u32;
    conf_data = conf_data & 0x0000ffff;
    conf_data = conf_data | PCI_COM_BUS_MASTER_EN;
    set_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_STATUS_COMMAND, conf_data);
    // dump_command_status();
}

pub fn dump_bar() {
    let bar: BaseAddressRegister = BaseAddressRegister::new(get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_BAR));
    let mut height = 50;
    if bar.io_address() {
        print_str(800, height, "IO BASE", 0);
        height += 15;
        let mut printer = Printer::new(800, height, 0);
        write!(printer, "{:x}", bar.base_addr()).unwrap();
    } else {
        if bar.memory_32bit_type() {
            print_str(800, height, "MEM BASE 32BIT", 0);
            height += 15;
            let mut printer = Printer::new(800, height, 0);
            write!(printer, "{:x}", bar.base_addr()).unwrap();
        }
        if bar.memory_1m_type() {
            print_str(800, height, "MEM BASE 1M", 0);
            height += 15;
            let mut printer = Printer::new(800, height, 0);
            write!(printer, "{:x}", bar.base_addr()).unwrap();
        }
        if bar.memory_64bit_type() {
            let bar_upper = get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_BAR + 4) as u32;
            print_str(800, height, "MEM BASE 64BIT UPPER", 0);
            height += 15;
            let mut printer = Printer::new(800, height, 0);
            write!(printer, "{:x}", bar_upper).unwrap();
            print_str(800, height, "MEM BASE 64BIT", 0);
            height += 15;
            let mut printer = Printer::new(800, height, 0);
            write!(printer, "{:x}", bar.base_addr()).unwrap();
        }
        if bar.prefetchable() {
            height += 15;
            print_str(800, height, "PREFETCHABLE", 0);
        } else {
            height += 15;
            print_str(800, height, "NON PREFETCHABLE", 0);
        }
    }
}

pub fn dump_nic_ims() {
    let ims: u32 = get_nic_reg(NIC_REG_IMS);
    let mut printer = Printer::new(900, 200, 0);
    write!(printer, "{:x}", ims).unwrap();
}

pub fn test_nic_set() {
    dump_nic_ims();
    set_nic_reg(NIC_REG_IMS, 0x0000beef);
    let ims: u32 = get_nic_reg(NIC_REG_IMS);
    let mut printer = Printer::new(900, 215, 0);
    write!(printer, "{:x}", ims).unwrap();
    set_nic_reg(NIC_REG_IMC, 0xffffffff);
    let ims: u32 = get_nic_reg(NIC_REG_IMS);
    let mut printer = Printer::new(900, 230, 0);
    write!(printer, "{:x}", ims).unwrap();
}

pub fn disable_nic_interrupt() {
    let mut conf_data: u32 = get_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_STATUS_COMMAND) as u32;
    conf_data = conf_data & 0x0000ffff;
    conf_data = conf_data | PCI_COM_INTR_DIS;
    set_pci_conf_reg(NIC_BUS_NUM, NIC_DEV_NUM, NIC_FN_NUM, PCI_CONF_STATUS_COMMAND, conf_data);

    set_nic_reg(NIC_REG_IMC, 0xffffffff);
}

#[derive(Copy, Clone)]
struct BufferAddr {
    desc_base_low: u32,
    desc_base_high: u32,
}

impl BufferAddr {
    fn new() -> Self {
        BufferAddr {
            desc_base_low: 0,
            desc_base_high: 0,
        }
    }
    const fn new_const() -> Self {
        BufferAddr {
            desc_base_low: 0,
            desc_base_high: 0,
        }
    }
    fn low(&self) -> u32 { self.desc_base_low }
    fn high(&self) -> u32 { self.desc_base_high }
}

#[derive(Copy, Clone)]
#[repr(align(16), C)]
struct RxDesc {
    // イーサネットフレーム用バッファ(RX_BUFFER内のアドレス)のアドレスを指定
    recv_buf_addr: BufferAddr,
    // イーサネットフレーム用バッファの長さ
    length: u16,
    packet_checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

impl RxDesc {
    fn new() -> Self {
        RxDesc {
            recv_buf_addr: BufferAddr::new(),
            length: 0,
            packet_checksum: 0,
            status: 0,
            errors: 0,
            special: 0,
        }
    }
    const fn new_const() -> Self {
        RxDesc {
            recv_buf_addr: BufferAddr::new_const(),
            length: 0,
            packet_checksum: 0,
            status: 0,
            errors: 0,
            special: 0,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(align(16), C)]
struct TxDesc {
    tx_buf_address: BufferAddr,
    length: u16,
    cso: u8,
    cmd: u8,
    sta_rsv: u8,
    css: u8,
    special: u16
}

impl TxDesc {
    fn new() -> Self {
        TxDesc {
            tx_buf_address: BufferAddr::new(),
            length: 0,
            cso: 0,
            cmd: NIC_TDESC_CMD_RS | NIC_TDESC_CMD_EOP,
            sta_rsv: 0,
            css: 0,
            special: 0
        }
    }
    const fn new_const() -> Self {
        TxDesc {
            tx_buf_address: BufferAddr::new_const(),
            length: 0,
            cso: 0,
            cmd: NIC_TDESC_CMD_RS | NIC_TDESC_CMD_EOP,
            sta_rsv: 0,
            css: 0,
            special: 0
        }
    }
}

const RXDESC_NUM: usize = 80;
const TXDESC_NUM: usize = 8; // 128バイトアラインメントがある
const PACKET_BUFFER_SIZE: u16 = 1024;

// イーサネットフレームを格納するBuffer
static mut RX_BUFFER: [[u8; PACKET_BUFFER_SIZE as usize]; RXDESC_NUM as usize] = [[0 as u8; PACKET_BUFFER_SIZE as usize]; RXDESC_NUM as usize];
// static mut TX_BUFFER: [u8; PACKET_BUFFER_SIZE as usize] = [0x0; PACKET_BUFFER_SIZE as usize];
const fn rx_desc_data_size() -> usize { size_of::<RxDesc>() * RXDESC_NUM }
const fn tx_desc_data_size() -> usize { size_of::<TxDesc>() * TXDESC_NUM }

// RxDescの配列を格納するリングバッファ
static mut RX_DESC_DATA: [RxDesc; RXDESC_NUM] = [RxDesc::new_const(); RXDESC_NUM];
static mut TX_DESC_DATA: [TxDesc; TXDESC_NUM] = [TxDesc::new_const(); TXDESC_NUM];

// RX_DESC_DATAのベースアドレス
// static mut RX_DESC_DATA_BASE: Option<*mut RxDesc> = None;
// RX_DESC_DATAのリングバッファ上において、現在のデータを習得する先としてのindex
// static mut CURRENT_RX_IDX: usize = 0x1;
lazy_static! {
    static ref CURRENT_RX_IDX: Mutex<usize> = Mutex::new(0x0);
    static ref CURRENT_TX_IDX: Mutex<usize> = Mutex::new(0x0);
}


pub fn rx_init() {
    let mut printer = Printer::new(700, 55, 0);
    write!(printer, "{:?}", unsafe { &mut RX_DESC_DATA as *mut [RxDesc; 80] }).unwrap();

    let mut rx_desc_data_for_initialize: [RxDesc; RXDESC_NUM] = [RxDesc::new(); RXDESC_NUM];
    // recv_buf_addrのアドレスを指定する作業
    for (idx, cur_rxdesc) in unsafe { RX_DESC_DATA.iter_mut().enumerate() } {
        (*cur_rxdesc).recv_buf_addr.desc_base_low = unsafe { transmute::<*mut &[u8], u32>(&mut RX_BUFFER[idx] as *const _ as *mut &[u8]) };
        rx_desc_data_for_initialize[idx] = *cur_rxdesc;
    }

    unsafe { RX_DESC_DATA = rx_desc_data_for_initialize; }

    let mut printer = Printer::new(700, 70, 0);
    write!(printer, "{:?}", unsafe { &mut RX_DESC_DATA as *mut [RxDesc; 80] }).unwrap();
    let mut printer = Printer::new(700, 100, 0);
    write!(printer, "{:?}", unsafe { &mut RX_DESC_DATA[0] as *mut RxDesc }).unwrap();
    let mut printer = Printer::new(700, 115, 0);
    write!(printer, "{:?}", unsafe { &mut RX_BUFFER as *const _ }).unwrap();
    let mut printer = Printer::new(700, 130, 0);
    write!(printer, "{:x}", unsafe { RX_DESC_DATA[0].recv_buf_addr.low() }).unwrap();
    let mut printer = Printer::new(700, 145, 0);
    write!(printer, "{:?}", unsafe { RX_DESC_DATA[0].recv_buf_addr.high() }).unwrap();

    /* rxdescの先頭アドレスとサイズをNICレジスタへ設定 */
    // set_nic_reg(NIC_REG_RDBAH, unsafe { RX_DESC_DATA.unwrap()[0].recv_buf_addr.desc_base_high });
    // set_nic_reg(NIC_REG_RDBAL, unsafe { RX_DESC_DATA.unwrap()[0].recv_buf_addr.desc_base_low });
    set_nic_reg(NIC_REG_RDBAH, 0x00);
    set_nic_reg(NIC_REG_RDBAL, unsafe { &mut RX_DESC_DATA as *mut [RxDesc; RXDESC_NUM] as u32 });
    set_nic_reg(NIC_REG_RDLEN, rx_desc_data_size() as u32 ); // 1280 = 0x500

    set_nic_reg(NIC_REG_RDH, unsafe { *CURRENT_RX_IDX.lock() as u32 }); // 0
    set_nic_reg(NIC_REG_RDT, (RXDESC_NUM - 1) as u32); // 79 = 0x4f

    /* NICの受信動作設定 */
    // 0b01 << 16 | 1 << 15 | 1 << 4 | 1 << 3 | 1 << 2 | 1 << 1
    // 0b1_1000_0000_0001_1110 = 0x1801e
    set_nic_reg(NIC_REG_RCTL, (PACKET_RBSIZE_BIT | NIC_RCTL_BAM | NIC_RCTL_MPE | NIC_RCTL_UPE | NIC_RCTL_SBP | NIC_RCTL_EN) as u32);
    dump_nic_reg_for_net();
}

pub fn dump_nic_reg_for_net() {
    let rdbah = get_nic_reg(NIC_REG_RDBAH);
    let mut printer = Printer::new(700, 175, 0);
    write!(printer, "{:?}", rdbah).unwrap();

    let rdbal = get_nic_reg(NIC_REG_RDBAL);
    let mut printer = Printer::new(700, 190, 0);
    write!(printer, "{:x}", rdbal).unwrap();

    let rdlen = get_nic_reg(NIC_REG_RDLEN);
    let mut printer = Printer::new(700, 205, 0);
    write!(printer, "{:x}", rdlen).unwrap();

    let rdh = get_nic_reg(NIC_REG_RDH);
    let mut printer = Printer::new(700, 220, 0);
    write!(printer, "{:x}", rdh).unwrap();

    let rdt = get_nic_reg(NIC_REG_RDT);
    let mut printer = Printer::new(700, 235, 0);
    write!(printer, "{:x}", rdt).unwrap();

    let rctl = get_nic_reg(NIC_REG_RCTL);
    let mut printer = Printer::new(700, 250, 0);
    write!(printer, "{:x}", rctl).unwrap();
}

pub fn receive_frame() -> Vec<u8> {
    let mut buf: Vec<u8> = vec![];

    let mut current_rxdesc: RxDesc = unsafe { RX_DESC_DATA[*CURRENT_RX_IDX.lock()] };

    // let mut printer = Printer::new(100, 130, 0);
    // write!(printer, "{:?}", current_rxdesc.status & NIC_RDESC_STAT_DD == NIC_RDESC_STAT_DD).unwrap();
    //

    // let mut printer = Printer::new(100, 130, 0);
    // write!(printer, "{:x}", &mut current_rxdesc as *mut RxDesc as u32).unwrap();
    //
    // let mut printer = Printer::new(100, 145, 0);
    // write!(printer, "{:?}", current_rxdesc.status).unwrap();
    //
    // let mut printer = Printer::new(100, 160, 0);
    // write!(printer, "{:x}", &mut current_rxdesc.status as *mut u8 as u32).unwrap();

    //
    // let mut printer = Printer::new(100, 175, 0);
    // write!(printer, "{:?}", current_rxdesc.errors).unwrap();
    //
    // let mut printer = Printer::new(100, 190, 0);
    // write!(printer, "{:?}", current_rxdesc.recv_buf_addr.desc_base_low).unwrap();
    // dump_nic_reg_for_net();

    if current_rxdesc.status & NIC_RDESC_STAT_DD == NIC_RDESC_STAT_DD {
        let mut printer = Printer::new(100, 190, 0);
        write!(printer, "{:?}", current_rxdesc.length).unwrap(); // この時点でICMPは4バイト多くなってる
        for idx in 0..current_rxdesc.length {
            let byte = unsafe { *(current_rxdesc.recv_buf_addr.desc_base_low as *mut u8).offset(idx as isize) };
            buf.push(byte);
        }
        // current_rxdesc.status = 0;
        unsafe { write_mem!(&mut RX_DESC_DATA[*CURRENT_RX_IDX.lock()].status as *mut u8, 0) };
        set_nic_reg(NIC_REG_RDT, unsafe { *CURRENT_RX_IDX.lock() as u32 });
        unsafe {
            let idx = {
                (*CURRENT_RX_IDX.lock()).clone()
            };
            *CURRENT_RX_IDX.lock() = (idx + 1) % RXDESC_NUM;
        }
    }
    return buf;
}

pub fn tx_init() {
    let mut tx_desc_data_for_initialization: [TxDesc; TXDESC_NUM] = [TxDesc::new(); TXDESC_NUM];
    for (idx, cur_rxdesc) in unsafe { TX_DESC_DATA.iter_mut().enumerate() } {
        (*cur_rxdesc).tx_buf_address.desc_base_low = 0x00000000;
        (*cur_rxdesc).tx_buf_address.desc_base_high = 0x00000000;
        tx_desc_data_for_initialization[idx] = *cur_rxdesc;
    }
    unsafe { TX_DESC_DATA = tx_desc_data_for_initialization; }

    /* txdescの先頭アドレスとサイズをNICレジスタへ設定 */
    set_nic_reg(NIC_REG_TDBAH, 0x00);
    set_nic_reg(NIC_REG_TDBAL, unsafe { &mut TX_DESC_DATA as *mut [TxDesc; TXDESC_NUM] as u32 });
    set_nic_reg(NIC_REG_TDLEN, tx_desc_data_size() as u32);

    set_nic_reg(NIC_REG_TDH, unsafe { *CURRENT_TX_IDX.lock() as u32 });
    set_nic_reg(NIC_REG_TDT, unsafe { *CURRENT_TX_IDX.lock() as u32 });

    set_nic_reg(NIC_REG_TCTL, (0x40 << NIC_TCTL_COLD_SHIFT) | (0x0f << NIC_TCTL_CT_SHIFT) | NIC_TCTL_PSP | NIC_TCTL_EN);

    let mut printer = Printer::new(700, 400, 0);
    write!(printer, "{:x}", unsafe { &mut TX_DESC_DATA as *mut [TxDesc; TXDESC_NUM] as u32 }).unwrap();
    let mut printer = Printer::new(700, 415, 0);
    write!(printer, "{:?}", unsafe { &mut TX_DESC_DATA[0] as *mut TxDesc }).unwrap();
    let mut printer = Printer::new(700, 430, 0);
    write!(printer, "{:x}", unsafe { TX_DESC_DATA[0].tx_buf_address.low() }).unwrap();
    let mut printer = Printer::new(700, 445, 0);
    write!(printer, "{:x}", unsafe { TX_DESC_DATA[0].tx_buf_address.high() }).unwrap();
    let mut printer = Printer::new(700, 460, 0);
    write!(printer, "{:x}", unsafe { &mut TX_DESC_DATA[0].tx_buf_address as *mut BufferAddr as u32 }).unwrap();
    let mut printer = Printer::new(700, 475, 0);
    write!(printer, "{:x}", unsafe { &mut TX_DESC_DATA[1].tx_buf_address as *mut BufferAddr as u32 }).unwrap();
}


pub fn send_frame(mut buf: Vec<u8>) -> u8 {
    unsafe {
        let current_idx = unsafe { *CURRENT_TX_IDX.lock() };
        let mut slice: &[u8] = &[&buf[..]].concat();
        let mut dma_buf: DmaBox<[u8]> = DmaBox::from(slice);

        let idx = { (*CURRENT_TX_IDX.lock()).clone() };
        *CURRENT_TX_IDX.lock() = (idx + 1) % TXDESC_NUM;

        TX_DESC_DATA[current_idx].tx_buf_address.desc_base_low = dma_buf.as_ptr() as u32;
        TX_DESC_DATA[current_idx].length = buf.len() as u16;
        TX_DESC_DATA[current_idx].sta_rsv = 0x00;
        set_nic_reg(NIC_REG_TDT, unsafe { *CURRENT_TX_IDX.lock() as u32 });

        let mut send_status: u8 = 0;
        while send_status == 0 { send_status = read_mem!(&TX_DESC_DATA[current_idx]).sta_rsv & 0x0f; }
        let mut printer = Printer::new(500, 705, 0);
        write!(printer, "{:x}", send_status).unwrap();
        return send_status;
    };
}

pub fn send_buf_frame(buf: DmaBox<[u8]>) -> u8 {
    unsafe {
        let current_idx = unsafe { *CURRENT_TX_IDX.lock() };
        let idx = { (*CURRENT_TX_IDX.lock()).clone() };
        *CURRENT_TX_IDX.lock() = (idx + 1) % TXDESC_NUM;

        TX_DESC_DATA[current_idx].length = buf.len() as u16;
        TX_DESC_DATA[current_idx].tx_buf_address.desc_base_low = buf.as_ptr() as u32;
        TX_DESC_DATA[current_idx].sta_rsv = 0;

        set_nic_reg(NIC_REG_TDT, unsafe { *CURRENT_TX_IDX.lock() as u32 });
        let mut send_status: u8 = 0;
        while send_status == 0 { send_status = read_mem!(&TX_DESC_DATA[current_idx]).sta_rsv & 0x0f; }
        return send_status;
    };
}

pub fn send_test_frame() -> u8 {
    let buf: [u8; 590]  = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x08, 0x00, 0x27, 0x66, 0x10, 0x65, 0x08, 0x00, 0x45, 0x00,
        0x02, 0x40, 0x0c, 0x54, 0x00, 0x00, 0xff, 0x11, 0xb4, 0x4c, 0xc0, 0xa8, 0x38, 0x80, 0xff, 0xff,
        0xff, 0xff, 0x00, 0x43, 0x00, 0x44, 0x02, 0x2c, 0x86, 0xef, 0x02, 0x01, 0x06, 0x00, 0x84, 0x17,
        0x02, 0xa6, 0x00, 0x00, 0x00, 0x00, 0xc0, 0xa8, 0x38, 0x67, 0xc0, 0xa8, 0x38, 0x67, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x27, 0x99, 0xf3, 0x6c, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63, 0x36, 0x04, 0xc0, 0xa8, 0x38, 0x64,
        0x35, 0x01, 0x02, 0x33, 0x04, 0x00, 0x00, 0x04, 0xb0, 0x01, 0x04, 0xff, 0xff, 0xff, 0x00, 0xff,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    unsafe {
        let current_idx = unsafe { *CURRENT_TX_IDX.lock() };
        TX_DESC_DATA[current_idx].length = buf.len() as u16;
        TX_DESC_DATA[current_idx].tx_buf_address.desc_base_low = buf.as_ptr() as u32;

        TX_DESC_DATA[current_idx].sta_rsv = 0;

        let idx = { (*CURRENT_TX_IDX.lock()).clone() };
        *CURRENT_TX_IDX.lock() = (idx + 1) % TXDESC_NUM;

        set_nic_reg(NIC_REG_TDT, unsafe { *CURRENT_TX_IDX.lock() as u32 });
        let mut send_status: u8 = 0;
        while send_status == 0 { send_status = read_mem!(&TX_DESC_DATA[current_idx]).sta_rsv & 0x0f; }
        return send_status;
    };
}

static mut PRINT_IDX: usize = 0;
pub fn dump_frame() -> usize {
    let receive_buf = receive_frame();

    for c in receive_buf.iter() {
        let mut printer = Printer::new(unsafe { PRINT_IDX as u32 }, 30, 0);
        write!(printer, "{:x}", c).unwrap();
        unsafe { PRINT_IDX += 8; }
    }
    return receive_buf.len();
}

pub fn nic_init() {
    disable_nic_interrupt();
    rx_init();
    get_mac_addr();
    tx_init();
}