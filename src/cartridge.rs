use std::{
    fs::File,
    io::{self, Read},
    str,
};

use log::{debug, info, warn};

#[derive(PartialEq, Eq, Debug)]
enum Type {
    MBC1,
    MBC3,
}

pub struct Cartridge {
    rom: Vec<u8>,
    rom_bank: u8,
    mbc_type: Type,

    // RAM
    cartridge_memory_enabled: bool,
    pub external_memory: Option<Vec<u8>>,
    pub external_memory_bank: u8,

    rtc_latched: bool,
}

impl Cartridge {
    pub fn get(&self, location: usize) -> u8 {
        match location {
            0x000..=0x7fff => self.get_rom(location),
            0xa000..=0xbfff => self.get_external_ram(location),
            _ => panic!("Unknown location: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        match location {
            0x0000..=0x1fff => {
                info!(
                    "Setting external ram: {:#b} => {}",
                    value,
                    value & 0x0f == 0x0a
                );
                self.cartridge_memory_enabled = value & 0x0f == 0x0a
            }

            0x2000..=0x3fff => {
                match self.mbc_type {
                    Type::MBC1 => self.rom_bank = value & 0b11111,
                    Type::MBC3 => self.rom_bank = value,
                };

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
                    info!("Changing to memory bank: {}", self.external_memory_bank);
                    self.external_memory_bank = value
                } else {
                    todo!("support RTC registers");
                }
            }
            0x6000..=0x7fff => {
                let set_one = value == 1;
                info!("Latch-change {} => {}", self.rtc_latched, set_one);
                if !self.rtc_latched && set_one {
                    // todo here we should actually set some internal variables so that we can read
                    // todo!("Latching!")
                } else if self.rtc_latched && set_one {
                    panic!("from latched to latched!")
                } else if self.rtc_latched && !set_one {
                    todo!("latch => 0!")
                } else if !self.rtc_latched && !set_one {
                    warn!("from not-latched to not-latched!")
                }
                // todo!("Latch RTC")
            }
            0xa000..=0xbfff => {
                if !self.cartridge_memory_enabled {
                    panic!("writing on cartridge when ram is disabled");
                }
                if self.external_memory.is_none() {
                    panic!("no external memory defined");
                }
                let relative_loc = location - 0xa000;
                let actual_loc = relative_loc + (self.external_memory_bank as usize) * 0x2000;
                self.external_memory
                    .as_mut()
                    .expect("there should be some cartridge memory now..")[actual_loc];
            }

            _ => {
                panic!("Memory write to {:#x} value: {:#x}", location, value);
            }
        }
    }

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
        let actual_loc = relative_loc + (self.external_memory_bank as usize) * 0x2000;
        self.rom[actual_loc]
    }

    fn load_rom(file_path: &str) -> io::Result<Vec<u8>> {
        let mut f = File::open(file_path)?;
        let mut buffer = Vec::new();

        f.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    pub fn default(file_path: &str) -> Cartridge {
        let result = Self::load_rom(file_path);

        let buffer = result.unwrap();
        if buffer.len() < 0x150 {
            panic!("Rom size to small");
        }

        let _title = str::from_utf8(&buffer[0x134..0x142]).unwrap();

        // println!("Title = {}", title);

        info!("Type = {:#x}", buffer[0x143]);
        info!("GB/SGB Indicator = {:#x}", buffer[0x146]);
        let rom_size = buffer[0x148];
        info!("ROM size = {:#x}", rom_size);
        let ram_size = buffer[0x149];
        info!("RAM size = {:#x}", ram_size);

        let cartridge_type = buffer[0x147];
        let mbc_type = match cartridge_type {
            0x1..=0x3 => Type::MBC1,
            0x0f..=0x13 => Type::MBC3,

            _t => todo!("unsupported mbc_type {:#x}", _t),
        };
        info!("Cartridge type: {:?} ({:#x})", mbc_type, cartridge_type);
        // std::panic::set_hook(Box::new(|panic_info| {
        //     let backtrace = std::backtrace::Backtrace::capture();
        //     eprintln!("My backtrace: {:#?}", backtrace);
        // }));

        if rom_size >= 5 && mbc_type == Type::MBC1 {
            todo!("handle large MBC1 cartridges.")
        }

        let expected_rom_size = 32 * (2u32.pow(rom_size as u32)) * 1024u32;

        if buffer.len() as u32 != expected_rom_size {
            panic!(
                "Wrong length found. Expected {} - Found {}",
                expected_rom_size,
                buffer.len()
            );
        } else {
            println!("ROM size Bytes = {}", expected_rom_size);
        }

        let external_ram = match ram_size {
            0x00 => None,
            0x02 => Some(vec![0; 8 * 1024]),
            0x03 => Some(vec![0; 32 * 1024]),
            _ => panic!("not handled this ram size: {:#x}", ram_size),
        };

        Cartridge {
            rom: buffer,
            rom_bank: 1,
            mbc_type: mbc_type,
            cartridge_memory_enabled: false,
            external_memory: external_ram,
            external_memory_bank: 0,
            rtc_latched: false,
        }
    }
}
