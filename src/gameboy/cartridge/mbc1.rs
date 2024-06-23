use log::{debug, info};
use std::{
    fs::File,
    io::{Write},
    path::{self},
};

pub struct MBC1 {
    rom: Vec<u8>,
    rom_bank: u8,

    // RAM
    ram_enabled: bool,
    ram: Option<Vec<u8>>,
    ram_bank: u8,

    save_file: Option<path::PathBuf>,
}

impl super::Cartridge for MBC1 {
    fn get(&self, location: usize) -> u8 {
        match location {
            0x000..=0x7fff => self.get_rom(location),
            0xa000..=0xbfff => self.get_external_ram(location),
            _ => panic!("Unknown location: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0x0000..=0x1fff => {
                info!(
                    "Setting external ram: {:#b} => {}",
                    value,
                    value & 0x0f == 0x0a
                );
                self.ram_enabled = value & 0x0f == 0x0a
                // For MBC3 it also enables writing to Timer Registers
            }

            0x2000..=0x3fff => {
                self.rom_bank = value & 0b11111;

                if self.rom_bank == 0 {
                    // todo MBC1 has issue with 20, 40 etc
                    self.rom_bank = 1;
                }
                debug!(
                    "Changing to bank: {} (value: {})",
                    self.rom_bank,
                    value & 0b11111
                );
            }
            0x4000..=0x5fff => {
                if value <= 0x3 {
                    info!("Changing to memory bank: {}", self.ram_bank);
                    self.ram_bank = value;
                } else {
                    todo!("MBC1: not handled write to {:#x}", location)
                }
                // TODO In 1MB MBC1 multi-carts (see below), this 2-bit register is instead applied to bits 4-5 of the
                // ROM bank number and the top bit of the main 5-bit main ROM banking register is ignored.
            }
            0x6000..=0x7fff => {
                todo!("MBC1: Writing to location {:#x}", location)
            }
            0xa000..=0xbfff => {
                if !self.ram_enabled {
                    panic!("writing on cartridge when ram is disabled");
                }
                if self.ram.is_none() {
                    panic!("no external memory defined");
                }

                let relative_loc = location - 0xa000;
                let actual_loc = relative_loc + (self.ram_bank as usize) * 0x2000;
                self.ram
                    .as_mut()
                    .expect("there should be some cartridge memory now..")[actual_loc] = value;
            }

            _ => {
                panic!("Memory write to {:#x} value: {:#x}", location, value);
            }
        }
    }
}

impl Drop for MBC1 {
    fn drop(&mut self) {
        if let Some(filepath) = &self.save_file {
            let mut file = File::create(filepath).unwrap();
            let res = file.write_all(self.ram.as_ref().unwrap());
            if res.is_err() {
                panic!("{:?}", res);
            }
        }
    }
}

impl MBC1 {
    pub fn get_rom(&self, location: usize) -> u8 {
        if location <= 0x3fff {
            self.rom[location]
        } else if (0x4000..=0x7fff).contains(&location) {
            // TODO handle 1MB MBC1 ROMs
            let relative_loc = location - 0x4000;
            let actual_loc = relative_loc + (self.rom_bank as usize) * 0x4000;
            self.rom[actual_loc]
        } else {
            panic!("not a rom location! {:#x}", location)
        }
    }

    fn get_external_ram(&self, location: usize) -> u8 {
        let relative_loc = location - 0xA000;
        let actual_loc = relative_loc + (self.ram_bank as usize) * 0x2000;
        self.ram.as_ref().unwrap()[actual_loc]
    }

    pub fn new(
        buffer: Vec<u8>,
        external_ram: Option<Vec<u8>>,
        save_file: Option<path::PathBuf>,
    ) -> Self {
        MBC1 {
            rom: buffer,
            rom_bank: 1,
            ram: external_ram,
            ram_enabled: false,
            ram_bank: 0,
            save_file,
        }
    }
}
