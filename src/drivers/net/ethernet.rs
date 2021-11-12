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

#[repr(C)]
pub struct EthernetHdr {
    dst_mac_addr: [u8; 6],
    src_mac_addr: [u8; 6],
    ether_type: u16,
    payload: DmaBox<[u8]>,
}

impl EthernetHdr {
    fn to_slice(&self) -> DmaBox<[u8]> {
        let slice: &[u8] = &self.dst_mac_addr[..];
        let slice: &[u8] = &[&slice[..], &self.src_mac_addr[..]].concat();
        let slice: &[u8] = &[&slice[..], &self.ether_type.to_be_bytes()].concat();
        let s: &[u8] = &[&slice[..], &self.payload[..]].concat();
        let mut printer = Printer::new(115, 560, 0);
        write!(printer, "{:?}", s.len()).unwrap();
        DmaBox::from(s)
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

        if ether_type == ETHERNET_TYPE_ARP {
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
    // let mut printer = Printer::new(800, 590, 0);
    // write!(printer, "{:x}", v.get(15).unwrap() as *const u8 as u32).unwrap();
    let mut printer = Printer::new(800, 605, 0);
    write!(printer, "{:x}", v.as_ptr() as *const u8 as u32).unwrap();
    e1000_send_packet(v)
}