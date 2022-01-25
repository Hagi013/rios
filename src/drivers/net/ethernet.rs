use alloc::vec::Vec;
use alloc::string::String;

use super::e1000::{get_mac_addr, e1000_send_packet};
use super::net_util::{switch_endian16, any_as_u8_vec, push_to_vec};
use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;
use crate::memory::dma::DmaBox;

pub const ETHERNET_TYPE_ARP: u16 = 0x0806;
pub const ETHERNET_TYPE_IP: u16 = 0x0800;

pub const HARDWARE_TYPE_ETHERNET: u16 = 0x01;

pub const DEFAULT_ETHERNET_ADDRESS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

#[repr(C)]
pub struct EthernetHdr {
    dst_mac_addr: [u8; 6],
    src_mac_addr: [u8; 6],
    ether_type: u16,
    payload: DmaBox<[u8]>,
}

impl EthernetHdr {
    fn to_slice(&self) -> DmaBox<[u8]> {
        let slice: &[u8] = &[
            &self.dst_mac_addr[..],
            // 何故か `&self.src_mac_addr[..]` だと先頭に `0xffff` が付加されるのでこれで回避する・・・
            // ToDo 普通に `&self.src_mac_addr[..]` だとだめな理由を調べる(networkがarpのみの時代は問題なく動いていたはず・・)
            self.src_mac_addr.clone().as_ref()[..].as_ref(),
            &self.ether_type.to_be_bytes()[..],
            &self.payload[..]
        ].concat();
        DmaBox::from(slice)
    }

    pub fn get_src_mac_addr(&self) -> &[u8; 6] {
        &self.src_mac_addr
    }

    pub fn is_arp_type(&self) -> bool {
        self.ether_type == ETHERNET_TYPE_ARP
    }

    pub fn is_ip_type(&self) -> bool {
        self.ether_type == ETHERNET_TYPE_IP
    }

    pub fn get_type(&self) -> u16 {
        self.ether_type
    }

    pub fn get_data(&self) -> DmaBox<[u8]> {
        self.payload.clone()
    }

    pub fn parse_from_frame(frame: Vec<u8>) -> Option<Self> {
        let ether_type = (frame[12] as u16) << 8 | frame[13] as u16;
        let mut printer = Printer::new(30, 75, 0);
        write!(printer, "{:x}", frame.len() as u32).unwrap();
        let mut printer = Printer::new(30, 90, 0);
        write!(printer, "{:x}", frame[12]).unwrap();
        let mut printer = Printer::new(50, 90, 0);
        write!(printer, "{:x}", frame[13]).unwrap();
        let mut printer = Printer::new(30, 105, 0);
        write!(printer, "{:x}", ether_type as u32).unwrap();

        if ether_type == ETHERNET_TYPE_ARP || ether_type == ETHERNET_TYPE_IP {
            Some(Self {
                dst_mac_addr: [frame[0], frame[1], frame[2], frame[3], frame[4], frame[5]],
                src_mac_addr: [frame[6], frame[7], frame[8], frame[9], frame[10], frame[11]],
                ether_type,
                payload: DmaBox::from(&frame[14..]),
            })
        } else {
            None
        }
    }

    pub fn get_dst_mac_addr_from_buf(buf: &DmaBox<[u8]>) -> [u8; 6] {
        [buf[0], buf[1], buf[2], buf[3], buf[4], buf[5]]
    }
}


pub fn send_ethernet_packet(dst_mac_addr: [u8; 6], data: DmaBox<[u8]>, len: usize, protocol: u16) -> Result<(), String> {
    let src_mac_addr = get_mac_addr();
    let ethernet_hdr = EthernetHdr {
        dst_mac_addr,
        src_mac_addr,
        // ether_type: switch_endian16(protocol),
        ether_type: protocol,
        payload: data,
    };
    let v = ethernet_hdr.to_slice();
    let mut printer = Printer::new(800, 605, 0);
    write!(printer, "{:x}", v.as_ptr() as *const u8 as u32).unwrap();
    e1000_send_packet(v)
}