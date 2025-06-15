use crate::gameboy::cartridge::Cartridge;
use crate::gameboy::memory_bus::MemoryAccessor;

pub struct Empty {}

impl MemoryAccessor for Empty {
    fn get(&self, location: usize) -> u8 {
        panic!("Missing cartridge")
    }

    fn write(&mut self, location: usize, value: u8) {
        panic!("Missing cartridge")
    }
}

impl Cartridge for Empty {}
