mod io_registers;
pub use io_registers::IORegisters;
use log::{debug, info, trace};

use crate::gpu;

pub struct Memory {
    rom: Vec<u8>,
    rom_bank: u8,
    // ram: &'a mut Vec<u8>,
    high_ram: Vec<u8>,
    work_ram: Vec<u8>,

    tile_data: Vec<u8>,
    tile_maps: Vec<u8>,

    /// OAM
    oam: Vec<u8>,

    /// I/O registers
    pub io_registers: IORegisters,

    pub interrupt_enable: u8,

    debug_counter: u8,
}

impl Memory {
    pub fn get(&self, location: usize) -> u8 {
        if location <= 0x7fff {
            self.get_rom(location)
        } else if location <= 0xfffe && location >= 0xff80 {
            trace!("HRAM Read: {:#x}", location);
            self.high_ram[location - 0xff80]
        } else if location <= 0xdfff && location >= 0xc000 {
            trace!("WRAM Read: {:#x}", location);
            self.work_ram[location - 0xc000]
        } else if location <= 0x97FF && location >= 0x8000 {
            // trace!("Getting Tile Data: {:#x}", location);
            self.tile_data[location - 0x8000]
        } else if location <= 0x9FFF && location >= 0x9800 {
            debug!("Reading Tile Map");
            self.tile_maps[location - 0x9800]
        } else if location <= 0xff77 && location >= 0xff00 {
            self.io_registers.get(location)
        } else if location == 0xffff {
            trace!("IME");
            self.interrupt_enable
        } else {
            panic!("Unknown location: {:#x}", location)
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        if location <= 0x3fff && location >= 0x2000 {
            self.rom_bank = value & 0b11111;
            if self.rom_bank == 0 {
                // todo 20, 40 etc also step
                self.rom_bank = 1;
            }
            info!(
                "###### Changing to bank: {} (value: {})",
                self.rom_bank,
                value & 0b11111
            );
        } else if location <= 0x7FFF {
            panic!("how can I write to ROM?! {:#x}:{:0b}", location, value)
        } else if location <= 0xdfff && location >= 0xc000 {
            // in CGB mode, the 2nd 4k are rotatable
            trace!("Writting to WRAM: {:#x}", location);
            self.work_ram[location - 0xc000] = value;
        } else if location == 0xff46 {
            let location = (value as u16) << 8;
            debug!(
                "Triggering DMA transfter to OAM! {:#x} --> {:#x}",
                value, location
            );
            for i in 0..0xA0 {
                self.oam[i] = self.get(location as usize + i);
            }
            self.dump_oam()
        } else if location <= 0xff7f && location >= 0xff00 {
            self.io_registers.write(location, value);
        } else if location <= 0xfffe && location >= 0xff80 {
            trace!("Writting to HRAM: {:#x}", location);
            self.high_ram[location - 0xff80] = value;
        } else if location == 0xffff {
            debug!(
                "Writting to Interrupt Enable Register {:#b} -> {:#b}",
                self.interrupt_enable, value
            );
            self.interrupt_enable = value;
        } else if location <= 0x97FF && location >= 0x8000 {
            // println!("Writting to Tile Data");
            // panic!("Wrote: {:#x}", value);
            if value != 0 {
                debug!(
                    "finally! non empty in Tile Data: {:#x} - {:#b} = {:#x}",
                    location, value, value
                );
                // if self.debug_counter == 128 {
                //     self.dump_tile_data();
                //     panic!("reached counter")
                // }
                // self.debug_counter += 1;
            }
            self.tile_data[location - 0x8000] = value

            // Starts writing here in location: 0x36e3
        } else if location <= 0x9FFF && location >= 0x9800 {
            debug!("Writing to Tile Map");
            // panic!("ASDD");
            // if value != 0 {
            //     panic!(
            //         "finally! non empty in TileMaps: {:#x} - {:#b}",
            //         location, value
            //     )
            // }
            self.tile_maps[location - 0x9800] = value
            // Starts writing here in location: 36e3
        } else {
            panic!("Need to handle memory write to: {:#x}", location)
        }
    }

    pub fn get_rom(&self, location: usize) -> u8 {
        if location <= 0x3fff {
            self.rom[location as usize]
        } else if location <= 0x7fff && location >= 0x4000 {
            let relative_loc = location - 0x4000;
            let actual_loc = relative_loc + (self.rom_bank as usize) * 0x4000;
            // println!(
            // "Read from bank {} - location: {:#x}",
            // self.rom_bank, actual_loc
            // );
            self.rom[actual_loc]
        } else if location >= 0xff80 && location <= 0xfffe {
            self.high_ram[location - 0xff80]
        } else if location >= 0xc000 && location <= 0xDFFF {
            // todo temporary to try test framework
            self.work_ram[location - 0xc000]
        } else {
            panic!("not a rom location! {:#x}", location)
        }
    }

    pub fn get_ffxx(&self, steps: usize) -> u8 {
        let location = 0xff00 + steps as usize;
        self.get(location)
    }

    pub fn write_ffxx(&mut self, steps: u8, value: u8) {
        let location = 0xff00 + steps as usize;
        self.write(location, value);
    }

    pub fn dump_tile_data(&self) {
        // println!("DUMPING TILE DATA");
        // for tile in 0..384 {
        //     let mut sum = 0i32;
        //     for i in 0..16 {
        //         sum += self.tile_data[tile * 16 + i] as i32;
        //     }
        //     if sum > 0 {
        //         for i in 0..16 {
        //             print!("{:#04x} ", self.tile_data[tile * 16 + i]);
        //         }
        //         println!()
        //     }
        // }
        // println!("DUMPING TILE DATA COMPLETED");
    }

    pub fn dump_tile(&self, tile_id: u8) {
        println!("DUMPING TILE DATA");
        for i in 0..16 {
            print!("{:#04x} ", self.tile_data[tile_id as usize * 16 + i]);
        }
        println!();
        println!("DUMPING TILE DATA COMPLETED");
    }

    pub fn dump_oam(&self) {
        println!("DUMPING OAM DATA");
        for object in 0..40 {
            let tile = self.get_oam_object(object);
            println!("{:?}", tile)
        }
        println!("DUMPING OAM DATA COMPLETED");
    }

    pub fn get_oam_object(&self, object: usize) -> gpu::Tile {
        let y = self.oam[object * 4];
        let x = self.oam[object * 4 + 1];
        let tile_index = self.oam[object * 4 + 2];
        let flags = self.oam[object * 4 + 3];
        gpu::Tile::new(y, x, tile_index, flags)
    }

    pub fn get_tile_data(&self, baseline: usize, id: usize, row: usize) -> (u8, u8) {
        let a = self.tile_data[baseline - 0x8000 + id * 16 + row * 2];
        let b = self.tile_data[baseline - 0x8000 + id * 16 + row * 2 + 1];
        (a, b)
    }

    pub fn default_with_rom(buffer: Vec<u8>) -> Memory {
        Memory {
            rom: buffer,
            rom_bank: 1,

            high_ram: vec![0; 0xfffe - 0xff80 + 1],
            work_ram: vec![0; 0xdfff - 0xc000 + 1], // 4+4 but half could be rotatable..

            io_registers: IORegisters::default(),

            interrupt_enable: 0,

            tile_data: vec![0; 0x97FF - 0x8000 + 1],
            tile_maps: vec![0; 0x9FFF - 0x9800 + 1],
            oam: vec![0; 0xFE9F - 0xFE00 + 1],

            debug_counter: 0,
        }
    }
}
