use core::mem::size_of;
use alloc::vec::Vec;
use alloc::string::String;
use crate::drivers::net::ip::IpProtocol::Udp;
use crate::drivers::net::ip::send_ip_packet;
use crate::memory::volatile::{read_mem, write_mem};
use crate::memory::dma::DmaBox;

use super::ethernet::EthernetHdr;
use super::ip::{reply_ip_packet, reply_ip_packet_with_no_check_from_ip, get_my_ip, IpHdr, IpProtocol};
use super::dhcp::reply_dhcp;

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;

const UDP_PROTOCOL_NUMBER: u8 = 17;
const UDP_PADDING: u8 = 0x0;

#[repr(C)]
struct PseudoUpd<'a> {
    source_ip: [u8; 4],
    dest_ip: [u8; 4],
    padding: u8,
    protocol: u8,
    length: u16,
    real_udp_header: &'a mut UdpHdr,
}

impl <'a>PseudoUpd<'a> {
    fn new(source_ip: [u8; 4], dest_ip: [u8; 4], udp_header: &mut UdpHdr) -> PseudoUpd {
        PseudoUpd {
            source_ip,
            dest_ip,
            padding: UDP_PADDING,
            protocol: UDP_PROTOCOL_NUMBER,
            length: udp_header.length,
            real_udp_header: udp_header,
        }
    }

    fn calc_checksum(&mut self) {
        let slice: &[u16] = &[
            (self.source_ip[0] as u16) << 8 | self.source_ip[1] as u16,
            (self.source_ip[2] as u16) << 8 | self.source_ip[3] as u16,
            (self.dest_ip[0] as u16) << 8 | self.dest_ip[1] as u16,
            (self.dest_ip[2] as u16) << 8 | self.dest_ip[3] as u16,
            (self.padding as u16) << 8 | self.protocol as u16,
            self.length,
            self.real_udp_header.source_port,
            self.real_udp_header.dest_port,
            self.real_udp_header.length,
            self.real_udp_header.checksum,
        ];
        let mut data_u16_list: Vec<u16> = vec![];
        for idx in 0..self.real_udp_header.data.len() / 2 + 1 {
            if idx * 2 >= self.real_udp_header.data.len() {
                continue;
            }
            if idx * 2 + 1 >= self.real_udp_header.data.len() {
                data_u16_list.push((self.real_udp_header.data[idx * 2] as u16) << 8);
                continue;
            }
            data_u16_list.push((self.real_udp_header.data[idx * 2] as u16) << 8 | (self.real_udp_header.data[idx * 2 + 1] as u16));
        }
        let slice: &[u16] = &[&slice[..], data_u16_list.as_slice()].concat();
        let sum: u32 = slice.iter().fold(0, |acc, &x| { acc + (x as u32) });
        let upper: u16 = (sum >> 16) as u16;
        let bottom: u16 = (sum & 0x0000ffff) as u16;
        let checksum = (bottom as u32 + upper as u32) ^ 0x0000ffff;
        self.real_udp_header.checksum = checksum as u16;
    }
}

#[repr(C)]
struct UdpHdr {
    source_port: u16,
    dest_port: u16,
    length: u16,
    checksum: u16,
    data: DmaBox<[u8]>,
}

impl UdpHdr {
    pub fn new() -> Self {
        let s: &[u8] = &[];
        UdpHdr {
            source_port: 0x00,
            dest_port: 0x00,
            length: 0x00,
            checksum: 0x00,
            data: DmaBox::from(s),
        }
    }

    pub fn to_slice(&self) -> DmaBox<[u8]> {
        let slice: &[u8] = &[
            &self.source_port.to_be_bytes()[..],
            &self.dest_port.to_be_bytes()[..],
            &self.length.to_be_bytes()[..],
            &self.checksum.to_be_bytes()[..],
            &self.data[..],
        ].concat();
        DmaBox::from(slice)
    }

    pub fn calc_length(&mut self) {
        let size = size_of::<u16>() + size_of::<u16>() + size_of::<u16>() + size_of::<u16>() + self.data.len();
        self.length = size as u16;
    }

    pub fn cacl_checksum(&mut self, source_ip: [u8; 4], dest_ip: [u8; 4]) {
        let mut pseudo_udp = PseudoUpd::new(source_ip, dest_ip, self);
        pseudo_udp.calc_checksum();
    }

    pub fn get_data(&self) -> DmaBox<[u8]> {
        self.data.clone()
    }

    pub fn parse_from_buf(buf: DmaBox<[u8]>) -> UdpHdr {
        UdpHdr {
            source_port: (buf[0] as u16) << 8 | buf[1] as u16,
            dest_port: (buf[2] as u16) << 8 | buf[3] as u16,
            length: (buf[4] as u16) << 8 | buf[5] as u16,
            checksum: (buf[6] as u16) << 8 | buf[7] as u16,
            data: DmaBox::from(&buf[8..]),
        }
    }
}

pub fn send_udp(source_port: u16, dest_port: u16, dest_ip_addr: [u8; 4], data: DmaBox<[u8]>) -> Result<(), String> {
    let source_ip_addr: [u8; 4] = get_my_ip();
    let mut udp_header = UdpHdr {
        source_port,
        dest_port,
        checksum: 0x00,
        length: 0x00,
        data,
    };
    udp_header.calc_length();
    udp_header.cacl_checksum(source_ip_addr, dest_ip_addr);
    send_ip_packet(Udp, &dest_ip_addr, udp_header.to_slice())
}

pub fn receive_udp(received_ethernet_header: EthernetHdr) -> Result<(), String> {
    let received_ip_header = IpHdr::parsed_from_buf(received_ethernet_header.get_data());
    let received_udp_header = UdpHdr::parse_from_buf(received_ip_header.get_data());
    let mut printer = Printer::new(600, 375, 0);
    write!(printer, "{:?}", received_udp_header.dest_port).unwrap();
    if received_udp_header.source_port == 67 && received_udp_header.dest_port == 68 {
        let mut received_payload = received_udp_header.get_data();
        return reply_dhcp(received_ethernet_header, received_payload)
    }
    Ok(())
}

pub fn reply_udp(received_ethernet_header: EthernetHdr, upper_layer_payload: DmaBox<[u8]>, from_ip_check_flag: bool) -> Result<(), String> {
    let received_ip_header = IpHdr::parsed_from_buf(received_ethernet_header.get_data());
    let received_udp_header = UdpHdr::parse_from_buf(received_ip_header.get_data());
    let mut reply_udp_header = UdpHdr::new();
    write_mem!(
        &mut reply_udp_header,
        UdpHdr {
            source_port: received_udp_header.dest_port,
            dest_port: received_udp_header.source_port,
            length: 0x00,
            checksum: 0x00,
            data: upper_layer_payload,
        }
    );
    reply_udp_header.calc_length();
    // FIXME 本当は計算したい
    // reply_udp_header.cacl_checksum(received_ip_header.get_dst_ip_addr(), received_ip_header.get_src_ip_addr());
    if from_ip_check_flag {
        return reply_ip_packet(received_ethernet_header, reply_udp_header.to_slice());
    } else {
        return reply_ip_packet_with_no_check_from_ip(received_ethernet_header, reply_udp_header.to_slice());
    }
}
