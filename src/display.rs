use log::trace;
use minifb::{Key, KeyRepeat, Window, WindowOptions};

use crate::gpu::Tile;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

pub struct Display {
    screen: Vec<u32>,
    window: Window,
}
impl Display {
    pub fn wipe_line(&mut self, line: u8) {
        for p in 0..WIDTH {
            self.screen[line as usize * WIDTH + p] = 0xffffff;
        }
    }

    pub fn wipe_screen(&mut self) {
        for elem in self.screen.iter_mut() {
            *elem = 0xffffff;
        }
    }

    pub fn draw_bg_tile(&mut self, x_pos: u8, x: u8, y: u8, tile_data: (u8, u8), palette: u8) {
        let pixel = 7 - (x_pos % 8);
        // let x = x + 7 - pixel;
        let lsb = tile_data.0 & (1 << pixel) > 0;
        let msb = tile_data.1 & (1 << pixel) > 0;

        let color_code = (msb as u8) << 1 | lsb as u8;

        let color_code = use_palette(palette, color_code);
        let color = get_color(color_code);
        if y as usize >= HEIGHT || x as usize >= WIDTH {
            return;
        }

        self.screen[y as usize * WIDTH + x as usize] = color
    }

    pub fn draw_tile(&mut self, tile: Tile, y: u8, tile_data: (u8, u8), palette: u8) {
        let skip = if tile.x < 8 { 8 - tile.x } else { 0 };

        for pixel in (0..8 - skip).rev() {
            let x = tile.x + 7 - pixel - 8;
            let lsb = tile_data.0 & (1 << pixel) > 0;
            let msb = tile_data.1 & (1 << pixel) > 0;

            let color_code = (msb as u8) << 1 | lsb as u8;
            if color_code == 0 {
                trace!("skiping transparent for sprite");
                continue;
            }

            let color_code = use_palette(palette, color_code);
            let color = get_color(color_code);
            trace!(
                "({}, {}) Color: {:#x} All: {:#x}",
                x,
                y,
                color,
                y as usize * WIDTH + x as usize
            );
            if y as usize >= HEIGHT || x as usize >= WIDTH {
                continue;
            }

            self.screen[y as usize * WIDTH + x as usize] = color
        }
    }

    pub fn default() -> Display {
        let screen_buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

        let mut window_opts = WindowOptions::default();
        window_opts.scale = minifb::Scale::X2;

        let mut window = Window::new("Test - ESC to exit", WIDTH, HEIGHT, window_opts)
            .unwrap_or_else(|e| {
                panic!("{}", e);
            });

        // Limit to max ~60 fps update rate
        window.limit_update_rate(Some(std::time::Duration::from_micros(16666)));
        // window.limit_update_rate(None);

        Display {
            screen: screen_buffer,
            window: window,
        }
    }

    pub fn refresh_buffer(&mut self) {
        if self.window.is_open() && !self.window.is_key_down(Key::Escape) {
            self.window
                .update_with_buffer(&self.screen, WIDTH, HEIGHT)
                .unwrap();
        } else {
            panic!("window deado")
        }
    }

    pub fn get_pressed_keys(&self) -> Vec<minifb::Key> {
        self.window.get_keys_pressed(KeyRepeat::Yes)
    }
}

fn get_color(color_code: u8) -> u32 {
    let color = if color_code == 0 {
        0xffffff // white
    } else if color_code == 1 {
        0xa9a9a9 // light gray
    } else if color_code == 2 {
        0x545454 // dark gray
    } else {
        0x000000 // black
    };
    color
}

pub fn use_palette(palette: u8, id: u8) -> u8 {
    let bit = 1 << (id * 2);
    let l = ((palette & bit) != 0) as u8;
    let m = ((palette & (bit << 1)) != 0) as u8;
    let color_id = m << 1 | l;

    return color_id;
}
