use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::net::ip::IpHdr;
use crate::memory::dma::DmaBox;
use crate::memory::volatile::{write_mem};

use super::ip::{send_ip_packet, reply_ip_packet, IpProtocol};

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;
use crate::EthernetHdr;

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

#[repr(C)] // https://ja.wikipedia.org/wiki/Internet_Control_Message_Protocol
pub struct IcmpHeader {
    icmp_type: IcmpEchoType,
    icmp_code: u8,
    checksum: u16,
}

impl IcmpHeader {
    fn check_type_from_payload(buf: DmaBox<[u8]>) -> IcmpEchoType {
        Self::check_type(buf[0])
    }

    fn check_type(b: u8) -> IcmpEchoType {
        if b == IcmpEchoType::EchoReplyMessage as u8 { IcmpEchoType::EchoReplyMessage }
        else if b == IcmpEchoType::DestinationUnreachableMessage as u8 { IcmpEchoType::DestinationUnreachableMessage }
        else if b == IcmpEchoType::SourceQuenchMessage as u8 { IcmpEchoType::SourceQuenchMessage }
        else if b == IcmpEchoType::RedirectMessage as u8 { IcmpEchoType::RedirectMessage }
        else if b == IcmpEchoType::EchoMessage as u8 { IcmpEchoType::EchoMessage }
        else if b == IcmpEchoType::RouterAdvertisementMessage as u8 { IcmpEchoType::RouterAdvertisementMessage }
        else if b == IcmpEchoType::RouterSolicitationMessage as u8 { IcmpEchoType::RouterSolicitationMessage }
        else if b == IcmpEchoType::TimeExceededMessage as u8 { IcmpEchoType::TimeExceededMessage }
        else if b == IcmpEchoType::ParameterProblemMessage as u8 { IcmpEchoType::ParameterProblemMessage }
        else if b == IcmpEchoType::TimestampMessage as u8 { IcmpEchoType::TimestampMessage }
        else if b == IcmpEchoType::TimestampReplyMessage as u8 { IcmpEchoType::TimestampReplyMessage }
        else if b == IcmpEchoType::InformationRequestMessage as u8 { IcmpEchoType::InformationRequestMessage }
        else if b == IcmpEchoType::InformationReplyMessage as u8 { IcmpEchoType::InformationReplyMessage }
        else if b == IcmpEchoType::AddressMaskRequestMessage as u8 { IcmpEchoType::AddressMaskRequestMessage }
        else if b == IcmpEchoType::AddressMaskReplyMessage as u8 { IcmpEchoType::AddressMaskReplyMessage }
        else { IcmpEchoType::Traceroute }
    }

    fn parse_from_buf(buf: &[u8]) -> IcmpHeader {
        IcmpHeader {
            icmp_type: Self::check_type(buf[0]),
            icmp_code: buf[1],
            checksum: (buf[2] as u16) << 8 | (buf[3] as u16),
        }
    }
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

    pub fn parse_from_buf(buf: DmaBox<[u8]>) -> EchoMessage {
        EchoMessage {
            icmp_header: IcmpHeader::parse_from_buf(&buf[0..=3]),
            identifier: (buf[4] as u16) << 8 | (buf[5] as u16),
            sequence_num: (buf[6] as u16) << 8 | (buf[7] as u16),
            data: DmaBox::from(&buf[8..&buf.len() - 4]),
        }
    }
}


pub fn send_icmp(dst_ip_addr: &[u8; 4]) -> Result<(), String> {
    let mut icmp = EchoMessage::new();
    write_mem!(&mut icmp as *mut EchoMessage, EchoMessage::new());
    icmp.calc_checksum();
    send_ip_packet(IpProtocol::Icmp, dst_ip_addr, icmp.to_slice())
}

pub fn receive_icmp(parsed_ethernet_header: EthernetHdr) -> Result<(), String> {
    let parsed_ip_header = IpHdr::parsed_from_buf(parsed_ethernet_header.get_data());
    match IcmpHeader::check_type_from_payload(parsed_ip_header.get_data()) {
        IcmpEchoType::EchoMessage => {
            let echo_message = EchoMessage::parse_from_buf(parsed_ip_header.get_data());
            let mut reply_message = EchoMessage {
                icmp_header: IcmpHeader {
                    icmp_type: IcmpEchoType::EchoReplyMessage,
                    icmp_code: 0x00,
                    checksum: 0x00,
                },
                identifier: echo_message.identifier,
                sequence_num: echo_message.sequence_num,
                data: echo_message.data,
            };
            reply_message.calc_checksum();
            let payload = reply_message.to_slice();
            reply_ip_packet(parsed_ethernet_header, payload);
            let mut printer = Printer::new(600, 590, 0);
            write!(printer, "{:?}", "reply icmp").unwrap();
        },
        IcmpEchoType::EchoReplyMessage => {
            let replied_message = EchoMessage::parse_from_buf(parsed_ip_header.get_data());
            // identifierとsequence_numberがこちらから送ったものと一致しているかを確認
            // 再度送るのであればidentifierは同じ、sequence_numberはインクリメントする
            let mut reply_message = EchoMessage {
                icmp_header: IcmpHeader {
                    icmp_type: IcmpEchoType::EchoMessage,
                    icmp_code: 0x00,
                    checksum: 0x00,
                },
                identifier: replied_message.identifier,
                sequence_num: replied_message.sequence_num + 1,
                data: replied_message.data,
            };
            reply_message.calc_checksum();
            let payload = reply_message.to_slice();
            reply_ip_packet(parsed_ethernet_header, payload);
            let mut printer = Printer::new(600, 665, 0);
            write!(printer, "{:?}", "received EchoReplyMessage, and replied").unwrap();
        },
        _ => {}
    }
    Ok(())
}

