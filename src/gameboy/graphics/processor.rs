use log::{info, trace};

pub enum LcdStatusFlag {
    LcdEnabled = 1 << 7,
    WindowTileMapArea = 1 << 6,
    WindowEnabled = 1 << 5,
    TileDataArea = 1 << 4,
    BGTileMapArea = 1 << 3,
    ObjectSize = 1 << 2,
    ObjectEnabled = 1 << 1,
    BgWindowEnabled = 1 << 0,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

pub struct Processor {
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
    pub lyc: u8,
    /// FF47
    pub bgp: u8,
    /// FF48
    pub obp0: u8,
    /// FF49
    pub obp1: u8,
    /// ff4a
    pub wy: u8,
    /// ff4b
    pub wx: u8,

    //Helpers
    pub win_y_counter: u8,
}

impl Processor {
    pub fn get(&self, location: usize) -> u8 {
        trace!("Read: {:#x}", location);
        match location {
            0xff40 => self.lcd_control,
            0xff41 => {
                let compare = (self.ly == self.lyc) as u8;
                self.lcd_status | (compare << 2)
            }
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff45 => self.lyc,
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,

            _ => panic!("gpu location read: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        trace!("Writting to gpu registers: {:#x}: {:#b}", location, value);
        match location {
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
            0xff45 => {
                if value == self.ly {
                    todo!("Do I need to trigger STAT interrupt?");
                }
                trace!("LYC: {}", value);
                self.lyc = value
            }
            0xff47 => self.bgp = value,
            0xff48 => self.obp0 = value,
            0xff49 => self.obp1 = value,
            0xff4a => self.wy = value,
            0xff4b => self.wx = value,
            0xff44 => panic!("writing to scanline"),

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

    pub fn is_object_double_size(&self) -> bool {
        self.has_lcd_flag(LcdStatusFlag::ObjectSize)
    }

    pub fn is_object_enabled(&self) -> bool {
        self.has_lcd_flag(LcdStatusFlag::ObjectEnabled)
    }

    pub fn is_bg_window_enabled(&self) -> bool {
        self.has_lcd_flag(LcdStatusFlag::BgWindowEnabled)
    }

    pub fn is_window_enabled(&self) -> bool {
        self.has_lcd_flag(LcdStatusFlag::WindowEnabled)
    }

    pub fn lcd_enabled(&self) -> bool {
        self.lcd_control & LcdStatusFlag::LcdEnabled as u8 > 0
    }

    fn has_lcd_flag(&self, flag: LcdStatusFlag) -> bool {
        self.lcd_control & flag as u8 > 0
    }

    /// For window/background only
    pub fn get_tile_data_baseline(&self) -> usize {
        if self.has_lcd_flag(LcdStatusFlag::TileDataArea) {
            0x8000
        } else {
            0x8800
        }
    }

    pub fn get_tile_map(&self, in_window: bool) -> usize {
        let mut tilemap = 0x9800;

        // When LCDC.3 is enabled and the X coordinate of the current scanline is not inside the window then tilemap $9C00 is used.
        if !in_window && self.has_lcd_flag(LcdStatusFlag::BGTileMapArea) {
            tilemap = 0x9c00;
        }

        // When LCDC.6 is enabled and the X coordinate of the current scanline is inside the window then tilemap $9C00 is used.
        if in_window && self.has_lcd_flag(LcdStatusFlag::WindowTileMapArea) {
            tilemap = 0x9c00;
        }

        tilemap
    }

    pub fn should_trigger_lyc_stat_interrupt(&self) -> bool {
        self.lcd_status & (1 << 6) > 0 && self.ly == self.lyc
    }

    pub fn should_trigger_mode_stat_interrupt(&self, mode: Mode) -> bool {
        if self.lcd_status & (1 << 5) > 0 && mode == Mode::Two {
            return true;
        }
        if self.lcd_status & (1 << 4) > 0 && mode == Mode::One {
            return true;
        }
        if self.lcd_status & (1 << 3) > 0 && mode == Mode::Zero {
            return true;
        }

        false
    }

    pub fn default() -> Processor {
        Processor {
            // scanline: 0,
            lcd_control: 0x91,
            lcd_status: 0x86, // I start with mode 2 instead of 1 (since ly = 0)
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            wy: 0,
            wx: 0,
            bgp: 0xfc,
            obp0: 0xff,
            obp1: 0xff,

            win_y_counter: 0,
        }
    }
}
