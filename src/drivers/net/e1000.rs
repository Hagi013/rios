use crate::arch::graphic::{Graphic, Printer, print_str};
use core::fmt::Write;


use super::super::bus::pci::{
  get_nic_reg,
  set_nic_reg,
};

const NIC_REG_EERD: u16 = 0x0014;

const NIC_EERD_START: u32 = 1 << 0;
const NIC_EERD_DONE: u32 = 1 << 4;
const NIC_EERD_ADDRESS_SHIFT: u32 = 8;
const NIC_EERD_NIC_DATA_SHIFT: u32 = 16;


const EERD_TIMEOUT: usize = 1000000;

pub fn get_eeprom_data(eeprom_addr: u8) -> i32 {
    set_nic_reg(NIC_REG_EERD, ((eeprom_addr as u32) << NIC_EERD_ADDRESS_SHIFT) | NIC_EERD_START);

    let mut wait = EERD_TIMEOUT.clone();
    while wait != 0 {
        let eerd: u32 = get_nic_reg(NIC_REG_EERD);
        if eerd & NIC_EERD_DONE == NIC_EERD_DONE {
            print_str(300, 315, "fetch eerd.", 0);
            return (eerd as i32) >> NIC_EERD_NIC_DATA_SHIFT;
        }
        wait -= 1;
        if wait == 0 {
            let mut printer = Printer::new(300, 315, 0);
            write!(printer, "{:x}", eerd).unwrap();
            print_str(300, 330, "TIMEOUT.", 0);
        }
    }
    return -1;
}

pub fn get_mac_addr() {
    let eeprom_accessible = get_eeprom_data(0x00);
    let mut printer = Printer::new(300, 230, 0);
    write!(printer, "{:?}", eeprom_accessible).unwrap();

    if eeprom_accessible >= 0 {
        print_str(300, 300, "EEPROM ACCESSIBLE.", 0);
    } else {
        print_str(300, 300, "EEPROM NOT ACCESSIBLE.", 0);
    }

    let mac_1_0: u16 = get_eeprom_data(0x00) as u16;
    let mac_3_2: u16 = get_eeprom_data(0x01) as u16;
    let mac_5_4: u16 = get_eeprom_data(0x02) as u16;
    let mac_address: [u16; 6] = [mac_1_0 & 0xff, mac_1_0 >> 8, mac_3_2 & 0xff, mac_3_2 >> 8, mac_5_4 & 0xff, mac_5_4 >> 8];

    for (idx, c) in mac_address.iter().enumerate() {
        let mut printer = Printer::new(300 + idx as u32 * 15, 345, 0);
        write!(printer, "{:x}", c).unwrap();
    }
}


