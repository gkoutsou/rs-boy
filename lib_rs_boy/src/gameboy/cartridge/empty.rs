use crate::gameboy::cartridge::Cartridge;
use crate::gameboy::memory_bus::MemoryAccessor;

pub struct Empty {}

impl MemoryAccessor for Empty {
    fn get(&self, _location: usize) -> u8 {
        panic!("Missing cartridge")
    }

    fn write(&mut self, _location: usize, _value: u8) {
        panic!("Missing cartridge")
    }
}

impl Cartridge for Empty {}
