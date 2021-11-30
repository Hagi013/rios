use alloc::string::String;
use alloc::vec::Vec;
use crate::memory::dma::DmaBox;
use crate::memory::volatile::{write_mem};

use super::ip::{send_ip_packet, IpProtocol};

#[repr(u8)]
#[derive(Copy, Clone)]
enum IcmpEchoType {
    EchoReplyMessage = 0,
    DestinationUnreachableMessage =  3,
    SourceQuenchMessage = 4,
    RedirectMessage = 5,
    EchoMessage = 8,
    RouterAdvertisementMessage = 9,
    RouterSolicitationMessage = 10,
    TimeExceededMessage = 11,
    ParameterProblemMessage = 12,
    TimestampMessage = 13,
    TimestampReplyMessage = 14,
    InformationRequestMessage = 15,
    InformationReplyMessage = 16,
    AddressMaskRequestMessage = 17,
    AddressMaskReplyMessage = 18,
    Traceroute = 30,
}

#[repr(C)]
struct IcmpHeader {
    icmp_type: IcmpEchoType,
    icmp_code: u8,
    checksum: u16,
}

#[repr(C)]
struct EchoMessage {
    icmp_header: IcmpHeader,
    identifier: u16,
    sequence_num: u16,
    data: DmaBox<[u8]>,
}

impl EchoMessage {
    pub fn new() -> Self {
        let s: &[u8] = &[];
        EchoMessage {
            icmp_header: IcmpHeader {
                icmp_type: IcmpEchoType::EchoMessage,
                icmp_code: 0x0,
                checksum: 0x0,
            },
            identifier: 0x0,
            sequence_num: 0x0,
            data: DmaBox::from(s),
        }
    }

    fn to_slice(&self) -> DmaBox<[u8]> {
        let mut slice: &[u8] = &[
            self.icmp_header.icmp_type as u8,
            self.icmp_header.icmp_code,
        ];
        let slice = &[&slice[..], &self.icmp_header.checksum.to_be_bytes()].concat();
        let slice = &[&slice[..], &self.identifier.to_be_bytes()].concat();
        let slice = &[&slice[..], &self.sequence_num.to_be_bytes()].concat();
        let s: &[u8]  = &[&slice[..], &self.data[..]].concat();
        DmaBox::from(s)
    }

    // チェックサムは、ICMPタイプから始まるICMPメッセージの1の補数の合計について、16ビットの1の補数を取ったものである。
    // チェックサム計算中は、チェックサムフィールドを0にする。このチェックサムは将来置き換えられる可能性がある。
    fn calc_checksum(&mut self) {
        let slice: &[u16] = &[
            (self.icmp_header.icmp_type as u8 as u16) << 8 | self.icmp_header.icmp_code as u16,
            self.icmp_header.checksum,
            self.identifier,
            self.sequence_num,
        ];
        let mut data_u16_list: Vec<u16> = vec![];
        for idx in 0..self.data.len() / 2 + 1 {
            if idx * 2 >= self.data.len() {
                continue;
            }
            if idx * 2 + 1 >= self.data.len() {
                // data = &[&data[..], &[(self.data[idx * 2] as u16) << 8]].concat();
                data_u16_list.push((self.data[idx * 2] as u16) << 8);
                continue;
            }
            // data = &[&data[..], &[((self.data[idx * 2] as u16) << 8 | self.data[idx * 2 + 1] as u16)]].concat();
            data_u16_list.push(((self.data[idx * 2] as u16) << 8 | self.data[idx * 2 + 1] as u16));
        }
        let slice: &[u16] = &[&slice[..], data_u16_list.as_slice()].concat();
        let sum: u32 = slice.iter().fold(0, |acc, &x| { acc + (x as u32) });
        let upper: u16 = (sum >> 16) as u16;
        let bottom: u16 = (sum & 0x0000ffff) as u16;
        self.icmp_header.checksum = (bottom + upper) ^ 0xffff;
    }
}

pub fn send_icmp(dst_ip_addr: &[u8; 4]) -> Result<(), String> {
    let mut icmp = EchoMessage::new();
    write_mem!(&mut icmp as *mut EchoMessage, EchoMessage::new());
    icmp.calc_checksum();
    send_ip_packet(IpProtocol::Icmp, dst_ip_addr, icmp.to_slice())
}

