use std::str;

mod empty;
mod mbc1;
mod mbc3;
mod nombc;

use log::info;

use super::memory_bus::MemoryAccessor;

#[derive(PartialEq, Eq, Debug)]
enum Type {
    NoMBC,
    MBC1,
    MBC3,
}

pub trait Cartridge: MemoryAccessor {
    fn get_ram(&mut self) -> &mut [u8] {
        &mut [] // Default empty slice
    }
}

pub fn load_rom(rom: Vec<u8>) -> Box<dyn Cartridge> {
    if rom.len() < 0x150 {
        panic!("Rom size to small");
    }

    let title = str::from_utf8(&rom[0x134..0x142]).unwrap();
    let title = title.trim_end_matches(0x0 as char);

    info!("Title = {}", title);

    info!("CGB flag = {:#x}", rom[0x143]);
    info!("GB/SGB Indicator = {:#x}", rom[0x146]);
    let rom_size = rom[0x148];
    info!("ROM size = {:#x}", rom_size);
    let ram_size = rom[0x149];
    info!("RAM size = {:#x}", ram_size);

    let cartridge_type = rom[0x147];
    let mbc_type = match cartridge_type {
        0x0 => Type::NoMBC,
        0x1..=0x3 => Type::MBC1,
        0x0f..=0x13 => Type::MBC3,

        _t => todo!("unsupported mbc_type {:#x}", _t),
    };
    info!("Cartridge type: {:?} ({:#x})", mbc_type, cartridge_type);

    if rom_size >= 5 && mbc_type == Type::MBC1 {
        todo!("handle large MBC1 cartridges.")
    }

    let expected_rom_size = 32 * (2u32.pow(rom_size as u32)) * 1024u32;

    if rom.len() as u32 != expected_rom_size {
        panic!(
            "Wrong length found. Expected {} - Found {}",
            expected_rom_size,
            rom.len()
        );
    } else {
        info!("ROM size Bytes = {}", expected_rom_size);
    }

    let external_ram_size = match ram_size {
        0x00 => None,
        0x02 => Some(8 * 1024),
        0x03 => Some(32 * 1024),
        _ => panic!("not handled this ram size: {:#x}", ram_size),
    };

    let external_ram = if !external_ram_size.is_none() {
        Some(vec![0; external_ram_size.unwrap()])
    } else {
        None
    };

    match mbc_type {
        Type::NoMBC => Box::new(nombc::NoMBC::new(rom)),
        Type::MBC1 => Box::new(mbc1::MBC1::new(rom, external_ram)),
        Type::MBC3 => Box::new(mbc3::MBC3::new(rom, external_ram)),
    }
}

pub fn no_cartridge() -> Box<dyn Cartridge> {
    Box::new(empty::Empty {})
}
