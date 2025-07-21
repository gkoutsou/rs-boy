use crate::gameboy::graphics::processor::LcdStatusFlag::LcdEnabled;
use crate::gameboy::memory_bus::MemoryAccessor;
use log::{info, trace};
use std::ptr::eq;

pub enum LcdStatusFlag {
    LcdEnabled = 1 << 7,
    WindowTileMapArea = 1 << 6,
    WindowEnabled = 1 << 5,
    TileDataArea = 1 << 4,
    BGTileMapArea = 1 << 3,
    ObjectSize = 1 << 2,
    ObjectEnabled = 1 << 1,
    BgWindowEnabled = 1 << 0, // This is in DMG or CGB-compat
}

impl LcdStatusFlag {
    fn to_byte(self) -> u8 {
        self as u8
    }
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
    /// ff46 DMA
    dma_last_value: u8,
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
    pub gpu_mode: Mode,
    /// used when the LCD is disabled as a cache of the last known state of ly==lyc. This allows us
    /// to 'freeze' that value until the ppu is enabled again
    frozen_compare_bit: bool,
    /// used so that we only trigger the lyc==ly STAT interrupt right after one of the two changed
    /// state. It is set to true right after we update either value, then resets after we (potentially)
    /// raise the interrupt
    pub lyc_or_ly_recently_changed: bool,
}

impl MemoryAccessor for Processor {
    fn get(&self, location: usize) -> u8 {
        trace!("Read: {:#x}", location);
        match location {
            0xff40 => self.lcd_control,
            0xff41 => {
                let compare_bit = if self.lcd_enabled() {
                    ((self.ly == self.lyc) as u8) << 2
                } else {
                    (self.frozen_compare_bit as u8) << 2
                };
                let ppu_mode = if self.lcd_enabled() { self.gpu_mode as u8 } else { 0 };
                info!("Read: {:#x}", 1<<7 | self.lcd_status | compare_bit | ppu_mode);
                1 << 7 | self.lcd_status | compare_bit | ppu_mode
            }
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff45 => self.lyc,
            0xff46 => self.dma_last_value, // Write Only
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,

            _ => panic!("gpu location read: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        trace!("Writing to gpu registers: {:#x}: {:#b}", location, value);
        match location {
            0xff40 => {
                if (value & LcdEnabled.to_byte() == 0) && (self.lcd_control & LcdEnabled.to_byte() != 0) {
                    info!("Disabling LCD {:#b}", value);
                    // the comparison bit is frozen when the LCD is disabled since the comparator is
                    // not running.
                    self.frozen_compare_bit = self.ly == self.lyc;
                    // TODO stat_lyc_onoff.gb (r1 step 3) requires this to be zero. But that feels
                    //  strange. Need to check more
                    // self.gpu_mode = Mode::Zero;
                    self.ly = 0;
                } else if (value & LcdEnabled.to_byte() != 0) && (self.lcd_control & LcdEnabled.to_byte() == 0) {
                    info!("Enabling LCD {:#b}", value);
                    // TODO When re-enabling the LCD, the PPU will immediately start drawing again,
                    //  but the screen will stay blank during the first frame
                    self.gpu_mode = Mode::Two;
                }
                // TODO is this needed?
                // if !self.has_lcd_flag(WindowEnabled) && (self.lcd_control & WindowEnabled as u8) == 0 {
                //     self.win_y_counter = 0;
                // }

                self.lcd_control = value;

            }
            0xff41 => self.lcd_status = value & !0b111,
            0xff42 => self.scy = value,
            0xff43 => self.scx = value,
            0xff45 => {
                // if value == self.ly {
                //     todo!("Do I need to trigger STAT interrupt?");
                // }
                trace!("LYC: {}", value);
                self.lyc_or_ly_recently_changed = self.lyc != value;
                self.lyc = value
            }
            0xff46 => self.dma_last_value = value, // This executes more
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
}

impl Processor {
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
        let equals = self.lcd_status & (1 << 6) > 0 && self.ly == self.lyc;

        // This should only trigger when the state changed, not every time the condition matches
        equals && self.lyc_or_ly_recently_changed
    }

    pub fn should_trigger_mode_stat_interrupt(&self) -> bool {
        if self.lcd_status & (1 << 5) > 0 && self.gpu_mode == Mode::Two {
            return true;
        }
        if self.lcd_status & (1 << 4) > 0 && self.gpu_mode == Mode::One {
            return true;
        }
        if self.lcd_status & (1 << 3) > 0 && self.gpu_mode == Mode::Zero {
            return true;
        }

        false
    }

    pub fn new() -> Self {
        Processor {
            // scanline: 0,
            lcd_control: 0x91,
            lcd_status: 0x80, // Becomes 86 due to ly=lyc & gpu_mode = Two
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            wy: 0,
            wx: 0,
            bgp: 0xfc,
            obp0: 0xff,
            obp1: 0xff,
            dma_last_value: 0xff,

            win_y_counter: 0,
            gpu_mode: Mode::Two, // First frame is empty
            frozen_compare_bit: true,
            lyc_or_ly_recently_changed: false,
        }
    }
}
