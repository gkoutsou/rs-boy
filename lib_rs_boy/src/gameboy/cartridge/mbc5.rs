use super::Cartridge;
use crate::gameboy::memory_bus::MemoryAccessor;
use log::{debug, info, trace, warn};

pub struct MBC5 {
    rom: Vec<u8>,
    rom_bank: u16,

    // RAM - 8, 32, 128 are possible
    ram_enabled: bool,
    ram: Option<Vec<u8>>,
    ram_bank: u8,

    // MBC Specific
    // Rumble - not supported
    // TODO On cartridges which feature a rumble motor, bit 3 of the RAM Bank register is connected
    //  to the Rumble circuitry instead of the RAM chip. Setting the bit to 1 enables the rumble
    //  motor and keeps it enabled until the bit is reset again.

    rumble_support: bool,
    rumbling: bool,
    total_ram_banks: u8,
}

impl Cartridge for MBC5 {
    fn get_ram(&mut self) -> &mut [u8] {
        self.ram.as_deref_mut().unwrap_or(&mut [])
    }

    fn get_rumble_state(&self) -> Option<bool> {
        if self.rumble_support {
            return Some(self.rumbling)
        }

        None
    }
}

impl MemoryAccessor for MBC5 {
    fn get(&self, location: usize) -> u8 {
        match location {
            0x0000..=0x7FFF => self.get_rom(location),
            0xA000..=0xBFFF => self.get_external_ram(location),
            _ => panic!("Unknown location: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0x0000..=0x1FFF => {
                trace!(
                    "Setting external ram: {:#b} => {}",
                    value,
                    value & 0x0f == 0x0a
                );
                self.ram_enabled = value & 0x0f == 0x0a
            }

            0x2000..=0x2FFF => {
                // keep 9th bit, then place the incoming lower 8
                self.rom_bank = self.rom_bank & 0x100;
                self.rom_bank = self.rom_bank | value as u16;

                debug!("Changing to ROM bank: {}",self.rom_bank);
            }
            0x3000..=0x3FFF => {
                // keep lower 8 bits, then replace the 9th
                let new_value = (value as u16 & 0x1) << 8;
                self.rom_bank = self.rom_bank & 0xFF;
                self.rom_bank = self.rom_bank | new_value;

                debug!("Changing to ROM bank: {}",self.rom_bank);
            }
            0x4000..=0x5FFF => {
                let value = value & 0xF;
                if self.rumble_support {
                    self.rumbling = (value >> 3) > 0;
                }

                self.ram_bank = value % self.total_ram_banks;
                debug!("Changing to memory bank: {}", self.ram_bank);
            }

            0x6000..=0x7FFF => {
                warn!("Writing to memory location: {:#x}", location);
            }

            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    panic!("writing on cartridge when ram is disabled");
                    return;
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

impl MBC5 {
    pub fn get_rom(&self, location: usize) -> u8 {
        if location <= 0x3fff {
            self.rom[location]
        } else if (0x4000..=0x7fff).contains(&location) {
            let relative_loc = location - 0x4000;
            let actual_loc = relative_loc + (self.rom_bank as usize) * 0x4000;
            self.rom[actual_loc]
        } else {
            panic!("not a rom location! {:#x}", location)
        }
    }

    fn get_external_ram(&self, location: usize) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        let relative_loc = location - 0xA000;
        let actual_loc = relative_loc + (self.ram_bank as usize) * 0x2000;
        self.ram.as_ref().unwrap()[actual_loc]
    }

    pub fn new(buffer: Vec<u8>, external_ram: Option<Vec<u8>>, cartridge_type: u8) -> Self {
        let mut rumble_support = false;
        let mut ram = None;
        match cartridge_type {
            // $19	MBC5
            0x19 => {},
            // $1A	MBC5+RAM
            0x1A => {
                ram = external_ram;
            },
            // $1B	MBC5+RAM+BATTERY
            0x1B => {
                ram = external_ram;
            },
            // $1C	MBC5+RUMBLE
            0x1C => rumble_support = true,
            // $1D	MBC5+RUMBLE+RAM
            0x1D => {
                rumble_support = true;
                ram = external_ram;
            },
            // $1E	MBC5+RUMBLE+RAM+BATTERY
            0x1E => {
                rumble_support = true;
                ram = external_ram;
            }
            _ => {
                panic!("Unknown cartridge type! {:#x}", cartridge_type);
            }
        }

        let total_ram_banks = if ram.as_ref().is_some() { ram.as_ref().unwrap().len() / 8096 } else { 0 };

        MBC5 {
            rom: buffer,
            rom_bank: 1,
            ram,
            ram_enabled: false,
            ram_bank: 0,
            total_ram_banks: total_ram_banks as u8,
            rumble_support,
            rumbling: false,
        }
    }
}
