use std::{
    fs::File,
    io::{self, Read, Write},
    path::{self},
    str,
};

mod mbc1;
mod mbc3;
mod nombc;

use log::{debug, info, warn};

#[derive(PartialEq, Eq, Debug)]
enum Type {
    NoMBC,
    MBC1,
    MBC3,
}

pub(crate) trait Cartridge {
    fn get(&self, location: usize) -> u8;
    fn write(&mut self, location: usize, value: u8);
}

fn load_file(file_path: &path::Path) -> io::Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let mut buffer = Vec::new();

    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn load(file_path: path::PathBuf) -> Box<dyn Cartridge> {
    let result = load_file(file_path.as_path());

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
            Some(load_file(file_path).unwrap())
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

    match mbc_type {
        Type::NoMBC => Box::new(nombc::NO_MBC::new(buffer)),
        Type::MBC1 => Box::new(mbc1::MBC1::new(buffer, external_ram, save_file)),
        Type::MBC3 => Box::new(mbc3::MBC3::new(buffer, external_ram, save_file)),
    }
}
