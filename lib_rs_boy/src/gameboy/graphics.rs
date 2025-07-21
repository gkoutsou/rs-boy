pub(crate) mod engine;
mod processor;
mod tile;

use super::memory_bus::MemoryAccessor;
use crate::gameboy::interrupts;
pub use engine::Buffer;
use log::{debug, info, trace};
pub use processor::Mode;
pub use processor::Processor;
use std::cmp::min;
pub use tile::Tile;

const TILE_MAP_START_LOCATION: usize = 0x9800;
const TILE_MAP_END_LOCATION: usize = 0x9FFF;

pub struct Display {
    pub engine: Buffer,
    processor: Processor,

    tile_data: Vec<u8>,
    tile_maps: Vec<u8>,
    pub oam: Vec<u8>,

    dots: u32,
    /// when in ModeTwo we are iterating the oam memory, this is the index of the next to check
    oam_memory_check_index: u8,
    oam_collected_sprites: Vec<Tile>,
    interrupt: u8,
}

impl Display {
    pub fn gpu_step(&mut self, dots: u32) -> (u8, bool) {
        let mut trigger_render = false;
        self.interrupt = 0;
        if !self.processor.lcd_enabled() {
            trace!("LCD disabled!");
            self.dots = 0;
            return (self.interrupt, trigger_render);
        }
        self.dots += dots;

        if self.processor.should_trigger_lyc_stat_interrupt() {
            self.interrupt |= interrupts::STAT;
        }
        self.processor.lyc_or_ly_recently_changed = false;

        match self.processor.gpu_mode {
            Mode::Two => {
                let line = self.processor.ly;
                let double_size = self.processor.is_object_double_size();

                // Find objects in OAM memory (1 object every 2 dots)
                let start = self.oam_memory_check_index as usize;
                let end = (min(self.dots, 80) / 2 )as usize;

                for i in start..end {
                    self.oam_memory_check_index += 1;
                    let tile = self.get_oam_object(i);

                    if !tile.object_in_scanline(line, double_size) {
                        continue
                    }

                    // The amount of sprites already stored in the OAM Buffer must be less than 10
                    if self.oam_collected_sprites.len() == 10 {
                        continue;
                    }

                    self.oam_collected_sprites.push(tile);
                }

                if self.dots >= 80 {
                    self.dots -= 80;
                    self.set_gpu_mode(Mode::Three);
                }
            }
            Mode::One => {
                if self.dots >= 456 {
                    self.processor.ly += 1;
                    self.processor.lyc_or_ly_recently_changed = true;
                    self.dots -= 456;
                    // if self.processor.should_trigger_lyc_stat_interrupt() {
                    //     self.interrupt |= interrupts::STAT;
                    //     println!(
                    //         "todo: check and enable interrupt - lyc - One {}-{}",
                    //         self.processor.lyc, self.processor.ly
                    //     )
                    // }

                    if self.processor.ly > 153 {
                        self.processor.ly = 0;
                        self.processor.lyc_or_ly_recently_changed = true;
                        self.oam_memory_check_index = 0;
                        self.oam_collected_sprites.clear();
                        self.set_gpu_mode(Mode::Two);
                    }
                }
            }
            Mode::Zero => {
                if self.dots >= 204 {
                    self.dots -= 204;

                    self.processor.ly += 1;
                    self.processor.lyc_or_ly_recently_changed = true;
                    // if self.processor.should_trigger_lyc_stat_interrupt() {
                    //     self.interrupt |= interrupts::STAT;
                    //     debug!(
                    //         "todo: check and enable interrupt - lyc - Zero {}-{}",
                    //         self.processor.lyc, self.processor.ly
                    //     );
                    // }

                    if self.processor.ly == 144 {
                        self.interrupt |= interrupts::VBLANK;
                        // The window_y_counter should only reset on v-blank
                        self.processor.win_y_counter = 0;
                        trigger_render = true;

                        self.set_gpu_mode(Mode::One);
                    } else {
                        self.oam_memory_check_index = 0;
                        self.oam_collected_sprites.clear();
                        self.set_gpu_mode(Mode::Two);
                    }
                }
            }
            Mode::Three => {
                let line = self.processor.ly;

                if self.dots >= 172 {
                    // TODO: A bit lazy way for now, but let's iterate all the sprites and print them out at once.
                    //  Correct behaviour is to print a pixel/dot, and to mix pixels instead of drawing over BG
                    self.engine.wipe_line(line);
                    self.draw_background();
                    self.draw_sprites(line);

                    self.set_gpu_mode(Mode::Zero);
                    self.dots -= 172;
                }
            }
        }
        (self.interrupt, trigger_render)
    }

    fn draw_sprites(&mut self, line: u8) {
        if !self.processor.is_object_enabled() {
            trace!("objects are disabled :sadge:");
            return;
        }

        let double_size = self.processor.is_object_double_size();

        let mut previous_x_coordinate = 255;

        let mut indexed_sprites: Vec<(usize, &Tile)> = self.oam_collected_sprites.iter().enumerate().collect();
        indexed_sprites.sort_by(|a, b| {
            b.1.x.cmp(&a.1.x).then_with(|| b.0.cmp(&a.0))
        });

        for tile in indexed_sprites.iter().map(|&(_i, tile)| tile) {
            // If same X coordinate, the previous has priority
            if tile.x == previous_x_coordinate {
                debug!("same x, previous has priority");
                // todo!("this is wrong.. only if opaque!")
                // continue;
            }
            previous_x_coordinate = tile.x;

            if tile.x == 0 || tile.x >= 168 {
                debug!("sprite's x is outside of bounds. ignoring");
                continue;
            }

            let index = if double_size {
                if line + 16 - tile.y < 8 {
                    if !tile.is_y_flipped() {
                        tile.tile_index & 0xfe
                    } else {
                        tile.tile_index | 0x01
                    }
                } else {
                    if !tile.is_y_flipped() {
                        tile.tile_index | 0x01
                    } else {
                        tile.tile_index & 0xfe
                    }
                }
            } else {
                tile.tile_index
            };

            let y_pos = 16 + line as usize - tile.y as usize;
            let final_y_pos = if !tile.is_y_flipped() {
                y_pos % 8
            } else {
                7 - (y_pos % 8)
            };

            debug!("line: {} tile.y: {}", line, tile.y);
            let tile_data = self.get_tile_data(0x8000, index, final_y_pos);

            let palette = if (tile.flags & (1 << 4)) > 0 {
                self.processor.obp1
            } else {
                self.processor.obp0
            };
            self.engine.draw_tile(&tile, line, tile_data, palette);
        }
    }

    fn draw_background(&mut self) {
        let line = self.processor.ly;
        if !self.processor.is_bg_window_enabled() {
            trace!("bg/window is disabled. must draw white :sadge:");
            return;
        }

        let wx = self.processor.wx;
        let wy = self.processor.wy;
        let in_window =
            self.processor.is_window_enabled() && line >= wy && wx <= (engine::WIDTH + 7) as u8;

        for x in 0..160u8 {
            let in_window = in_window && x + 7 >= wx;

            let y_pos = if in_window {
                self.processor.win_y_counter
            } else {
                self.processor.scy.wrapping_add(line)
            };

            // which of the 8 vertical pixels of the current tile is the scanline on?
            let tile_row = y_pos / 8;

            let tile_map = self.processor.get_tile_map(in_window);

            // translate the current x pos to window space if necessary
            let x_pos = if in_window {
                x + 7 - wx
            } else {
                self.processor.scx.wrapping_add(x)
            };

            let tile_col = x_pos / 8;

            let tile_id = self.get_tile_map_id(tile_map, tile_row , tile_col);
            let tile_data_baseline = self.processor.get_tile_data_baseline();
            let tile_data = self.get_tile_data(tile_data_baseline, tile_id, y_pos as usize % 8);

            let palette = self.processor.bgp;
            self.engine.draw_bg_tile(x_pos, x, line, tile_data, palette);
        }
        if in_window {
            self.processor.win_y_counter += 1;
        }
    }

    fn set_gpu_mode(&mut self, mode: Mode) {
        self.processor.gpu_mode = mode;

        if self.processor.should_trigger_mode_stat_interrupt() {
            self.interrupt |= interrupts::STAT;
            debug!("todo: check and enable interrupt - mode");
        }
    }

    pub fn get_oam_object(&self, object: usize) -> Tile {
        let y = self.oam[object * 4];
        let x = self.oam[object * 4 + 1];
        let tile_index = self.oam[object * 4 + 2];
        let flags = self.oam[object * 4 + 3];
        Tile::new(y, x, tile_index, flags)
    }

    pub fn get_tile_data(&self, baseline: usize, id: u8, row: usize) -> (u8, u8) {
        let baseline = if baseline == 0x8800 {
            baseline - 0x8000 + (id as i8 as i16 + 128) as usize * 16
        } else {
            baseline - 0x8000 + id as usize * 16
        };
        // let id = id as usize;
        let a = self.tile_data[baseline + row * 2];
        let b = self.tile_data[baseline + row * 2 + 1];
        (a, b)
    }

    fn get_tile_map_id(&self, tile_map: usize, tile_row: u8, tile_col: u8) -> u8 {
        self.tile_maps[tile_map + tile_row as usize * 32 + tile_col as usize - TILE_MAP_START_LOCATION]
    }

    pub(crate) fn new() -> Self {
        Display {
            engine: Buffer::new(),
            processor: Processor::new(),

            tile_data: vec![0; 0x97FF - 0x8000 + 1],
            tile_maps: vec![0; TILE_MAP_END_LOCATION - TILE_MAP_START_LOCATION + 1],
            oam: vec![0; 0xFE9F - 0xFE00 + 1],

            dots: 0,
            interrupt: 0,
            oam_memory_check_index: 0,
            oam_collected_sprites: Vec::with_capacity(10),
        }
    }
}

impl MemoryAccessor for Display {
    fn get(&self, location: usize) -> u8 {
        match location {
            0x8000..=0x97FF => {
                if self.processor.gpu_mode == Mode::Three && self.processor.lcd_enabled() {
                    return 0xFF;
                }
                self.tile_data[location - 0x8000]
            },
            TILE_MAP_START_LOCATION..=TILE_MAP_END_LOCATION => {
                if (self.processor.gpu_mode == Mode::Three) && self.processor.lcd_enabled() {
                    return 0xFF;
                }
                self.tile_maps[location - TILE_MAP_START_LOCATION]
            },
            0xff40..=0xff4b => self.processor.get(location),
            0xFE00..=0xFE9F => {
                if (self.processor.gpu_mode != Mode::One && self.processor.gpu_mode != Mode::Zero) && self.processor.lcd_enabled() {
                    return 0xFF;
                }
                self.oam[location - 0xFE00]
            },

            _ => panic!("Unknown location: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0xfe00..=0xfe9f => {
                if (self.processor.gpu_mode != Mode::One && self.processor.gpu_mode != Mode::Zero) && self.processor.lcd_enabled() {
                    return;
                }
                self.oam[location - 0xfe00] = value;
            }

            0xff40..=0xff4b => self.processor.write(location, value),

            0x8000..=0x97FF => {
                if (self.processor.gpu_mode == Mode::Three) && self.processor.lcd_enabled() {
                    return
                }
                self.tile_data[location - 0x8000] = value
            }

            TILE_MAP_START_LOCATION..=TILE_MAP_END_LOCATION => {
                debug!("Writing to Tile Map");
                if (self.processor.gpu_mode == Mode::Three) && self.processor.lcd_enabled() {
                    return
                }
                self.tile_maps[location - TILE_MAP_START_LOCATION] = value
            }
            _ => {
                panic!(
                    "Memory write to graphics {:#x} value: {:#x}",
                    location, value
                );
            }
        }
    }
}
