pub struct NO_MBC {
    rom: Vec<u8>,
}

impl super::Cartridge for NO_MBC {
    fn get(&self, location: usize) -> u8 {
        match location {
            0x000..=0x7fff => self.rom[location],
            _ => panic!("Unknown location: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        panic!("no cartridge registers")
    }
}

impl NO_MBC {
    pub fn new(buffer: Vec<u8>) -> Self {
        NO_MBC { rom: buffer }
    }
}
