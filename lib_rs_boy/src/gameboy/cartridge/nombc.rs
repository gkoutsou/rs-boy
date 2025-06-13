use super::Cartridge;
use crate::gameboy::memory_bus::MemoryAccessor;

pub struct NoMBC {
    rom: Vec<u8>,
}

impl Cartridge for NoMBC {}

impl MemoryAccessor for NoMBC {
    fn get(&self, location: usize) -> u8 {
        match location {
            0x000..=0x7fff => self.rom[location],
            _ => panic!("Unknown location: {:#x}", location),
        }
    }

    fn write(&mut self, _location: usize, _value: u8) {
        panic!("no cartridge registers")
    }
}

impl NoMBC {
    pub fn new(buffer: Vec<u8>) -> Self {
        NoMBC { rom: buffer }
    }
}
