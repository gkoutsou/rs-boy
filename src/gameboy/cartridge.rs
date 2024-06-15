use std::{
    fs::File,
    io::{self, Read, Write},
    path::{self},
    str,
};

use log::{debug, info, warn};

#[derive(PartialEq, Eq, Debug)]
enum Type {
    NoMBC,
    MBC1,
    MBC3,
}

pub struct Cartridge {
    mbc_type: Type,

    rom: Vec<u8>,
    rom_bank: u8,

    // RAM
    ram_enabled: bool,
    ram: Option<Vec<u8>>,
    ram_bank: u8,

    // MBC Speficit
    rtc_access: bool,
    rtc_latched: bool,

    save_file: Option<path::PathBuf>,
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
        if self.mbc_type == Type::NoMBC {
            panic!("no cartridge registers")
        }
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
                match self.mbc_type {
                    Type::MBC1 => self.rom_bank = value & 0b11111,
                    Type::MBC3 => self.rom_bank = value,
                    _ => todo!(
                        "Writing to location {:#x} for: {:?}",
                        location,
                        self.mbc_type
                    ),
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
                    info!("Changing to memory bank: {}", self.ram_bank);
                    self.ram_bank = value;
                    self.rtc_access = false;
                } else if self.mbc_type == Type::MBC3 {
                    self.rtc_access = true;
                    todo!("support RTC registers");
                } else {
                    todo!("{:?}: not handled write to {:#x}", self.mbc_type, location)
                }
                // TODO In 1MB MBC1 multi-carts (see below), this 2-bit register is instead applied to bits 4-5 of the
                // ROM bank number and the top bit of the main 5-bit main ROM banking register is ignored.
            }
            0x6000..=0x7fff => {
                match self.mbc_type {
                    Type::MBC3 => {
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
                    _ => todo!("{:?}: Writing to location {:#x}", self.mbc_type, location,),
                }
            }
            0xa000..=0xbfff => {
                if !self.ram_enabled {
                    panic!("writing on cartridge when ram is disabled");
                }
                if self.ram.is_none() {
                    panic!("no external memory defined");
                }

                if self.mbc_type == Type::MBC3 && self.rtc_access {
                    todo!("MBC3: need to write RTC memory instead")
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
        if self.mbc_type == Type::MBC3 && self.rtc_access {
            todo!("MBC3: need to read RTC memory instead")
        }
        let relative_loc = location - 0xA000;
        let actual_loc = relative_loc + (self.ram_bank as usize) * 0x2000;
        self.ram.as_ref().unwrap()[actual_loc]
    }

    fn load_file(file_path: &path::Path) -> io::Result<Vec<u8>> {
        let mut f = File::open(file_path)?;
        let mut buffer = Vec::new();

        f.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    pub fn load(file_path: path::PathBuf) -> Cartridge {
        let result = Self::load_file(file_path.as_path());

        let buffer = result.unwrap();
        if buffer.len() < 0x150 {
            panic!("Rom size to small");
        }

        let title = str::from_utf8(&buffer[0x134..0x142]).unwrap();
        let title = title.trim_end_matches(0x0 as char);

        info!("Title = {}", title);

        info!("Type = {:#x}", buffer[0x143]);
        info!("GB/SGB Indicator = {:#x}", buffer[0x146]);
        let rom_size = buffer[0x148];
        info!("ROM size = {:#x}", rom_size);
        let ram_size = buffer[0x149];
        info!("RAM size = {:#x}", ram_size);

        let cartridge_type = buffer[0x147];
        let mbc_type = match cartridge_type {
            0x0 => Type::NoMBC,
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

        let external_ram_size = match ram_size {
            0x00 => None,
            0x02 => Some(8 * 1024),
            0x03 => Some(32 * 1024),
            _ => panic!("not handled this ram size: {:#x}", ram_size),
        };

        let save_file = if external_ram_size.is_some() {
            Some(path::PathBuf::from(title).with_extension("gbsave"))
        } else {
            None
        };

        let external_ram = if let Some(file_path) = &save_file {
            if file_path.exists() {
                Some(Self::load_file(file_path).unwrap())
            } else {
                Some(vec![0; external_ram_size.unwrap()])
            }
        } else {
            None
        };

        // let external_ram = if save_file.is_none() {
        //     None
        // } else if !save_file.as_ref().unwrap().exists() {
        //     Some(vec![0; external_ram_size.unwrap()])
        // } else {
        //     Some(Self::load_file(save_file.as_ref().unwrap().as_path()).unwrap())
        // };

        Cartridge {
            mbc_type,
            rom: buffer,
            rom_bank: 1,
            ram: external_ram,
            ram_enabled: false,
            ram_bank: 0,
            save_file,
            rtc_latched: false,
            rtc_access: false,
        }
    }
}

impl Drop for Cartridge {
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
