#![feature(allocator_api)]

use core::mem::size_of;
use alloc::string::String;
use alloc::vec::Vec;
use super::e1000::get_mac_addr;
use super::ethernet::{ETHERNET_TYPE_ARP, ETHERNET_TYPE_IP, HARDWARE_TYPE_ETHERNET, EthernetHdr, send_ethernet_packet};
use super::net_util::{switch_endian16, switch_endian32, any_as_u8_vec, any_as_u8_slice, push_to_vec};

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;

use crate::memory::dma::{
    DmaBox,
};


enum ArpType {
    ArpRequest = 1,
    AroReply = 2,
}

#[derive(Clone, Copy)]
struct ArpTableEntry {
    ip_addr: [u8; 4],
    mac_addr: [u8; 6],
}

impl ArpTableEntry {
    const fn new_const() -> Self {
        ArpTableEntry {
            ip_addr: [0x0, 0x0, 0x0, 0x0],
            mac_addr: [0x0, 0x0, 0x0, 0x0, 0x0, 0x0],
        }
    }
}

const ARP_TABLE_NUM: usize = 512;
static mut ARP_TABLE: [ArpTableEntry; ARP_TABLE_NUM] = [ArpTableEntry::new_const(); ARP_TABLE_NUM];


const BROADCAST_MAC_ADDR: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

#[repr(C)]
struct Arp {
    hardware_type: u16,
    protocol: u16,
    hardware_addr_len: u8,
    protocol_addr_len: u8,
    opcode: u16,
    src_hardware_addr: [u8; 6],
    src_protocol_addr: [u8; 4],
    dst_hardware_addr: [u8; 6],
    dst_protocol_addr: [u8; 4],
}

impl Arp {
    fn to_slice(&self) -> DmaBox<[u8]> {
        let mut slice: &[u8] = &self.hardware_type.to_be_bytes();
        let mut slice: &[u8] = &[&slice[..], &self.protocol.to_be_bytes()].concat();
        let mut slice: &[u8] = &[&slice[..], &self.hardware_addr_len.to_be_bytes()].concat();
        let mut slice: &[u8] = &[&slice[..], &self.protocol_addr_len.to_be_bytes()].concat();
        let mut slice: &[u8] = &[&slice[..], &self.opcode.to_be_bytes()].concat();
        let mut slice: &[u8] = &[&slice[..], &self.src_hardware_addr[..]].concat();
        let mut slice: &[u8] = &[&slice[..], &self.src_protocol_addr[..]].concat();
        let mut slice: &[u8] = &[&slice[..], &self.dst_hardware_addr[..]].concat();
        let mut s: &[u8] = &[&slice[..], &self.dst_protocol_addr[..]].concat();
        let mut printer = Printer::new(100, 560, 0);
        write!(printer, "{:?}", s.len()).unwrap();
        unsafe {
            DmaBox::from(s)
        }
    }
}


pub fn send_arp_packet(dst_hardware_addr: &[u8; 6], dst_protocol_addr: &[u8; 4]) -> Result<(), String> {
    let src_mac_addr: [u8; 6] = get_mac_addr();
    // let src_protocol_addr: [u8; 4] = [10, 0, 2, 14];
    let src_protocol_addr: [u8; 4] = [192, 168, 56, 102];
    let hardware_addr_len: u8 = 6;
    let protocol_addr_len: u8 = 4;
    let arp_opcode = ArpType::ArpRequest; // 1
    let hardware_type = HARDWARE_TYPE_ETHERNET; // 0x01
    let protocol = ETHERNET_TYPE_IP; // 0x0800

    let arp_packet = Arp {
        // hardware_type: switch_endian16(hardware_type),
        hardware_type,
        // protocol: switch_endian16(protocol),
        protocol,
        hardware_addr_len,
        protocol_addr_len,
        // opcode: switch_endian16(arp_opcode as u16),
        opcode: arp_opcode as u16,
        src_hardware_addr: src_mac_addr,
        src_protocol_addr,
        dst_hardware_addr: [dst_hardware_addr[0], dst_hardware_addr[1], dst_hardware_addr[2], dst_hardware_addr[3], dst_hardware_addr[4], dst_hardware_addr[5]],
        dst_protocol_addr: [dst_protocol_addr[0], dst_protocol_addr[1], dst_protocol_addr[2], dst_protocol_addr[3]],
    };
    let v = arp_packet.to_slice();
    let mut printer = Printer::new(700, 700, 0);
    // write!(printer, "{:x}", &v as *const Vec<u8> as u32).unwrap();
    write!(printer, "{:x}", v.as_ptr() as *const u8 as u32).unwrap();
    let mut printer = Printer::new(700, 715, 0);
    write!(printer, "{:x}", v[0]).unwrap();
    let mut printer = Printer::new(700, 730, 0);
    write!(printer, "{:x}", v[2]).unwrap();
    send_ethernet_packet(BROADCAST_MAC_ADDR, v, size_of::<Arp>(), ETHERNET_TYPE_ARP)
}
