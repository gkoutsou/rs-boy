use log::{debug, info, trace};

use crate::gpu;

pub struct IORegisters {
    // ime: bool,
    // interrupt_enable: u8,
    // pub scanline: u8,
    pub interrupt_flag: u8,

    /// ff01
    serial_transfer_data: u8,
    /// ff02
    serial_transfer_control: u8,
    /// FF04
    /// This register is incremented at a rate of 16384Hz (~16779Hz on SGB).
    /// Writing any value to this register resets it to $00. Additionally,
    /// this register is reset when executing the stop instruction, and only
    /// begins ticking again once stop mode ends.
    div: u8,
    /// FF05
    /// This timer is incremented at the clock frequency specified by the TAC
    /// register ($FF07). When the value overflows (exceeds $FF) it is reset to
    /// the value specified in TMA (FF06) and an interrupt is requested, as
    /// described below.
    tima: u8,
    /// FF06
    tma: u8,
    /// FF07
    tac: u8,

    /// FF26
    audio_master: u8,

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
    ///
    /// 6 - LYC int select
    ///
    /// 5 - Mode 2 int select
    ///
    /// 4 - Mode 1 int select
    ///
    /// 3 - Mode 0 int select
    ///
    /// 2 - LYC == LY
    ///
    /// 0-1 - PPU mode
    pub lcd_status: u8,
    /// ff42
    pub scy: u8,
    /// ff43
    pub scx: u8,
    /// ff44
    pub ly: u8,
    /// ff45
    lyc: u8, // TODO enable interrupt
    /// FF47
    pub bgp: u8,
    /// FF48
    pub obp0: u8,
    /// FF49
    obp1: u8,
    /// ff4a
    pub wy: u8,
    /// ff4b
    pub wx: u8,

    // temporary:
    step_timer: u32,
}

impl IORegisters {
    pub fn step_timer(&mut self, ticks: u32) {
        // a dot is: 4194000 Hz
        // div step:   16384 Hz
        // so a div is stepped every 255.981445313 dots
        self.step_timer += ticks / 4;
        if self.step_timer >= 256 {
            self.step_timer -= 256;
            self.div = self.div.wrapping_add(1);
            println!("div: {}", self.div)
        }
    }

    pub fn get(&self, location: usize) -> u8 {
        println!("Read: {:#x}", location);
        match location {
            0xff01 => self.serial_transfer_data,
            0xff02 => self.serial_transfer_control,
            0xFF04 => self.div,
            // 0xFF05 => self.tima,
            // 0xFF06 => self.tma,
            // 0xFF07 => self.tac,
            0xff40 => self.lcd_control,
            0xff41 => self.lcd_status,
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff45 => self.lyc,
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,

            0xff0f => self.interrupt_flag,

            // ignore
            // 0xFF4D => 0,
            // sound
            0xff26 => self.audio_master,
            0xFF10..=0xFF25 => 0, // todo
            _ => panic!("i/o register location read: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        trace!("Writting to I/O Register: {:#x}: {:#b}", location, value);
        match location {
            0xff01 => self.serial_transfer_data = value,
            0xff02 => self.serial_transfer_control = value,
            0xFF04 => self.div = 0, // writing any value resets it
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value,
            0xff40 => {
                if value & (1 << 7) == 0 && self.lcd_control & (1 << 7) != 0 {
                    info!("Disabling LCD {:#b}", value)
                } else if value & (1 << 7) != 0 && self.lcd_control & (1 << 7) == 0 {
                    info!("Enabling LCD {:#b}", value)
                }
                self.lcd_control = value;
            }
            0xff41 => self.lcd_status = value,
            0xff42 => self.scy = value,
            0xff43 => self.scx = value,
            0xff45 => self.lyc = value,
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
            0xff26 => self.audio_master = value,
            0xFF10..=0xFF25 => {
                // print!("{:#b}", value);
                // panic!("{:#x}", location)
            }
            0xFF30..=0xFF3F => (), // todo

            // 0xff0f => self.interrupt_flag,
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

    pub fn enable_video_interrupt(&mut self) {
        self.interrupt_flag |= 0x1;
    }

    pub fn lcd_enabled(&self) -> bool {
        return self.lcd_control & gpu::LcdStatusFlag::LcdEnabled as u8 > 0;
    }

    pub fn has_lcd_flag(&self, flag: gpu::LcdStatusFlag) -> bool {
        return self.lcd_control & flag as u8 > 0;
    }

    /// For window/background only
    pub fn get_tile_data_baseline(&self) -> usize {
        if self.has_lcd_flag(gpu::LcdStatusFlag::TileDataArea) {
            return 0x8000;
        } else {
            return 0x8800;
        }
    }

    pub fn get_tile_map(&self, in_window: bool) -> usize {
        let mut tilemap = 0x9800;

        // When LCDC.3 is enabled and the X coordinate of the current scanline is not inside the window then tilemap $9C00 is used.
        if !in_window && self.has_lcd_flag(gpu::LcdStatusFlag::BGTileMapArea) {
            tilemap = 0x9c00;
        }

        // When LCDC.6 is enabled and the X coordinate of the current scanline is inside the window then tilemap $9C00 is used.
        if in_window && self.has_lcd_flag(gpu::LcdStatusFlag::WindowTileMapArea) {
            tilemap = 0x9c00;
        }

        return tilemap;
    }

    pub fn default() -> IORegisters {
        IORegisters {
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
            audio_master: 0xf1, // todo crosscheck
            step_timer: 0,
        }
    }
}
