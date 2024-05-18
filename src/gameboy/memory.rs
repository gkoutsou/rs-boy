mod io_registers;

pub use io_registers::IORegisters;
use log::{debug, trace};

pub struct Memory {
    high_ram: Vec<u8>,
    work_ram: Vec<u8>,

    /// I/O registers
    pub io_registers: IORegisters,

    pub interrupt_enable: u8,
}

impl Memory {
    pub fn get(&self, location: usize) -> u8 {
        match location {
            0xff80..=0xfffe => self.high_ram[location - 0xff80],
            0xc000..=0xdfff => self.work_ram[location - 0xc000],
            0xff00..=0xff77 => self.io_registers.get(location),
            0xffff => {
                trace!("IME");
                self.interrupt_enable
            }
            _ => panic!("Unknown location: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        match location {
            0xc000..=0xdfff => {
                // in CGB mode, the 2nd 4k are rotatable
                trace!("Writting to WRAM: {:#x}", location);
                self.work_ram[location - 0xc000] = value;
            }

            0xff00..=0xff7f => self.io_registers.write(location, value),

            0xff80..=0xfffe => {
                trace!("Writting to HRAM: {:#x}", location);
                self.high_ram[location - 0xff80] = value;
            }

            0xffff => {
                debug!(
                    "Writting to Interrupt Enable Register {:#b} -> {:#b}",
                    self.interrupt_enable, value
                );
                self.interrupt_enable = value;
            }

            _ => panic!("Memory write to {:#x} value: {:#x}", location, value),
        }
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

    pub fn _dump_tile(&self, _tile_id: u8) {
        // println!("DUMPING TILE DATA");
        // for i in 0..16 {
        //     print!("{:#04x} ", self.tile_data[tile_id as usize * 16 + i]);
        // }
        // println!();
        // println!("DUMPING TILE DATA COMPLETED");
    }

    pub fn _dump_oam(&self) {
        // println!("DUMPING OAM DATA");
        // for object in 0..40 {
        //     let tile = self.get_oam_object(object);
        //     println!("{:?}", tile)
        // }
        // println!("DUMPING OAM DATA COMPLETED");
    }

    pub fn new() -> Self {
        Memory {
            high_ram: vec![0; 0xfffe - 0xff80 + 1],
            work_ram: vec![0; 0xdfff - 0xc000 + 1], // 4+4 but half could be rotatable..

            io_registers: IORegisters::new(),

            interrupt_enable: 0,
        }
    }
}
