#![feature(allocator_api)]

use core::mem::size_of;
use alloc::string::String;
use alloc::vec::Vec;
use super::e1000::get_mac_addr;
use super::ethernet::{ETHERNET_TYPE_ARP, ETHERNET_TYPE_IP, HARDWARE_TYPE_ETHERNET, EthernetHdr, send_ethernet_packet};
use super::net_util::{switch_endian16, switch_endian32, any_as_u8_vec, any_as_u8_slice, push_to_vec};
use super::ip::DEFAULT_MY_IP;

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;
use core::ops::Add;

use crate::memory::dma::{
    DmaBox,
};


#[repr(u16)]
#[derive(Clone, Copy)]
enum ArpType {
    ArpRequest = 1,
    ArpReply = 2,
}

impl ArpType {
    fn is_reply(opcode: u16) -> bool {
        opcode == ArpType::ArpReply as u16
    }
}

#[derive(Clone, Copy)]
pub struct ArpTableEntry {
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

    fn is_initial_state(&self) -> bool {
        self.ip_addr == [0x0, 0x0, 0x0, 0x0] && self.mac_addr == [0x0, 0x0, 0x0, 0x0, 0x0, 0x0]
    }

    fn same_ip_addr(&self, ip: &[u8; 4]) -> bool {
        if ip == &[0x0, 0x0, 0x0, 0x0] { return false; }
        ip == &self.ip_addr
    }

    fn same_mac_addr(&self, mac_addr: &[u8; 6]) -> bool {
        if mac_addr == &[0x0, 0x0, 0x0, 0x0, 0x0, 0x0] { return false; }
        mac_addr == &self.mac_addr
    }

    pub fn get_ip_addr(&self) -> [u8; 4] {
        self.ip_addr
    }

    pub fn get_mac_addr(&self) -> [u8; 6] {
        self.mac_addr
    }
}

struct ArpTable([ArpTableEntry; ARP_TABLE_NUM]);

impl ArpTable {
    const fn new() -> Self {
        ArpTable([ArpTableEntry::new_const(); ARP_TABLE_NUM])
    }

    fn addable_idx(&self, ip: &[u8; 4]) -> usize {
        for (idx, entry) in self.0.iter().enumerate() {
            if entry.is_initial_state() { return idx; }
            if entry.same_ip_addr(ip) { return idx; }
        }
        return self.0.len() - 1;
    }

    fn add(&mut self, ip_addr: [u8; 4], mac_addr: [u8; 6]) {
        let idx = self.addable_idx(&ip_addr);
        self.0[idx] = ArpTableEntry {
            ip_addr,
            mac_addr,
        };
    }

    fn get_mac_addr(&self, ip_addr: &[u8; 4]) -> Option<[u8; 6]> {
        for entry in self.0.iter() {
            if entry.same_ip_addr(ip_addr) { return Some(entry.mac_addr) }
        }
        None
    }

    fn get_ip_addr(&self, mac_addr: &[u8; 6]) -> Option<[u8; 4]> {
        for entry in self.0.iter() {
            if entry.same_mac_addr(mac_addr) { return Some(entry.ip_addr) }
        }
        None
    }

    fn get_entry_from_ip_addr(&self, ip_addr: &[u8; 4]) -> Option<ArpTableEntry> {
        for entry in self.0.iter() {
            if entry.same_ip_addr(ip_addr) { return Some(entry.clone()) }
        }
        None
    }

    fn get_entry_from_mac_addr(&self, mac_addr: &[u8; 6]) -> Option<ArpTableEntry> {
        for entry in self.0.iter() {
            if entry.same_mac_addr(mac_addr) { return Some(entry.clone()) }
        }
        None
    }
}

const ARP_TABLE_NUM: usize = 512;
// static mut ARP_TABLE: [ArpTableEntry; ARP_TABLE_NUM] = [ArpTableEntry::new_const(); ARP_TABLE_NUM];
static mut ARP_TABLE: ArpTable = ArpTable::new();

const BROADCAST_MAC_ADDR: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

#[repr(C)]
pub struct Arp {
    hardware_type: u16,
    protocol: u16,
    hardware_addr_len: u8,
    protocol_addr_len: u8,
    opcode: ArpType,
    src_hardware_addr: [u8; 6],
    src_protocol_addr: [u8; 4],
    dst_hardware_addr: [u8; 6],
    dst_protocol_addr: [u8; 4],
}

impl Arp {
    fn to_slice(&self) -> DmaBox<[u8]> {
        let slice: &[u8] = &self.hardware_type.to_be_bytes();
        let slice: &[u8] = &[&slice[..], &self.protocol.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.hardware_addr_len.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.protocol_addr_len.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &(self.opcode as u16).to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.src_hardware_addr[..]].concat();
        let slice: &[u8] = &[&slice[..], &self.src_protocol_addr[..]].concat();
        let slice: &[u8] = &[&slice[..], &self.dst_hardware_addr[..]].concat();
        let s: &[u8] = &[&slice[..], &self.dst_protocol_addr[..]].concat();
        let mut printer = Printer::new(100, 560, 0);
        write!(printer, "{:?}", s.len()).unwrap();
        unsafe {
            DmaBox::from(s)
        }
    }

    pub fn parse_buf(data: DmaBox<[u8]>) -> Option<Arp> {
        let protocol = (data[2] as u16) << 8 | data[3] as u16;
        let opcode =  if ArpType::is_reply((data[6] as u16) << 8 | data[7] as u16) { ArpType::ArpReply } else { ArpType::ArpRequest };

        if protocol == ETHERNET_TYPE_IP {
            Some(Self {
                hardware_type: (data[0] as u16) << 8 | data[1] as u16,
                protocol,
                hardware_addr_len: data[4],
                protocol_addr_len: data[5],
                opcode,
                src_hardware_addr: [data[8], data[9], data[10], data[11], data[12], data[13]],
                src_protocol_addr: [data[14], data[15], data[16], data[17]],
                dst_hardware_addr: [data[18], data[19], data[20], data[21], data[22], data[23]],
                dst_protocol_addr: [data[24], data[25], data[26], data[27]],
            })
        } else {
            None
        }
    }

    pub fn get_src_hardware_addr(&self) -> [u8; 6] {
        self.src_hardware_addr
    }
}


pub fn send_arp_packet(dst_hardware_addr: &[u8; 6], dst_protocol_addr: &[u8; 4]) -> Result<(), String> {
    let src_mac_addr: [u8; 6] = get_mac_addr();
    // let src_protocol_addr: [u8; 4] = [10, 0, 2, 14];
    let src_protocol_addr: [u8; 4] = DEFAULT_MY_IP;
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
        opcode: arp_opcode,
        src_hardware_addr: src_mac_addr,
        src_protocol_addr,
        dst_hardware_addr: [dst_hardware_addr[0], dst_hardware_addr[1], dst_hardware_addr[2], dst_hardware_addr[3], dst_hardware_addr[4], dst_hardware_addr[5]],
        dst_protocol_addr: [dst_protocol_addr[0], dst_protocol_addr[1], dst_protocol_addr[2], dst_protocol_addr[3]],
    };
    let v = arp_packet.to_slice();
    let mut printer = Printer::new(700, 700, 0);
    write!(printer, "{:x}", v.as_ptr() as *const u8 as u32).unwrap();
    send_ethernet_packet(BROADCAST_MAC_ADDR, v, size_of::<Arp>(), ETHERNET_TYPE_ARP)
}

pub fn receive_arp_packet(buf: DmaBox<[u8]>) -> Option<ArpTableEntry> {
    let parsed_arp = Arp::parse_buf(buf);
    if let Some(arp) = parsed_arp {
        match arp.opcode {
            ArpType::ArpReply => receive_arp_reply(arp),
            ArpType::ArpRequest => {
                send_reply_arp(arp);
                None
            },
        }
    } else {
        None
    }
}

pub fn send_reply_arp(arp: Arp) -> Result<(), String> {
    // 自分のIPじゃなかったらそのまま終了
    if arp.dst_protocol_addr != DEFAULT_MY_IP { return Ok(()); }

    let src_mac_addr: [u8; 6] = get_mac_addr();
    let src_protocol_addr: [u8; 4] = DEFAULT_MY_IP;
    let hardware_addr_len: u8 = 6;
    let protocol_addr_len: u8 = 4;
    let arp_opcode = ArpType::ArpReply; // 1
    let hardware_type = HARDWARE_TYPE_ETHERNET; // 0x01
    let protocol = ETHERNET_TYPE_IP; // 0x0800

    let arp_packet = Arp {
        hardware_type,
        protocol,
        hardware_addr_len,
        protocol_addr_len,
        opcode: arp_opcode,
        src_hardware_addr: src_mac_addr,
        src_protocol_addr,
        dst_hardware_addr: [arp.src_hardware_addr[0], arp.src_hardware_addr[1], arp.src_hardware_addr[2], arp.src_hardware_addr[3], arp.src_hardware_addr[4], arp.src_hardware_addr[5]],
        dst_protocol_addr: [arp.src_protocol_addr[0], arp.src_protocol_addr[1], arp.src_protocol_addr[2], arp.src_protocol_addr[3]],
    };
    let v = arp_packet.to_slice();
    let mut printer = Printer::new(700, 715, 0);
    write!(printer, "{:x}", v.as_ptr() as *const u8 as u32).unwrap();
    send_ethernet_packet(arp_packet.dst_hardware_addr, v, size_of::<Arp>(), ETHERNET_TYPE_ARP)
}

// pub fn receive_arp_reply(buf: DmaBox<[u8]>) -> Option<ArpTableEntry> {
//     let parsed_arp = Arp::parse_reply_buf(buf);
//     if let Some(arp) = parsed_arp {
//         unsafe {
//             ARP_TABLE.add(arp.src_protocol_addr, arp.src_hardware_addr);
//             ARP_TABLE.add(arp.dst_protocol_addr, arp.dst_hardware_addr);
//             return ARP_TABLE.get_entry_from_ip_addr(&arp.src_protocol_addr);
//         }
//     }
//     None
// }
pub fn receive_arp_reply(arp: Arp) -> Option<ArpTableEntry> {
    unsafe {
        ARP_TABLE.add(arp.src_protocol_addr, arp.src_hardware_addr);
        ARP_TABLE.add(arp.dst_protocol_addr, arp.dst_hardware_addr);
        return ARP_TABLE.get_entry_from_ip_addr(&arp.src_protocol_addr);
    }
}

pub fn get_my_hard_and_ip_addr() -> ([u8; 6], Option<[u8; 4]>) {
    let my_hardware_addr = get_mac_addr();
    let my_ip_addr = get_ip_addr_from_hardware_addr(&my_hardware_addr);
    (my_hardware_addr, my_ip_addr)
}

pub fn get_ip_addr_from_hardware_addr(hardware_addr: &[u8; 6]) -> Option<[u8; 4]> {
    unsafe {
        ARP_TABLE.get_ip_addr(hardware_addr)
    }
}

pub fn get_hardware_addr_from_ip_addr(ip_addr: &[u8; 4]) -> Option<[u8; 6]> {
    unsafe {
        ARP_TABLE.get_mac_addr(ip_addr)
    }
}
