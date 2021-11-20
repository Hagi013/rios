use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use alloc::string::String;

use super::e1000::{get_mac_addr, e1000_send_packet};
use super::net_util::{switch_endian16, any_as_u8_vec, push_to_vec};
use crate::memory::dma::DmaBox;
use crate::arp;

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;
use crate::drivers::net::ethernet::{send_ethernet_packet, ETHERNET_TYPE_IP, DEFAULT_ETHERNET_ADDRESS};

pub const DEFAULT_MY_IP: [u8; 4] = [192, 168, 56, 101];

#[repr(u8)]
#[derive(Clone, Copy)]
enum VersionIhl {
    Ip = 0x45, // Internet Protocol
    St = 0x55, // ST Datagram Mode
    Ipv6 = 0x65, // Internet Protocol version 6
    TpIx = 0x75, // TP/IX: The Next Internet
    Pip = 0x85, // The P Internet Protocol
    Tuba = 0x95, // TUBA
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum IpProtocol {
    Icmp = 0x01,
    Tcp = 0x06,
    Udp = 0x07,
}


#[repr(C)]
pub struct IpHdr {
    version_ihl: VersionIhl,
    escp_ecn: u8,
    length: u16,
    identifier: u16,
    flag_flagment_offset: u16,
    ttl: u8,
    protocol: IpProtocol,
    checksum: u16,
    src_ip_addr: [u8; 4],
    dst_ip_addr: [u8; 4],
    payload: DmaBox<[u8]>,
}

impl IpHdr {
    fn new() -> IpHdr {
        let empty_slice: &[u8] = &[];
        IpHdr {
            version_ihl: VersionIhl::Ip,
            escp_ecn: 0x00,
            length: 0x00,
            identifier: 0x00,
            flag_flagment_offset: 0x00,
            ttl: 30,
            protocol: IpProtocol::Tcp,
            checksum: 0x00,
            src_ip_addr: [0x00, 0x00, 0x00, 0x00],
            dst_ip_addr: [0x00, 0x00, 0x00, 0x00],
            payload: DmaBox::from(empty_slice),
        }
    }
}

impl IpHdr {
    fn to_slice(&self) -> DmaBox<[u8]> {
        let slice: &[u8] = &[(self.version_ihl as u8).to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.escp_ecn.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.length.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.identifier.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.flag_flagment_offset.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.ttl.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &(self.protocol as u8).to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.checksum.to_be_bytes()].concat();
        let slice: &[u8] = &[&slice[..], &self.src_ip_addr[..]].concat();
        let slice: &[u8] = &[&slice[..], &self.dst_ip_addr[..]].concat();
        let s: &[u8] = &[&slice[..], &self.payload[..]].concat();
        DmaBox::from(s)
    }

    pub fn get_offset(&self) -> bool {
        (self.flag_flagment_offset) & 0b1111111111111000 == 0b010
    }

    pub fn check_fragment_on(&self) -> bool {
        0b010 & (self.flag_flagment_offset) == 0b010
    }

    pub fn fragment_on(&mut self) {
        self.flag_flagment_offset = self.flag_flagment_offset | 0b0000000000000010
    }

    pub fn fragment_off(&mut self) {
        self.flag_flagment_offset = self.flag_flagment_offset & 0b1111111111111101
    }

    pub fn check_last_packet(&self) -> bool {
        0b100 & (self.flag_flagment_offset) == 0b100
    }

    pub fn last_packet_on(&mut self) {
        self.flag_flagment_offset = self.flag_flagment_offset | 0b0000000000000100
    }

    pub fn get_data(&self) -> DmaBox<[u8]> {
        self.payload.clone()
    }

    pub fn calc_checksum(&mut self) {
        // u8をu16に合わせる際に小さいアドレスの方を8ビット右シフトしているのは、それでビッグエンディアンになるから
        let slice: &[u16] = &[
            (self.version_ihl as u8 as u16) << 8 | (self.escp_ecn as u16),
            self.length,
            self.identifier,
            self.flag_flagment_offset,
            (self.ttl as u16) << 8 | (self.protocol as u16),
            self.checksum,
            (self.src_ip_addr[0] as u16) << 8 | (self.src_ip_addr[1] as u16),
            (self.src_ip_addr[2] as u16) << 8 | (self.src_ip_addr[3] as u16),
            (self.dst_ip_addr[0] as u16) << 8 | (self.dst_ip_addr[1] as u16),
            (self.dst_ip_addr[2] as u16) << 8 | (self.dst_ip_addr[3] as u16),
        ];
        let mut payload_u16: Vec<u16> = vec![];
        for idx in 0..self.payload.len() / 2 + 1 {
            if idx * 2 >= self.payload.len() { continue; }
            if idx * 2 + 1 >= self.payload.len() {
                // slice = &[slice[..].as_ref(), &[(self.payload[idx * 2] as u16) << 8]].concat();
                payload_u16.push((self.payload[idx * 2] as u16) << 8);
                continue;
            }
            // slice = &[&slice[..], &[(self.payload[idx * 2] as u16) << 8 | (self.payload[idx * 2 + 1] as u16)]].concat();
            payload_u16.push((self.payload[idx * 2] as u16) << 8 | (self.payload[idx * 2 + 1] as u16));
        }
        let slice: &[u16] = &[&slice[..], payload_u16.as_slice()].concat();
        let sum: u32 = slice.iter().fold(0, |acc, &cur| { acc + (cur as u32) });
        self.checksum = ((0x0000ffff & sum) as u16 + (sum >> 8) as u16) as u16
    }

    pub fn calc_length(&mut self) -> u16 {
        self.length = (20 + self.payload.len()) as u16;
        self.length
    }
}

pub fn send_ip_packet(protocol: IpProtocol, dst_ip_addr: &[u8; 4], payload: DmaBox<[u8]>) -> Result<(), String> {
    let (_, my_ip_addr) = match arp::get_my_hard_and_ip_addr() {
        (hardware_addr, Some(ip_addr)) => (hardware_addr, ip_addr),
        (hardware_addr, None) => (hardware_addr, DEFAULT_MY_IP),
        _ => (DEFAULT_ETHERNET_ADDRESS, DEFAULT_MY_IP),
    };
    let mut ip = IpHdr {
        version_ihl: VersionIhl::Ip,
        escp_ecn: 0x00,
        length: 0x00,
        identifier: 0x00,
        flag_flagment_offset: 0x00,
        ttl: 30,
        protocol,
        checksum: 0x00,
        src_ip_addr: my_ip_addr,
        dst_ip_addr: [dst_ip_addr[0], dst_ip_addr[1], dst_ip_addr[2], dst_ip_addr[3]],
        payload,
    };
    ip.calc_length();
    ip.calc_checksum();

    // dst_mac_addrをdst_ipからARP_TABLEから取得 or ARPで取得する
    let dst_mac_addr = match protocol {
        IpProtocol::Icmp => [0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        _ => {
            match arp::get_hardware_addr_from_ip_addr(dst_ip_addr) {
                Some(addr) => addr,
                None => [0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            }
        }
    };
    let data = ip.to_slice();
    let len = data.len();
    send_ethernet_packet(dst_mac_addr, data, len, ETHERNET_TYPE_IP)
}