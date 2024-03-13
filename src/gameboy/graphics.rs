pub(crate) mod engine;
mod processor;
mod tile;

pub use engine::Engine;
use log::debug;
use log::info;
use log::trace;
pub use processor::Mode;
pub use processor::Processor;
pub use tile::Tile;

use crate::gameboy::interrupts;

pub struct Display {
    engine: Engine,
    processor: Processor,

    tile_data: Vec<u8>,
    tile_maps: Vec<u8>,
    pub oam: Vec<u8>,

    dots: u32,
    gpu_mode: Mode,
    interrupt: u8,
}

impl Display {
    pub fn gpu_step(&mut self, dots: u32) -> (u8, Option<Vec<minifb::Key>>) {
        self.interrupt = 0;
        if !self.processor.lcd_enabled() {
            trace!("LCD disabled!");
            self.dots = 0;
            self.processor.ly = 0;
            self.set_gpu_mode(Mode::Two);
            return (self.interrupt, None);
        }
        self.dots += dots;

        let mut pressed_keys = None;

        match self.gpu_mode {
            Mode::Two => {
                let _line = self.processor.ly;
                if self.dots >= 80 {
                    // scan pixels TODO ideally I should follow the ticks, not do it at once
                    self.dots -= 80;
                    self.set_gpu_mode(Mode::Three);
                }
            }
            Mode::One => {
                if self.dots >= 456 {
                    self.processor.ly += 1;
                    self.dots -= 456;
                    if self.processor.should_trigger_lyc_stat_interrupt() {
                        self.interrupt |= interrupts::STAT;
                        println!(
                            "todo: check and enable interrupt - lyc - One {}-{}",
                            self.processor.lyc, self.processor.ly
                        )
                    }

                    if self.processor.ly > 153 {
                        self.processor.ly = 0;
                        self.set_gpu_mode(Mode::Two);
                    }
                }
            }
            Mode::Zero => {
                if self.dots >= 204 {
                    self.dots -= 204;

                    self.processor.ly += 1;
                    if self.processor.should_trigger_lyc_stat_interrupt() {
                        self.interrupt |= interrupts::STAT;
                        println!(
                            "todo: check and enable interrupt - lyc - Zero {}-{}",
                            self.processor.lyc, self.processor.ly
                        );
                    }

                    if self.processor.ly == 144 {
                        self.interrupt |= interrupts::VBLANK;

                        self.engine.refresh_buffer();

                        let keys = self.engine.get_pressed_keys();
                        pressed_keys = Some(keys);

                        self.set_gpu_mode(Mode::One);
                    } else {
                        self.set_gpu_mode(Mode::Two);
                    }
                }
            }
            Mode::Three => {
                let line = self.processor.ly;

                if self.dots >= 172 {
                    self.engine.wipe_line(line);
                    self.draw_background();
                    self.draw_sprites(line);

                    self.set_gpu_mode(Mode::Zero);
                    self.dots -= 172;
                }
            }
        }
        (self.interrupt, pressed_keys)
    }

    fn draw_sprites(&mut self, line: u8) {
        if !self.processor.is_object_enabled() {
            trace!("objects are disabled :sadge:");
            return;
        }

        let double_size = self.processor.is_object_double_size();

        let mut object_counter = 0;
        let mut previous_x_coordinate = 255;
        for i in 0..40 {
            let tile = self.get_oam_object(i);

            if tile.object_in_scanline(line, double_size) {
                object_counter += 1;
                debug!("{}: found object {:?}", line, tile);
                if object_counter > 10 {
                    trace!("too many sprites on the line. Is it a bug?");
                    break;
                }

                // If same X coordinate, the previous has priority
                if tile.x == previous_x_coordinate {
                    info!("same x, previous has priority");
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
                        tile.tile_index & 0xfe
                    } else {
                        tile.tile_index | 0x01
                    }
                } else {
                    tile.tile_index
                };

                // if not double size or the top tile for double
                // let y_pos = if !double_size || line + 16 - tile.y < 8 {
                //     16 + line as usize - tile.y as usize
                // } else {
                //     16 + line as usize - (tile.y + 8) as usize
                // };
                let y_pos = 16 + line as usize - tile.y as usize;
                let final_y_pos = if !tile.is_y_flipped() {
                    y_pos % 8
                } else {
                    // TODO flipped - double is probably broken
                    7 - (y_pos % 8)
                };

                debug!("line: {} tile.y: {}", line, tile.y);
                let tile_data = self.get_tile_data(0x8000, index, final_y_pos);

                let palette = if (tile.flags & (1 << 4)) > 0 {
                    self.processor.obp1
                } else {
                    self.processor.obp0
                };
                self.engine.draw_tile(tile, line, tile_data, palette);
            }
        }
    }

    fn draw_background(&mut self) {
        let line = self.processor.ly;
        if !self.processor.is_bg_window_enabled() {
            trace!("bg/window is disabled. must draw white :sadge:");
            // todo we also wipe_line one up. probably uneccessary
            self.engine.wipe_line(line);
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

            // which of the 8 vertical pixels of the current
            // tile is the scanline on?
            let tile_row = y_pos / 8;

            let tile_map = self.processor.get_tile_map(in_window);

            // translate the current x pos to window space if necessary
            let x_pos = if in_window {
                x + 7 - wx
            } else {
                x.wrapping_add(self.processor.scx)
            };

            let tile_col = x_pos / 8;

            let tile_id = self.get(tile_map + tile_row as usize * 32 + tile_col as usize);
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
        self.gpu_mode = mode;
        self.processor.lcd_status &= !3; // wipe 2 first digits
        self.processor.lcd_status |= mode as u8;

        if self.processor.should_trigger_mode_stat_interrupt(mode) {
            self.interrupt |= interrupts::STAT;
            println!("todo: check and enable interrupt - mode");
        }

        if mode == Mode::Two && self.processor.wy == self.processor.ly {
            // reset window counter
            self.processor.win_y_counter = 0
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

    pub fn get(&self, location: usize) -> u8 {
        if (0x8000..=0x97FF).contains(&location) {
            // trace!("Getting Tile Data: {:#x}", location);
            self.tile_data[location - 0x8000]
        } else if (0x9800..=0x9FFF).contains(&location) {
            // debug!("Reading Tile Map");
            self.tile_maps[location - 0x9800]
        } else if (0xff40..=0xff4b).contains(&location) {
            self.processor.get(location)
        } else {
            panic!("Unknown location: {:#x}", location)
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        match location {
            0xfe00..=0xfe9f => {
                self.oam[location - 0xfe00] = value;
            }

            0xff40..=0xff4b => self.processor.write(location, value),

            0x8000..=0x97FF => {
                if value != 0 {
                    debug!(
                        "finally! non empty in Tile Data: {:#x} - {:#b} = {:#x}",
                        location, value, value
                    );
                }
                self.tile_data[location - 0x8000] = value
            }

            0x9800..=0x9FFF => {
                debug!("Writing to Tile Map");
                self.tile_maps[location - 0x9800] = value
            }
            _ => {
                panic!(
                    "Memory write to graphics {:#x} value: {:#x}",
                    location, value
                );
            }
        }
    }

    pub(crate) fn default() -> Display {
        Display {
            engine: Engine::default(),
            processor: Processor::default(),

            tile_data: vec![0; 0x97FF - 0x8000 + 1],
            tile_maps: vec![0; 0x9FFF - 0x9800 + 1],
            oam: vec![0; 0xFE9F - 0xFE00 + 1],

            dots: 0,
            gpu_mode: Mode::Two,
            interrupt: 0,
        }
    }
}
