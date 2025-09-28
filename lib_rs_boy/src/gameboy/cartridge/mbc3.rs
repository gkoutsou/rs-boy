use super::Cartridge;
use crate::gameboy::memory_bus::MemoryAccessor;
use log::{debug, info, trace, warn};

pub struct MBC3 {
    rom: Vec<u8>,
    rom_bank: u8,

    // RAM
    ram_enabled: bool,
    ram: Option<Vec<u8>>,
    ram_bank: u8,

    // MBC Specific
    rtc_access: bool,
    rtc_latched: bool,

    accessed_rtc_field: u8,
    latched_seconds: u8,
    latched_minutes: u8,
    latched_hours: u8,
    latched_day_lower_bit: u8,
    latched_day_upper_bit: bool,
    latched_halt: bool,
    latched_day_overflow_bit: bool,
}

impl Cartridge for MBC3 {
    fn get_ram(&mut self) -> &mut [u8] {
        self.ram.as_deref_mut().unwrap_or(&mut [])
    }
}

impl MemoryAccessor for MBC3 {
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
                trace!(
                    "Setting external ram: {:#b} => {}",
                    value,
                    value & 0x0f == 0x0a
                );
                self.ram_enabled = value & 0x0f == 0x0a
                // For MBC3 it also enables writing to Timer Registers
            }

            0x2000..=0x3fff => {
                self.rom_bank = value;

                if self.rom_bank == 0 {
                    self.rom_bank = 1;
                }
                debug!("Changing to ROM bank: {}",self.rom_bank);
            }
            0x4000..=0x5fff => {
                let value = value & 0xf;
                match value {
                    0x0..=0x7 => {
                        info!("Changing to RAM bank: {}", value);
                        self.ram_bank = value & 0x3; // This will act up if having more banks (mbc30)
                        self.rtc_access = false;
                    }
                    0x8..=0xc => {
                        self.rtc_access = true;
                        self.accessed_rtc_field = value;
                        debug!("TODO: Support RTC registers {:#x}", value)
                    }
                    _val => {
                        warn!("Weird RTC registers {:#x}", _val)

                    }
                }
            }
            0x6000..=0x7fff => {
                let latching = value == 1;
                debug!("Latch-change {} => {}", self.rtc_latched, latching);
                if !self.rtc_latched && latching {
                    // todo here we should actually set the internal variables correctly..
                    debug!("Latching is not implemented yet!");
                    self.latched_seconds = 0;
                    self.latched_minutes = 0;
                    self.latched_hours = 0;
                    self.latched_day_lower_bit = 0;
                    self.latched_day_upper_bit = false;
                    self.latched_halt = false;
                    self.latched_day_overflow_bit = false;
                } else if self.rtc_latched && latching {
                    debug!("from latched to latched!")
                } else if self.rtc_latched && !latching {
                    debug!("latch => 0!")
                } else if !self.rtc_latched && !latching {
                    debug!("from not-latched to not-latched!")
                }
                self.rtc_latched = latching;
                // todo!("Latch RTC")
            }
            0xa000..=0xbfff => {
                if self.rtc_access {
                    match self.accessed_rtc_field {
                        // TODO these are very sketchy at the moment
                        0x8 => self.latched_seconds = value,
                        0x9 => self.latched_minutes = value,
                        0xa => self.latched_hours = value,
                        0xb => self.latched_day_lower_bit = value,
                        0xc => {
                            self.latched_day_upper_bit = value == 1;
                            self.latched_halt = value & (1<<6) > 0;
                            self.latched_day_overflow_bit = value & (1<<7) > 0;
                        }

                        _ => todo!("MBC3: need to write RTC memory instead {:#x}: {:#b}", self.accessed_rtc_field, value),
                    }


                    return;
                }

                if !self.ram_enabled {
                    panic!("writing on cartridge when ram is disabled");
                }
                if self.ram.is_none() {
                    panic!("no external memory defined");
                }

                let relative_loc = location - 0xa000;
                let actual_loc = relative_loc + (self.ram_bank as usize) * 0x2000;
                // info!("Cartridge RAM location: \nrelative: {:#x}\n actual: {:#x}\n bank: {:#x}", relative_loc, actual_loc, self.ram_bank);
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

impl MBC3 {
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
        if self.rtc_access {
            debug!("MBC3: reading RTC memory instead");
            return match self.accessed_rtc_field {
                0x8 => self.latched_seconds,
                0x9 => self.latched_minutes,
                0xa => self.latched_hours,
                0xb => self.latched_day_lower_bit,
                0xc => self.latched_day_upper_bit as u8 |
                    ((self.latched_halt as u8) << 6) |
                ((self.latched_day_overflow_bit as u8) << 7),
                _ => panic!("not a rtc location! {:#x}", self.accessed_rtc_field),
            }
        }
        let relative_loc = location - 0xA000;
        let actual_loc = relative_loc + (self.ram_bank as usize) * 0x2000;
        self.ram.as_ref().unwrap()[actual_loc]
    }

    pub fn new(buffer: Vec<u8>, external_ram: Option<Vec<u8>>) -> Self {
        MBC3 {
            rom: buffer,
            rom_bank: 1,
            ram: external_ram,
            ram_enabled: false,
            ram_bank: 0,
            rtc_latched: false,
            rtc_access: false,

            accessed_rtc_field: 0,
            latched_seconds: 0,
            latched_minutes: 0,
            latched_hours: 0,
            latched_day_lower_bit: 0,
            latched_day_upper_bit: false,
            latched_halt: false,
            latched_day_overflow_bit: false,
        }
    }
}
