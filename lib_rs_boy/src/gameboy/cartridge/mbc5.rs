use super::Cartridge;
use crate::gameboy::memory_bus::MemoryAccessor;
use log::{debug, info, trace};

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

}

impl Cartridge for MBC5 {
    fn get_ram(&mut self) -> &mut [u8] {
        self.ram.as_deref_mut().unwrap_or(&mut [])
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
                if value <= 0xF {
                    info!("Changing to memory bank: {}", self.ram_bank);
                    self.ram_bank = value;
                } else {
                    todo!("MBC5: not handled write to {:#x}", location)
                }
            }

            0xA000..=0xBFFF => {
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
        let relative_loc = location - 0xA000;
        let actual_loc = relative_loc + (self.ram_bank as usize) * 0x2000;
        self.ram.as_ref().unwrap()[actual_loc]
    }

    pub fn new(buffer: Vec<u8>, external_ram: Option<Vec<u8>>) -> Self {
        MBC5 {
            rom: buffer,
            rom_bank: 1,
            ram: external_ram,
            ram_enabled: false,
            ram_bank: 0,
        }
    }
}
