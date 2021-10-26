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
        let mut slice: &[u8] = &self.dst_mac_addr[..];
        let mut slice: &[u8] = &[&slice[..], &self.src_mac_addr[..]].concat();
        let mut slice: &[u8] = &[&slice[..], &self.ether_type.to_be_bytes()].concat();
        let mut s: &[u8] = &[&slice[..], &self.payload[..]].concat();
        let mut printer = Printer::new(115, 560, 0);
        write!(printer, "{:?}", s.len()).unwrap();
        DmaBox::from(s)
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