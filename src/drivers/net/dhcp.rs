use alloc::string::String;
use core::convert::TryInto;

use crate::memory::dma::DmaBox;
use crate::e1000::get_mac_addr;
use crate::ethernet::EthernetHdr;
use crate::ip::set_my_ip;
use crate::udp::{reply_udp, send_udp};
use crate::get_uptime;

use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;

const MAGIC_COOKIE: u32 = 0x63825363;

#[repr(C)]
struct DhcpOption {
    option_type: DhcpOptionType,
    length: u8,
    value: DmaBox<[u8]>,
}

impl DhcpOption {
    fn len(&self) -> usize {
        1 + 1 + self.length as usize
    }

    fn to_slice(&self) -> DmaBox<[u8]> {
        let s: &[u8] = &[
            &(self.option_type as u8).to_be_bytes(),
            &self.length.to_be_bytes(),
            &self.value[..],
        ].concat();
        DmaBox::from(s)
    }
}

// ref) https://ja.wikipedia.org/wiki/Dynamic_Host_Configuration_Protocol
// ref2) https://support.huawei.com/enterprise/en/doc/EDOC1100058932/25cd2dfc/dhcp-messages
#[repr(u8)]
#[derive(Copy, Clone)]
enum DhcpOptionType {
    Pad = 0,
    SubnetMask = 1,
    TimeOffset = 2,
    Router = 3,
    TimeServer = 4,
    NameServer = 5,
    LogServer = 7,
    CookieServer = 8,
    LprServer = 9,
    ImpressServer = 10,
    ResourceLocationServer = 11,
    HostName = 12,
    BootFileSize = 13,
    MeritDumpFile = 14,
    DomainName = 15,
    SwapServer = 16,
    RootPath = 17,
    ExtensionsPath = 18,
    IPForwardingEnableDisable = 19,
    NonLocalSourceRoutingEnableDisable = 20,
    PolicyFilter = 21,
    MaximumDatagramReassemblySize = 22,
    DefaultIpTimeToLive = 23,
    PathMtuAgingTimeout	= 24,
    PathMtuPlateauTab = 25,
    InterfaceMtu = 26,
    AllSubnetsAreLocal = 27,
    BroadcastAddress = 28,
    PerformMaskDiscovery = 29,
    MaskSupplier = 30,
    PerformRouterDiscovery = 31,
    RouterSolicitationAddress = 32,
    StaticRoute = 33,
    TrailerEncapsulationOption = 34,
    ARPCacheTimeout = 35,
    EthernetEncapsulation = 36,
    TCPDefaultTtl = 37,
    TCPKeepaliveInterval = 38,
    TCPKeepaliveGarbage = 39,
    NetworkInformationServiceDomain = 40,
    NetworkInformationServers = 41,
    NetworkTimeProtocolNtpServers = 42,
    VendorSpecificInformation = 43,
    NetBIOSOverTcpIpNameServer = 44,
    NetBIOSOverTcpIpDatagramDistributionServer = 45,
    NetBIOSOverTcpIpNodeType = 46,
    NetBIOSOverTcpIpScope = 47,
    XWindowSystemFontServer = 48,
    XWindowSystemDisplayManager = 49,
    RequestedIpAddress = 50,
    IpAddressLeaseTime = 51,
    OptionOverload = 52,
    DhcpMessageType = 53,
    ServerIdentifier = 54,
    ParameterRequestList = 55,
    Message = 56,
    MaximumDhcpMessageSize = 57,
    RenewalT1TimeValue = 58,
    RebindingT2TimeValue = 59,
    VendorClassIdentifier = 60,
    ClientIdentifier = 61,
    NetworkInformationServicePlusDomain = 64,
    NetworkInformationServicePlusServers = 65,
    TftpServerName = 66,
    BootfileName = 67,
    MobileIpHomeAgent = 68,
    SmtpServer = 69,
    Pop3Server = 70,
    NntpServer = 71,
    DefaultWwwServer = 72,
    DefaultFingerProtocolServer = 73,
    DefaultIrcServer = 74,
    StreetTalkServer = 75,
    StreetTalkDirectoryAssistanceServer = 76,
    RelayAgentInformation = 82,
    NovellDirectoryServiceServers = 85,
    NdsTreeName = 86,
    NdsContext = 87,
    TimeZonePosixStyle = 100,
    TimeZoneTzDatabaseStyle = 101,
    DomainSearch = 119,
    ClasslessStaticRoute = 121,
    End = 255,
    None = 254,
}

impl DhcpOptionType {
    fn parse(dhcp_option_type: u8) -> DhcpOptionType {
        match dhcp_option_type {
            0 => Self::Pad,
            1 => Self::SubnetMask,
            2 => Self::TimeOffset,
            3 => Self::Router,
            4 => Self::TimeServer,
            5 => Self::NameServer,
            7 => Self::LogServer,
            8 => Self::CookieServer,
            9 => Self::LprServer,
            10 => Self::ImpressServer,
            11 => Self::ResourceLocationServer,
            12 => Self::HostName,
            13 => Self::BootFileSize,
            14 => Self::MeritDumpFile,
            15 => Self::DomainName,
            16 => Self::SwapServer,
            17 => Self::RootPath,
            18 => Self::ExtensionsPath,
            19 => Self::IPForwardingEnableDisable,
            20 => Self::NonLocalSourceRoutingEnableDisable,
            21 => Self::PolicyFilter,
            22 => Self::MaximumDatagramReassemblySize,
            23 => Self::DefaultIpTimeToLive,
            24 => Self::PathMtuAgingTimeout,
            25 => Self::PathMtuPlateauTab,
            26 => Self::InterfaceMtu,
            27 => Self::AllSubnetsAreLocal,
            28 => Self::BroadcastAddress,
            29 => Self::PerformMaskDiscovery,
            30 => Self::MaskSupplier,
            31 => Self::PerformRouterDiscovery,
            32 => Self::RouterSolicitationAddress,
            33 => Self::StaticRoute,
            34 => Self::TrailerEncapsulationOption,
            35 => Self::ARPCacheTimeout,
            36 => Self::EthernetEncapsulation,
            37 => Self::TCPDefaultTtl,
            38 => Self::TCPKeepaliveInterval,
            39 => Self::TCPKeepaliveGarbage,
            40 => Self::NetworkInformationServiceDomain,
            41 => Self::NetworkInformationServers,
            42 => Self::NetworkTimeProtocolNtpServers,
            43 => Self::VendorSpecificInformation,
            44 => Self::NetBIOSOverTcpIpNameServer,
            45 => Self::NetBIOSOverTcpIpDatagramDistributionServer,
            46 => Self::NetBIOSOverTcpIpNodeType,
            47 => Self::NetBIOSOverTcpIpScope,
            48 => Self::XWindowSystemFontServer,
            49 => Self::XWindowSystemDisplayManager,
            50 => Self::RequestedIpAddress,
            51 => Self::IpAddressLeaseTime,
            52 => Self::OptionOverload,
            53 => Self::DhcpMessageType,
            54 => Self::ServerIdentifier,
            55 => Self::ParameterRequestList,
            56 => Self::Message,
            57 => Self::MaximumDhcpMessageSize,
            58 => Self::RenewalT1TimeValue,
            59 => Self::RebindingT2TimeValue,
            60 => Self::VendorClassIdentifier,
            61 => Self::ClientIdentifier,
            64 => Self::NetworkInformationServicePlusDomain,
            65 => Self::NetworkInformationServicePlusServers,
            66 => Self::TftpServerName,
            67 => Self::BootfileName,
            68 => Self::MobileIpHomeAgent,
            69 => Self::SmtpServer,
            70 => Self::Pop3Server,
            71 => Self::NntpServer,
            72 => Self::DefaultWwwServer,
            73 => Self::DefaultFingerProtocolServer,
            74 => Self::DefaultIrcServer,
            75 => Self::StreetTalkServer,
            76 => Self::StreetTalkDirectoryAssistanceServer,
            82 => Self::RelayAgentInformation,
            85 => Self::NovellDirectoryServiceServers,
            86 => Self::NdsTreeName,
            87 => Self::NdsContext,
            100 => Self::TimeZonePosixStyle,
            101 => Self::TimeZoneTzDatabaseStyle,
            119 => Self::DomainSearch,
            121 => Self::ClasslessStaticRoute,
            255 => Self::End,
            _ => Self::None
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum DhcpOp {
    Request = 1,
    Reply = 2,
}

impl DhcpOp {
    fn parse(dhcp_op: u8) -> DhcpOp {
        match dhcp_op {
            1 => Self::Request,
            2 => Self::Reply,
            _ => Self::Request,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum DhcpHardType {
    Ethernet = 1,
    Other = 2,
}

impl DhcpHardType {
    fn parse(dhcp_hard_type: u8) -> Self {
        match dhcp_hard_type {
            1 => DhcpHardType::Ethernet,
            _ => DhcpHardType::Other,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum DhcpHardwareLen {
    Ethernet = 6,
    Other = 7,
}

impl DhcpHardwareLen {
    fn parse(dhcp_hard_len: u8) -> Self {
        match dhcp_hard_len {
            6 => DhcpHardwareLen::Ethernet,
            _ => DhcpHardwareLen::Other,
        }
    }
}

#[repr(u16)]
#[derive(Copy, Clone)]
enum DhcpFlags {
    UniCasts = 0,
    BroadCasts = 1,
}

impl DhcpFlags {
    fn parse(flags: u16) -> Self {
        match flags {
            0 => DhcpFlags::UniCasts,
            1 => DhcpFlags::BroadCasts,
            _ => DhcpFlags::UniCasts,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum DhcpMessageType {
    Discover = 1,
    Offer = 2,
    Request = 3,
    AcknowledgementAck = 5,
    AcknowledgementNak = 6,
}

impl DhcpMessageType {
    fn parse(dhcp_message_type: u8) -> Self {
        match dhcp_message_type {
            1 => DhcpMessageType::Discover,
            2 => DhcpMessageType::Offer,
            3 => DhcpMessageType::Request,
            5 => DhcpMessageType::AcknowledgementAck,
            6 => DhcpMessageType::AcknowledgementNak,
            _ => DhcpMessageType::Discover,
        }
    }
}

#[repr(C)]
struct Dhcp {
    opcode: DhcpOp,
    htype: DhcpHardType,
    hlen: DhcpHardwareLen,
    hops: u8,
    xid: u32,
    secs: u16,
    flags: DhcpFlags,
    ciaddr: [u8; 4],
    yiaddr: [u8; 4],
    siaddr: [u8; 4],
    giaddr: [u8; 4],
    chaddr: [u8; 16],
    sname: [u8; 64],
    file: [u8; 128],
    magic_cookie: u32,
    options: DmaBox<[u8]>, // [u8; 312]
}

impl Dhcp {
    fn new() -> Dhcp {
        let my_mac_addr = get_mac_addr();
        Dhcp {
            opcode: DhcpOp::Request,
            htype: DhcpHardType::Ethernet,
            hlen: DhcpHardwareLen::Ethernet,
            hops: 0x0,
            xid: 0x00000000,
            secs: 0x0000,
            flags: DhcpFlags::UniCasts,
            ciaddr: [0, 0, 0, 0],
            yiaddr: [0, 0, 0, 0],
            siaddr: [0, 0, 0, 0],
            giaddr: [0, 0, 0, 0],
            chaddr: [
                my_mac_addr[0], my_mac_addr[1], my_mac_addr[2], my_mac_addr[3], my_mac_addr[4], my_mac_addr[5],
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            sname: [0x0; 64],
            file: [0x0; 128],
            magic_cookie: MAGIC_COOKIE,
            options: DmaBox::from(&[] as &[u8]), // [u8; 312]
        }
    }

    fn create_discover(&mut self, xid: Option<u32>) {
        self.xid = if xid.is_none() { get_uptime() as u32 } else { xid.unwrap() };
        self.create_discover_option();
    }

    fn create_request(&mut self) {
        if self.is_offer() {
            let mut printer = Printer::new(530, 330, 0);
            write!(printer, "{:?}", "is_offer!").unwrap(); // ここまではきた
            self.create_request_option();
            self.opcode = DhcpOp::Request;
            self.ciaddr = [0, 0, 0, 0];
            self.yiaddr = [0, 0, 0, 0];
            self.giaddr = [0, 0, 0, 0];
        }
    }

    fn create_discover_option(&mut self) {
        let dhcp_discover = DhcpOption {
            option_type: DhcpOptionType::DhcpMessageType,
            length: 1,
            value: DmaBox::from(&[DhcpMessageType::Discover as u8] as &[u8])
        };
        let request_ip = DhcpOption {
            option_type: DhcpOptionType::RequestedIpAddress,
            length: 4,
            value: DmaBox::from(&[192 as u8, 168 as u8, 56 as u8, 103 as u8] as &[u8])
        };
        let request_list = DhcpOption {
            option_type: DhcpOptionType::ParameterRequestList,
            length: 4,
            value: DmaBox::from(&[1 as u8, 3 as u8, 15 as u8, 6 as u8] as &[u8])
        };
        let end = DhcpOption {
            option_type: DhcpOptionType::End,
            length: 0,
            value: DmaBox::from(&[] as &[u8])
        };
        let len = dhcp_discover.len() + end.len();
        let empty: &[u8; 312] = &[0x00; 312];
        let option: DmaBox<[u8]> = DmaBox::from(&[
            &dhcp_discover.to_slice()[..],
            &request_ip.to_slice()[..],
            &request_list.to_slice()[..],
            &end.to_slice()[..],
            &empty[len..],
        ].concat() as &[u8]);
        self.options = option;
    }

    fn create_request_option(&mut self) {
        let received_your_ip_addr = &self.yiaddr;
        // let server_ip_addr = &self.siaddr;
        let dhcp_discover = DhcpOption {
            option_type: DhcpOptionType::DhcpMessageType,
            length: 1,
            value: DmaBox::from(&[DhcpMessageType::Request as u8] as &[u8])
        };
        let dhcp_request_ip = DhcpOption {
            option_type: DhcpOptionType::RequestedIpAddress,
            length: 4,
            value: DmaBox::from(&received_your_ip_addr[..])
        };
        // let dhcp_server = DhcpOption {
        //     option_type: DhcpOptionType::ServerIdentifier,
        //     length: 4,
        //     value: DmaBox::from(server_ip_addr)
        // };
        let end = DhcpOption {
            option_type: DhcpOptionType::End,
            length: 0,
            value: DmaBox::from(&[] as &[u8])
        };
        let len = dhcp_discover.len() + dhcp_request_ip.len() + end.len();
        let empty: &[u8; 312] = &[0x00; 312];
        let option: DmaBox<[u8]> = DmaBox::from(&[
            &dhcp_discover.to_slice()[..],
            &dhcp_request_ip.to_slice()[..],
            // &dhcp_server.to_slice()[..],
            &end.to_slice()[..],
            &empty[len..],
        ].concat() as &[u8]);
        self.options = option;
    }

    fn dhcp_message_type(&self) -> Option<DhcpMessageType> {
        let mut idx = 0;
        loop {
            let dhcp_message_type = self.options[idx];
            if dhcp_message_type == DhcpOptionType::DhcpMessageType as u8 {
                idx += 2;
                return Some(DhcpMessageType::parse(self.options[idx]));
            }
            idx += 1;
            if idx >= self.options.len() { return None; }

            let len = self.options[idx] as usize;
            idx += len + 1;
            if idx >= self.options.len() { return None; }
        }
    }

    fn is_discover(&self) -> bool {
        match self.dhcp_message_type() {
            Some(DhcpMessageType::Discover) => true,
            _ => false,
        }
    }

    fn is_offer(&self) -> bool {
        match self.dhcp_message_type() {
            Some(DhcpMessageType::Offer) => true,
            _ => false,
        }
    }

    fn is_request(&self) -> bool {
        match self.dhcp_message_type() {
            Some(DhcpMessageType::Request) => true,
            _ => false,
        }
    }

    fn is_ack(&self) -> bool {
        match self.dhcp_message_type() {
            Some(DhcpMessageType::AcknowledgementAck) => true,
            _ => false,
        }
    }

    fn is_nak(&self) -> bool {
        match self.dhcp_message_type() {
            Some(DhcpMessageType::AcknowledgementNak) => true,
            _ => false,
        }
    }

    fn to_slice(&self) -> DmaBox<[u8]> {
        let s: &[u8] = &[
            &(self.opcode as u8).to_be_bytes(),
            &(self.htype as u8).to_be_bytes(),
            &(self.hlen as u8).to_be_bytes(),
            &self.hops.to_be_bytes(),
            &self.xid.to_be_bytes()[..],
            &self.secs.to_be_bytes()[..],
            &(self.flags as u16).to_be_bytes()[..],
            &self.ciaddr[..],
            &self.yiaddr[..],
            &self.siaddr[..],
            &self.giaddr[..],
            &self.chaddr[..],
            &self.sname[..],
            &self.file[..],
            &self.magic_cookie.to_be_bytes(),
            &self.options[..],
        ].concat();
        DmaBox::from(s)
    }

    fn parsed_from_buf(buf: DmaBox<[u8]>) -> Dhcp {
        Dhcp {
            opcode: DhcpOp::parse(buf[0]),
            htype: DhcpHardType::parse(buf[1]),
            hlen: DhcpHardwareLen::parse(buf[2]),
            hops: buf[3],
            xid: (buf[4] as u32) << 24 | (buf[5] as u32) << 16 | (buf[6] as u32)  << 8 | (buf[7] as u32),
            secs: (buf[8] as u16)  << 8 | (buf[9] as u16),
            flags: DhcpFlags::parse((buf[10] as u16)  << 8 | (buf[11] as u16)),
            ciaddr: [buf[12], buf[13], buf[14], buf[15]],
            yiaddr: [buf[16], buf[17], buf[18], buf[19]],
            siaddr: [buf[20], buf[21], buf[22], buf[23]],
            giaddr: [buf[24], buf[25], buf[26], buf[27]],
            chaddr: Dhcp::pop_chaddr(&buf[28..=43]),
            sname: Dhcp::pop_sname(&buf[44..=107]),
            file: Dhcp::pop_file(&buf[108..=235]),
            magic_cookie: (buf[236] as u32) << 24 | (buf[237] as u32) << 16 | (buf[238] as u32)  << 8 | (buf[239] as u32),
            options: DmaBox::from(&buf[240..]), // [u8; 312]
        }
    }

    fn pop_chaddr(slice: &[u8]) -> [u8; 16] {
        let mut s: [u8; 16] = [0; 16];
        for idx in 0..15 {
            s[idx] = slice[idx];
        }
        s
    }
    fn pop_sname(slice: &[u8]) -> [u8; 64] {
        let mut s: [u8; 64] = [0; 64];
        for idx in 0..64 {
            s[idx] = slice[idx];
        }
        s
    }
    fn pop_file(slice: &[u8]) -> [u8; 128] {
        let mut s: [u8; 128] = [0; 128];
        for idx in 0..128 {
            s[idx] = slice[idx];
        }
        s
    }
}

pub fn request_discover() -> Result<(), String>{
    let mut dhcp = Dhcp::new();
    let xid = get_uptime() as u32;
    dhcp.create_discover(Some(xid));
    let source_port: u16 = 68;
    let dest_port: u16 = 67;
    let dest_ip_addr: [u8; 4] = [255, 255, 255, 255];
    send_udp(source_port, dest_port, dest_ip_addr, dhcp.to_slice())
}

pub fn reply_dhcp(received_ethernet_header: EthernetHdr, received_payload: DmaBox<[u8]>) -> Result<(), String> {
    let mut dhcp = Dhcp::parsed_from_buf(received_payload);
    let mut printer = Printer::new(600, 390, 0);
    write!(printer, "{:x}", dhcp.dhcp_message_type().unwrap() as u32).unwrap();
    if dhcp.is_offer() {
        let mut printer = Printer::new(500, 330, 0);
        write!(printer, "{:?}", "OFFER!").unwrap();
        dhcp.create_request();
        let mut printer = Printer::new(600, 330, 0);
        write!(printer, "{:?}", dhcp.yiaddr[0]).unwrap();
        let mut printer = Printer::new(615, 330, 0);
        write!(printer, "{:?}", dhcp.yiaddr[1]).unwrap();
        let mut printer = Printer::new(630, 330, 0);
        write!(printer, "{:?}", dhcp.yiaddr[2]).unwrap();
        let mut printer = Printer::new(645, 330, 0);
        write!(printer, "{:?}", dhcp.yiaddr[3]).unwrap();
        let mut printer = Printer::new(515, 330, 0);
        write!(printer, "{:?}", "OFFER CREATED!").unwrap();
        return reply_udp(received_ethernet_header, dhcp.to_slice(), false)
    }
    if dhcp.is_ack() {
        let allocated_ip = &dhcp.yiaddr;
        let mut printer = Printer::new(600, 345, 0);
        write!(printer, "{:?}", allocated_ip[0]).unwrap();
        let mut printer = Printer::new(650, 345, 0);
        write!(printer, "{:?}", allocated_ip[1]).unwrap();
        let mut printer = Printer::new(700, 345, 0);
        write!(printer, "{:?}", allocated_ip[2]).unwrap();
        let mut printer = Printer::new(750, 345, 0);
        write!(printer, "{:?}", allocated_ip[3]).unwrap();

        set_my_ip(allocated_ip)
    }
    Ok(())
}
