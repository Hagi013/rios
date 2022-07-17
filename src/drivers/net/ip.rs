use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use alloc::string::String;

use super::e1000::{get_mac_addr, e1000_send_packet};
use super::net_util::{switch_endian16, any_as_u8_vec, push_to_vec};
use crate::memory::dma::DmaBox;
use crate::arp;
use crate::drivers::net::ethernet::{send_ethernet_packet, ETHERNET_TYPE_IP, DEFAULT_ETHERNET_ADDRESS, EthernetHdr};
use crate::memory::volatile::{read_mem, write_mem};

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;

use crate::arch::asmfunc::jmp_stop;

// pub const DEFAULT_MY_IP: [u8; 4] = [192, 168, 56, 103];
pub const DEFAULT_MY_IP: [u8; 4] = [0, 0, 0, 0];
pub static mut MY_IP: [u8; 4] = DEFAULT_MY_IP;

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

impl VersionIhl {
    fn get_u8(&self) -> u8 {
        match self {
            Self::Ip => 0x45,
            Self::St => 0x55,
            Self::Ipv6 => 0x65,
            Self::TpIx => 0x75,
            Self::Pip => 0x85,
            Self::Tuba => 0x95,
        }
    }

    fn parse(version_ihl: u8) -> VersionIhl {
        if version_ihl == (VersionIhl::Ip as u8) { VersionIhl::Ip }
        else if version_ihl == (VersionIhl::St as u8) { VersionIhl::St }
        else if version_ihl == (VersionIhl::Ipv6 as u8) { VersionIhl::Ipv6 }
        else if version_ihl == (VersionIhl::TpIx as u8) { VersionIhl::TpIx }
        else if version_ihl == (VersionIhl::Pip as u8) { VersionIhl::Pip }
        else { VersionIhl::Tuba }
    }
}


#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IpProtocol {
    Icmp = 1,
    Tcp = 6,
    Udp = 17,
}

impl IpProtocol {
    fn equals(&self, other: IpProtocol) -> bool {
        match self {
            &other => true,
            _ => false,
        }
    }

    fn parse(ip_protocol: u8) -> IpProtocol {
        if ip_protocol == IpProtocol::Icmp as u8 { IpProtocol::Icmp }
        else if ip_protocol == IpProtocol::Tcp as u8 { IpProtocol::Tcp }
        else { IpProtocol::Udp }
    }
}


#[repr(C)]
pub struct IpHdr {
    version_ihl: VersionIhl,
    dscp_ecn: u8, // https://ja.wikipedia.org/wiki/Type_of_Service
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
            dscp_ecn: 0x00,
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

    pub fn get_src_ip_addr(&self) -> [u8; 4] {
        [self.src_ip_addr[0], self.src_ip_addr[1], self.src_ip_addr[2], self.src_ip_addr[3]]
    }
    pub fn get_dst_ip_addr(&self) -> [u8; 4] {
        [self.dst_ip_addr[0], self.dst_ip_addr[1], self.dst_ip_addr[2], self.dst_ip_addr[3]]
    }

    pub fn get_type(&self) -> &IpProtocol { &self.protocol }
    pub fn is_tcp(&self) -> bool { self.protocol == IpProtocol::Tcp }
    pub fn is_udp(&self) -> bool { self.protocol == IpProtocol::Udp }
    pub fn is_icmp(&self) -> bool { self.protocol == IpProtocol::Icmp }

    pub fn parsed_from_buf(buf: DmaBox<[u8]>) -> IpHdr {
        IpHdr {
            version_ihl: VersionIhl::parse(buf[0]),
            dscp_ecn: buf[1],
            length: (buf[2] as u16) << 8 | buf[3] as u16,
            identifier: (buf[4] as u16) << 8 | buf[5] as u16,
            flag_flagment_offset: (buf[6] as u16) << 8 | buf[7] as u16,
            ttl: buf[8],
            protocol: IpProtocol::parse(buf[9]),
            checksum: (buf[10] as u16) << 8 | buf[11] as u16,
            src_ip_addr: [buf[12], buf[13], buf[14], buf[15]],
            dst_ip_addr: [buf[16], buf[17], buf[18], buf[19]],
            payload: DmaBox::from(&buf[20..]),
        }
    }

    fn to_slice(&self) -> DmaBox<[u8]> {
        let slice: &[u8] = &[&self.version_ihl.get_u8().to_be_bytes()[..]].concat();
        let slice: &[u8] = &[&slice[..], &self.dscp_ecn.to_be_bytes()].concat();
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
            (self.version_ihl as u8 as u16) << 8 | (self.dscp_ecn as u16),
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
        let sum: u32 = slice.iter().fold(0, |acc, &cur| { acc + (cur as u32) });
        let bottom = (0x0000ffff & sum) as u16;
        let upper = (sum >> 16) as u16;
        let checksum = (bottom as u32 + upper as u32) ^ 0x0000ffff;
        self.checksum = checksum as u16
    }

    pub fn calc_length(&mut self) {
        self.length = (20 + self.payload.len()) as u16;
    }

    pub fn set_payload(&mut self, buf: DmaBox<[u8]>) {
        self.payload = buf;
    }
}

pub fn send_ip_packet(protocol: IpProtocol, dst_ip_addr: &[u8; 4], payload: DmaBox<[u8]>) -> Result<(), String> {
    let (_, my_ip_addr) = match arp::get_my_hard_and_ip_addr() {
        (hardware_addr, Some(ip_addr)) => (hardware_addr, ip_addr),
        // (hardware_addr, None) => (hardware_addr, DEFAULT_MY_IP),
        (hardware_addr, None) => (hardware_addr, unsafe { MY_IP }),
        // _ => (DEFAULT_ETHERNET_ADDRESS, DEFAULT_MY_IP),
        _ => (DEFAULT_ETHERNET_ADDRESS, unsafe { MY_IP }),
    };
    let mut ip = IpHdr::new();
    write_mem!(
        &mut ip as *mut IpHdr,
        IpHdr {
            version_ihl: VersionIhl::Ip,
            dscp_ecn: 0x00,
            length: 0x00,
            identifier: 0x00,
            flag_flagment_offset: 0x00,
            ttl: 30,
            protocol,
            checksum: 0x00,
            src_ip_addr: my_ip_addr,
            dst_ip_addr: [dst_ip_addr[0], dst_ip_addr[1], dst_ip_addr[2], dst_ip_addr[3]],
            payload,
    });
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

pub fn reply_ip_packet(received_ethernet_header: EthernetHdr, payload: DmaBox<[u8]>) -> Result<(), String> {
    let received_ip_header = IpHdr::parsed_from_buf(received_ethernet_header.get_data());
    let (_, my_ip_addr) = match arp::get_my_hard_and_ip_addr() {
        (hardware_addr, Some(ip_addr)) => (hardware_addr, ip_addr),
        // (hardware_addr, None) => (hardware_addr, DEFAULT_MY_IP),
        // _ => (DEFAULT_ETHERNET_ADDRESS, DEFAULT_MY_IP),
        (hardware_addr, None) => (hardware_addr, unsafe { MY_IP }),
        _ => (DEFAULT_ETHERNET_ADDRESS, unsafe { MY_IP }),
    };
    if my_ip_addr != received_ip_header.dst_ip_addr { return Ok(()); }

    let mut reply_ip_header = IpHdr::new();
    write_mem!(
        &mut reply_ip_header as *mut IpHdr,
        IpHdr {
            version_ihl: received_ip_header.version_ihl,
            dscp_ecn: received_ip_header.dscp_ecn,
            length: 0x00,
            identifier: received_ip_header.identifier,
            flag_flagment_offset: received_ip_header.flag_flagment_offset,
            ttl: received_ip_header.ttl - 1,
            protocol: received_ip_header.protocol,
            checksum: 0x00,
            src_ip_addr: my_ip_addr,
            dst_ip_addr: [received_ip_header.src_ip_addr[0], received_ip_header.src_ip_addr[1], received_ip_header.src_ip_addr[2], received_ip_header.src_ip_addr[3]],
            payload,
        }
    );
    reply_ip_header.calc_length();
    reply_ip_header.calc_checksum();

    let dst_mac_addr = received_ethernet_header.get_src_mac_addr();
    let dst_mac_addr = [dst_mac_addr[0], dst_mac_addr[1], dst_mac_addr[2], dst_mac_addr[3], dst_mac_addr[4], dst_mac_addr[5]];
    let data = reply_ip_header.to_slice();
    let len = data.len();
    send_ethernet_packet(dst_mac_addr, data, len, ETHERNET_TYPE_IP)
}

// ToDo integrate with reply_ip_packet
pub fn reply_ip_packet_with_no_check_from_ip(received_ethernet_header: EthernetHdr, payload: DmaBox<[u8]>) -> Result<(), String> {
    let received_ip_header = IpHdr::parsed_from_buf(received_ethernet_header.get_data());
    let (_, my_ip_addr) = match arp::get_my_hard_and_ip_addr() {
        (hardware_addr, Some(ip_addr)) => (hardware_addr, ip_addr),
        (hardware_addr, None) => (hardware_addr, unsafe { MY_IP }),
        _ => (DEFAULT_ETHERNET_ADDRESS, unsafe { MY_IP }),
    };

    let mut reply_ip_header = IpHdr::new();
    write_mem!(
        &mut reply_ip_header as *mut IpHdr,
        IpHdr {
            version_ihl: received_ip_header.version_ihl,
            dscp_ecn: received_ip_header.dscp_ecn,
            length: 0x00,
            identifier: received_ip_header.identifier,
            flag_flagment_offset: received_ip_header.flag_flagment_offset,
            ttl: received_ip_header.ttl - 1,
            protocol: received_ip_header.protocol,
            checksum: 0x00,
            src_ip_addr: my_ip_addr,
            dst_ip_addr: [received_ip_header.src_ip_addr[0], received_ip_header.src_ip_addr[1], received_ip_header.src_ip_addr[2], received_ip_header.src_ip_addr[3]],
            payload,
        }
    );
    reply_ip_header.calc_length();
    reply_ip_header.calc_checksum();

    let dst_mac_addr = received_ethernet_header.get_src_mac_addr();
    let dst_mac_addr = [dst_mac_addr[0], dst_mac_addr[1], dst_mac_addr[2], dst_mac_addr[3], dst_mac_addr[4], dst_mac_addr[5]];
    let data = reply_ip_header.to_slice();
    let len = data.len();
    send_ethernet_packet(dst_mac_addr, data, len, ETHERNET_TYPE_IP)
}

pub fn get_my_ip() -> [u8; 4] {
    unsafe {
        [MY_IP[0], MY_IP[1], MY_IP[2], MY_IP[3]]
    }
}

pub fn set_my_ip(ip: &[u8; 4]) {
    unsafe {
        write_mem!(
            &mut MY_IP,
            [ip[0], ip[1], ip[2], ip[3]]
        );
        arp::set_my_ip(ip);
    }
}
