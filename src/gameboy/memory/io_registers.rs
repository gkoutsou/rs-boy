use log::{debug, trace};

use crate::gameboy::memory_bus::MemoryAccessor;

pub struct IORegisters {
    /// ff01
    serial_transfer_data: u8,
    /// ff02
    serial_transfer_control: u8,
}

impl MemoryAccessor for IORegisters {
    fn get(&self, location: usize) -> u8 {
        debug!("Read io/memory: {:#x}", location);
        match location {
            0xff01 => self.serial_transfer_data,
            0xff02 => self.serial_transfer_control,

            // ignore
            // 0xFF4D => 0,
            _ => panic!("i/o register location read: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        trace!("Writting to I/O Register: {:#x}: {:#b}", location, value);
        match location {
            0xff01 => self.serial_transfer_data = value,
            0xff02 => self.serial_transfer_control = value,

            // ignore
            0xFF4D => (),
            0xFF30..=0xFF3F => (), // todo

            0xFF56 => (),

            _ => {
                // let ten_millis = time::Duration::from_secs(10);
                // thread::sleep(ten_millis);
                panic!(
                    "i/o register location write: {:#x} - {:#x}",
                    location, value
                )
            }
        }
    }
}

impl IORegisters {
    pub fn new() -> IORegisters {
        IORegisters {
            // scanline: 0,
            serial_transfer_data: 0,
            serial_transfer_control: 0,
        }
    }
}
