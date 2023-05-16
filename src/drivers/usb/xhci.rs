use alloc::borrow::ToOwned;
use super::super::bus::pci;
use crate::memory::dma::{DmaBox, DmaRawVec};
use alloc::vec::Vec;
use core::cmp::max;

use core::fmt::Write;
use crate::arch::graphic::{Graphic, Printer, print_str};
use crate::{lazy_static, read_mem, write_mem};

const MOUSE_USB_DEV_NUM: u8 = 0x04;

struct CapLength(u8);
impl CapLength {
    fn new(cap_length: i32) -> CapLength {
        CapLength((cap_length & 0xff) as u8)
    }
    fn read_from_reg(cap_base_reg: u32) -> CapLength {
        // CapLength::new(pci::get_pci_conf_reg_from_memory(xhc_memory_addr))
        CapLength::new(unsafe { *(*&mut (cap_base_reg + 0x00 as u32) as *mut i32) })
    }
    fn read(&self) -> u8 {
        self.0
    }
}

struct DbOff(u16);
impl DbOff {
    fn new(db_off: i32) -> DbOff { DbOff((db_off & 0xffff) as u16) }
    fn read_from_reg(cap_base_reg: u32) -> DbOff {
        DbOff::new(unsafe { *(*&mut (cap_base_reg + 0x14 as u32) as *mut i32) })
    }
    fn read(&self) -> u16 { self.0 }
}

struct RtsOff(u16);
impl RtsOff {
    fn new(rts_off: i32) -> RtsOff { RtsOff((rts_off & 0xffff) as u16) }
    fn read_from_reg(cap_base_reg: u32) -> RtsOff {
        RtsOff::new(unsafe { *(*&mut (cap_base_reg + 0x18 as u32) as *mut i32) })
    }
    fn read(&self) -> u16 { self.0 }
}


struct HcsParam1(u32);
impl HcsParam1 {
    fn new(params: i32) -> HcsParam1 { HcsParam1(params as u32) }
    fn read_from_reg(cap_base_reg: u32) -> HcsParam1 {
        HcsParam1::new(unsafe { *((cap_base_reg + 0x04 as u32) as *mut i32) })
    }
    fn max_device_slot(&self) -> u8 {
        (self.0 & 0xff) as u8
    }
    fn max_ports(&self) -> u8 {
        (self.0 >> 24) as u8
    }
    fn max_interrupters(&self) -> u16 {
        (self.0 >> 8 & 0xffff) as u16
    }
}

struct HcsParam2(u32);
impl HcsParam2 {
    fn new(params: u32) -> HcsParam2 {
        HcsParam2(params)
    }
    fn read_from_reg(cap_base_reg: u32) -> HcsParam2 {
        HcsParam2::new(read_mem!((cap_base_reg + 0x08) as *mut u32))
    }
    fn max_scratchpad(&self) -> u16 {
        let lo = self.max_scratchpad_lo();
        let high = self.max_scratchpad_high();
        ((high as u16) << 5) | (lo as u16)
    }
    fn max_scratchpad_lo(&self) -> u8 {
        (self.0 >> 26 & 0b00011111 ) as u8
    }
    fn max_scratchpad_high(&self) -> u8 {
        (self.0 >> 20 & 0b00011111) as u8
    }
    fn erst_max(&self) -> u8 {
        (self.0 >> 4 & 0xf) as u8
    }
    fn ist(&self) -> u8 {
        (self.0 & 0xf) as u8
    }
}

struct HccParam1(u32);
impl HccParam1 {
    fn new(params: u32) -> HccParam1 { HccParam1(params) }
    fn read_from_reg(cap_base_reg: u32) -> HccParam1 {
        HccParam1::new(read_mem!((cap_base_reg + 0x10) as *mut u32))
    }
    fn csz(&self) -> u8 {
        (self.0 >> 2 & 1) as u8
    }
    fn is_context_data_structure_64bit(&self) -> bool { self.csz() == 1 }
}

struct UsbCmd(*mut u32);
impl UsbCmd {
    fn new(addr: *mut u32) -> UsbCmd {
        UsbCmd(addr)
    }
    fn read_from_reg(ops_base_reg: u32) -> UsbCmd { UsbCmd::new((ops_base_reg + 0x0) as *mut u32) }
    unsafe fn read(&self) -> u32 { read_mem!(self.0) }
    fn hcrst_is_on(&self) -> bool { unsafe { self.read() >> 1 & 0x1 == 1 } }
    fn set_hcrst_on(&mut self) { unsafe { *self.0 = *self.0 | 0b10; } }
    fn set_stop(&mut self) { unsafe { *self.0 = *self.0 & 0xfffffffe; } }
}

struct UsbSts(*mut u32);
impl UsbSts {
    fn new(addr: *mut u32) -> UsbSts {
        UsbSts(addr)
    }
    fn read_from_reg(ops_base_addr: u32) -> UsbSts { UsbSts::new((ops_base_addr + 0x4) as *mut u32) }
    // unsafe fn read(&self) -> u32 { *self.0 }
    unsafe fn read(&self) -> u32 { read_mem!(self.0) }
    fn hch_is_on(&self) -> bool { unsafe { self.read() & 0b1 == 1 } }
    fn cnr_is_on(&self) -> bool { unsafe { self.read() >> 11 & 0b1 == 1 } }
}

#[repr(packed)]
struct PageSize(u32);
impl PageSize {
    fn new(addr: u32) -> PageSize { PageSize(addr) }
    fn read_from_reg(ops_base_addr: u32) -> PageSize {
        PageSize::new(unsafe { *((ops_base_addr + 0x8) as *mut u32) })
    }
    fn read(&self) -> u32 { self.0 }

    fn read_page_size(&self) -> u16 {
        (self.read() & 0x0000ffff) as u16
    }

    // Page Size – RO. Default = Implementation defined. This field defines the page size supported by the xHC implementation.
    // This xHC supports a page size of 2^(n+12) if bit n is Set. For example, if bit 0 is Set, the xHC supports 4k byte page sizes.
    // For a Virtual Function, this register reflects the page size selected in the System Page Size field of
    // the SR-IOV Extended Capability structure. For the Physical Function 0,
    // this register reflects the implementation dependent default xHC page size.
    // Various xHC resources reference PAGESIZE to describe their minimum alignment requirements. The maximum possible page size is 128M.
    fn calc_page_size(&self) -> u16 {
        let page_size = self.read_page_size();
        let mut num_of_least_significant_bit = 0;
        for i in 0..16 {
            if page_size >> i & 0b1 == 1 {
                let num_of_least_significant_bit = i;
            }
        };
        2_i32.pow(num_of_least_significant_bit + 12) as u16
    }
}

#[repr(C, packed)]
struct Dcbaap {
    lo: *mut u32,
    high: *mut u32,
}

impl Dcbaap {
    fn new(lo: *mut u32, high: *mut u32) -> Dcbaap {
        Dcbaap { lo, high }
    }
    fn read_from_reg(ops_base_addr: u32) -> Dcbaap {
        unsafe {
            let low = (ops_base_addr + 0x30) as *mut u32;
            let high = (ops_base_addr + 0x34) as *mut u32;
            Dcbaap::new(low, high)
        }
    }
    fn set_dcbaap(&mut self, addr: u32) {
        unsafe { *self.lo = addr };
    }
}

struct Config(*mut u32);
impl Config {
    fn new(reg: *mut u32) -> Config {
        Config(reg)
    }
    unsafe fn read(&self) -> u32 { read_mem!(self.0) }
    fn read_from_reg(ops_base_addr: u32) -> Config {
        Config::new((ops_base_addr + 0x38) as *mut u32)
    }
    fn set_num_of_enable_devices(&mut self, num: u8) {
        unsafe { *self.0 = *self.0 | (num as u32); }
    }

}

// static mut DEVICE_CTX_BASE_ADDRESS_ARRAY: Vec<DeviceCtxBaseAddress> = Vec::new();

#[repr(C, packed)]
struct DeviceCtxBaseAddress {
    low: *mut usize,
    high: *mut usize,
}

impl DeviceCtxBaseAddress {
    fn new() -> Self {
        DeviceCtxBaseAddress {
            low: 0 as *mut usize,
            high: 0 as *mut usize,
        }
    }
    fn set_ptr(&mut self, addr: *mut usize) {
        self.low = addr;
    }
    // fn fetch_base_addr(&self) -> u32 {}
}

#[repr(transparent)]
struct ScratchpadBuffer<const N: usize> {
    buffer: [u32; N]
}

impl <const N: usize> ScratchpadBuffer<N> {
    fn new() -> ScratchpadBuffer<N> {
        ScratchpadBuffer {
            buffer: [3; N]
        }
    }
    fn as_ptr(&self) -> *mut usize {
        self.buffer.as_ptr() as *const _ as *mut usize
    }
}

// static mut DEVICE_32_ARRAY: Vec<Device<32>> = Vec::new();
// static mut DEVICE_64_ARRAY: Vec<Device<32>> = Vec::new();

pub const NUM_OF_ENDPOINTS: usize = 31;
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Device<const N: usize> {
    slot: Slot<N>,
    endpoints: [Endpoint<N>; NUM_OF_ENDPOINTS],
}

impl<const N: usize> Device<N> {
    fn new() -> Device<N> {
        Device {
            slot: Slot::new(),
            endpoints: [Endpoint::new(); NUM_OF_ENDPOINTS],
        }
    }
    fn as_ptr(&self) -> *mut usize {
        self as *const _ as *mut usize
    }
}

/// the Context Size (CSZ) field in the HCCPARAMS1 register = '0'.
/// If the Context Size (CSZ) field = '1' then the Device Context data structures consume 64 bytes each.
/// The offsets shall be 040h for the EP Context 0, 080h for EP Context 1, and so on.
#[repr(transparent)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Slot<const N: usize>([u32; N]);
impl<const N: usize> Slot<N> {
    const fn new() -> Slot<N> {
        Slot([1; N])
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Endpoint<const N: usize>([u32; N]);
impl<const N: usize> Endpoint<N> {
    const fn new() -> Endpoint<N> {
        Endpoint([2; N])
    }
}

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct TransferRequestBlock {
    parameter1: u32,
    parameter2: u32,
    status: u32,
    flags: u8,
    trb_type: u8,
    control: u16,
}


pub fn initialize_xhci() -> Result<(), ()> {
    let mut height = 255;
    if let Some(capability_base_addr) = pci::search_xhc_base_addr() {
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "capability_base".to_owned()).unwrap();
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", capability_base_addr.base_addr()).unwrap();

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "CapLength".to_owned()).unwrap();
        let cap_length = CapLength::read_from_reg(capability_base_addr.base_addr());
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", cap_length.read()).unwrap();

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "operational_base".to_owned()).unwrap();
        let operational_base_addr = capability_base_addr.base_addr() + (cap_length.read() as u32);
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", operational_base_addr).unwrap();

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "RTSOFF".to_owned()).unwrap();
        let rts_off = RtsOff::read_from_reg(capability_base_addr.base_addr());
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", rts_off.read()).unwrap();

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "runtime_base".to_owned()).unwrap();
        let runtime_base_addr = capability_base_addr.base_addr() + (rts_off.read() as u32);
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", runtime_base_addr).unwrap();

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "DBOFF".to_owned()).unwrap();
        let db_off = DbOff::read_from_reg(capability_base_addr.base_addr());
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", db_off.read()).unwrap();

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "doorbell_base".to_owned()).unwrap();
        let doorbell_base_addr = capability_base_addr.base_addr() + (db_off.read() as u32);
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", doorbell_base_addr).unwrap();


        // USBSTS.HCHがoff(halted)である場合、USBCMD.R/Sを0に
        if !UsbSts::read_from_reg(operational_base_addr).hch_is_on() {
            UsbCmd::read_from_reg(operational_base_addr).set_stop();
        }

        height += 15;
        // USBSTS.HCHが1であることを確認
        // HCH: Host Controller Haltedビット、xHCが停止状態であることを示す
        while !UsbSts::read_from_reg(operational_base_addr).hch_is_on() {}

        // USBCMD.HCRSTに1を書く
        // HCRST: Host Controller Resetビット、1を書き込むとxHCがリセット動作を開始する。リセットが完了するとxHCにより0クリアされる。HCHが0の時にHCRSTに1を書き込んではいけない
        UsbCmd::read_from_reg(operational_base_addr).set_hcrst_on();

        // USBCMD.HCRSTが0になるまで待つ
        while UsbCmd::read_from_reg(operational_base_addr).hcrst_is_on() {}
        // USBSTS.CNRが0になるまで待つ
        // CNR: Controller Not Readyビット、xHCの準備が整っていないことを示す。このビットが0になるまでは、DoorbellレジスタやUSBSTS以外のOperationalレジスタには書き込んではいけない
        while UsbSts::read_from_reg(operational_base_addr).cnr_is_on() {}

        // デバイスコンテキストの設定
        let max_slots = HcsParam1::read_from_reg(capability_base_addr.base_addr()).max_device_slot();
        let mut config = Config::read_from_reg(operational_base_addr);
        config.set_num_of_enable_devices(max_slots);

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "max_slots".to_owned()).unwrap();
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", max_slots).unwrap();

        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "config".to_owned()).unwrap();
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", unsafe { config.read() }).unwrap();

        // 1. examine the Max Scratched Buffers Hi and Lo fields in the HCSPARAMS2 register.
        let max_scratchpad_buffers_entry = HcsParam2::read_from_reg(capability_base_addr.base_addr()).max_scratchpad();
        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "mx_scrtchpd_buffs".to_owned()).unwrap();
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:?}", max_scratchpad_buffers_entry).unwrap();
        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "HcsParam2".to_owned()).unwrap();
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:?}", HcsParam2::read_from_reg(capability_base_addr.base_addr()).0).unwrap();

        // ToDo max_scratchpad_buffers_entryは0なのか・・・？？ 0として適当に1つとして設定しておくか

        let page_size = PageSize::read_from_reg(operational_base_addr).calc_page_size();
        height += 15;
        let mut printer = Printer::new(50, height, 0);
        write!(printer, "{:?}", "page_size".to_owned()).unwrap();
        let mut printer = Printer::new(200, height, 0);
        write!(printer, "{:x}", page_size).unwrap();

        // let page_size = if max_scratchpad_buffers_entry > 0 {
        //     // 2. allocates a Scratched Buffer Array with Max Scratchpad Buffers entry.
        //     // Scratchpad Buffer Array is used to define the locations of statically allocated memory pages that are available for the private use of the xHC.
        //     let page_size = PageSize::read_from_reg(operational_base_addr).read_page_size();
        //
        //     height += 15;
        //     let mut printer = Printer::new(200, height, 0);
        //     write!(printer, "{:x}", page_size).unwrap();
        //     page_size
        // } else {
        //     0
        // };

        // 3. initialize Device Context Base Address Array
        if HccParam1::read_from_reg(capability_base_addr.base_addr()).is_context_data_structure_64bit() {
            height += 15;
            let mut printer = Printer::new(50, height, 0);
            write!(printer, "{:?}", "64bit align is not implemented".to_owned()).unwrap();

            Dcbaap::read_from_reg(operational_base_addr);
        } else {
            let mut dcbaap = Dcbaap::read_from_reg(operational_base_addr);
            height += 15;
            let mut printer = Printer::new(50, height, 0);
            write!(printer, "{:?}", "dcbaap".to_owned()).unwrap();
            let mut printer = Printer::new(200, height, 0);
            write!(printer, "{:x}", dcbaap.lo as u32).unwrap();

            unsafe {
                // DEVICE_CTX_BASE_ADDRESS_ARRAY = Vec::with_capacity(max_slots as usize + 1);
                // DEVICE_32_ARRAY = Vec::with_capacity(max_slots as usize);
                // let mut device_ctx_base_address_array: Vec<DeviceCtxBaseAddress> = Vec::with_capacity(max_slots as usize + 1);
                let mut device_ctx_base_address_array: DmaBox<[DeviceCtxBaseAddress]> = (DmaRawVec::with_capacity(max_slots as usize + 1) as DmaRawVec<DeviceCtxBaseAddress>).into_box();
                let mut device_array: DmaBox<[Device<8>]> = (DmaRawVec::with_capacity(max_slots as usize) as DmaRawVec<Device<8>>).into_box();

                // 最初のelementはScratchpad Buffer Arrayのポインター
                // writes the base address of the Scratchpad Buffer Array to the DCBAA(Slot 0) entry.
                if max_scratchpad_buffers_entry != 0 {
                    // 4. For each entry in the Scratchpad Buffer Array.
                    // 4 - a. allocates a PAGESIZE Scratchpad Buffer.
                    // 4 - b. clears the Scratchpad Buffer to '0'.
                    // 4 - c. writes the base address of the allocated Scratchpad Buffer to associated entry in the Scratchpad Buffer Array.
                    // 4 - d. writes the base address of the Scratchpad Buffer Array to the DCBAA(Slot 0) entry.
                    // if page_size == 1024 {
                    //
                    // }
                    // let mut scratchpad_buffer_array: DmaBox<[ScratchpadBuffer<>]> = (DmaRawVec::with_capacity(max_slots as usize + 1) as DmaRawVec<ScratchpadBuffer>).into_box();
                    // for idx in 0..max_scratchpad_buffers_entry {
                    //     scratchpad_buffer_array[idx as usize] = ScratchpadBuffer::new();
                    // }
                    // let mut dcba = DeviceCtxBaseAddress::new();
                    // dcba.set_ptr(scratchpad_buffer_array[0].as_ptr());
                    // // device_ctx_base_address_array.push(dcba);
                    // device_ctx_base_address_array[0] = dcba;
                } else {
                    if page_size == 0x1000 { // page_sizeが4kb
                        let mut scratchpad_buffer_array: DmaBox<[ScratchpadBuffer<1024>]> = (DmaRawVec::with_capacity(1) as DmaRawVec<ScratchpadBuffer<1024>>).into_box();
                        scratchpad_buffer_array[0] = ScratchpadBuffer::new();
                        let mut dcba = DeviceCtxBaseAddress::new();
                        dcba.set_ptr(scratchpad_buffer_array[0].as_ptr());
                        device_ctx_base_address_array[0] = dcba;
                    }
                }

                // そもそもDMAを使うべきなのかどうか？初期化した値をどう保持させるか？
                for idx in 0..max_slots {
                    // device_array.push(Device::new());
                    device_array[idx as usize] = Device::new() as Device<8>;

                    let mut dcba = DeviceCtxBaseAddress::new();
                    dcba.set_ptr(device_array[idx as usize].as_ptr());
                    device_ctx_base_address_array[idx as usize + 1] = dcba;
                }
                dcbaap.set_dcbaap(device_ctx_base_address_array.as_ptr() as u32);

                // DeviceCtxBaseAddressは問題なく設定されており、device_ctx_base_address_arrayの配列ないでそれぞれDeviceCtxBaseAddress.lowでDeviceを向いていそう
                // ToDo dcbaの1つ目のDeviceについて、0xb0e00000 - 0xb0e00008 までで、0x00200000, 0x00000000 がsetされているのはなぜか確認する
                // ToDo また、device_ctx_base_address_array[0]がScratchpad Buffer Arrayに向いているかどうかだが、Scratchpad Buffer Arrayはまだ初期化できてないので、DeviceCtxBaseAddress::new();にしている

                // height += 15;
                // let mut printer = Printer::new(50, height, 0);
                // write!(printer, "{:?}", "DEVICE_CTX_A_A".to_owned()).unwrap();
                // let mut printer = Printer::new(200, height, 0);
                // write!(printer, "{:x}", &mut device_ctx_base_address_array as *mut _ as u32).unwrap();
                // let mut printer = Printer::new(300, height, 0);
                // write!(printer, "{:x}", device_ctx_base_address_array.len()).unwrap();
                //
                // height += 15;
                // let mut printer = Printer::new(50, height, 0);
                // write!(printer, "{:?}", "device_array".to_owned()).unwrap();
                // let mut printer = Printer::new(200, height, 0);
                // write!(printer, "{:x}", &mut device_array as *mut _ as u32).unwrap();
            }
        }
        loop {}
    }
    return Err(())
}