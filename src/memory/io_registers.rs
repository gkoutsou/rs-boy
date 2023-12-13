use log::{debug, trace};

use crate::gpu;

pub struct IORegisters {
    // ime: bool,
    // interrupt_enable: u8,
    // pub scanline: u8,
    pub interrupt_flag: u8,

    /// ff00
    joypad: u8,

    /// ff01
    serial_transfer_data: u8,
    /// ff02
    serial_transfer_control: u8,
    /// FF04
    div: u8,
    /// FF05
    tima: u8,
    /// FF06
    tma: u8,
    /// FF07
    tac: u8,

    /// ff40
    ///
    /// 7 - LCD & PPU enable: 0 = Off; 1 = On
    ///
    /// 6 - Window tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
    ///
    /// 5 - Window enable: 0 = Off; 1 = On
    ///
    /// 4 - BG & Window tile data area: 0 = 8800–97FF; 1 = 8000–8FFF
    ///
    /// 3 - BG tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
    ///
    /// 2 - OBJ size: 0 = 8×8; 1 = 8×16
    ///
    /// 1 - OBJ enable: 0 = Off; 1 = On
    ///
    /// 0 - BG & Window enable / priority [Different meaning in CGB Mode]: 0 = Off; 1 = On
    pub lcd_control: u8,
    /// ff41
    lcd_status: u8,
    /// ff42
    pub scy: u8,
    /// ff43
    pub scx: u8,
    /// ff44
    pub ly: u8,
    /// ff45
    lyc: u8,
    /// FF47
    bgp: u8,
    /// FF48
    obp0: u8,
    /// FF49
    obp1: u8,
    /// ff4a
    pub wy: u8,
    /// ff4b
    pub wx: u8,
}

impl IORegisters {
    pub fn get(&self, location: usize) -> u8 {
        match location {
            0xff00 => {
                // If neither buttons nor d-pad is selected ($30 was written), then the low nibble
                // reads $F (all buttons released).
                if self.joypad == 0x30 {
                    self.joypad | 0xf
                } else {
                    self.joypad
                }
            }
            0xff01 => self.serial_transfer_data,
            0xff02 => self.serial_transfer_control,
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,

            0xff40 => self.lcd_control,
            0xff41 => self.lcd_status,
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,

            0xff0f => self.interrupt_flag,

            // ignore
            // 0xFF4D => _
            _ => panic!("i/o register location read: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        trace!("Writting to I/O Register: {:#x}: {:#b}", location, value);
        match location {
            0xff00 => self.joypad = value,
            0xff01 => self.serial_transfer_data = value,
            0xff02 => self.serial_transfer_control = value,
            0xFF04 => self.div = value,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value,
            0xff40 => {
                self.lcd_control = value;
                // if value != 1 << 7 {
                // panic!("{:#b}", value)
                // }
            }
            0xff41 => self.lcd_status = value,
            0xff42 => self.scy = value,
            0xff43 => self.scx = value,
            0xff47 => self.bgp = value,
            0xff48 => self.obp0 = value,
            0xff49 => self.obp1 = value,
            0xff4a => self.wy = value,
            0xff4b => self.wx = value,
            0xff44 => panic!("writing to scanline"),
            0xff0f => self.interrupt_flag = value,

            // ignore
            0xFF4D => (),
            // sound
            0xFF10..=0xFF26 => (),

            // 0xff0f => self.interrupt_flag,
            _ => panic!("i/o register location write: {:#x}", location),
        }
    }

    pub fn enable_video_interrupt(&mut self) {
        self.interrupt_flag |= 0x1;
    }

    pub fn lcd_enabled(&self) -> bool {
        return self.lcd_control & gpu::LcdStatusFlag::LcdEnabled as u8 > 0;
    }

    pub fn has_lcd_flag(&self, flag: gpu::LcdStatusFlag) -> bool {
        return self.lcd_control & flag as u8 > 0;
    }

    pub fn default() -> IORegisters {
        IORegisters {
            joypad: 0,
            // scanline: 0,
            interrupt_flag: 0,
            lcd_control: 0x91,
            lcd_status: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            serial_transfer_data: 0,
            serial_transfer_control: 0,
            wy: 0,
            wx: 0,
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            bgp: 0xfc,
            obp0: 0xff,
            obp1: 0xff,
        }
    }
}
