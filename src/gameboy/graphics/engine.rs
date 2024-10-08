use super::window::{HEIGHT, WIDTH};
use log::trace;

const WHITE: u32 = 0xffffff;
const LIGHT_GRAY: u32 = 0xa9a9a9;
const DARK_GRAY: u32 = 0x545454;
const BLACK: u32 = 0x000000;

pub struct Buffer {
    pub screen: Vec<u32>,
}
impl Buffer {
    pub fn wipe_line(&mut self, line: u8) {
        for p in 0..WIDTH {
            self.screen[line as usize * WIDTH + p] = 0xffffff;
        }
    }

    pub fn _wipe_screen(&mut self) {
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

    pub fn draw_tile(&mut self, tile: super::Tile, y: u8, tile_data: (u8, u8), palette: u8) {
        let skip = if tile.x < 8 { 8 - tile.x } else { 0 };

        let range: Box<dyn Iterator<Item = u8>> = if tile.is_x_flipped() {
            // panic!("ASD");
            Box::new(0..(8 - skip))
        } else {
            Box::new((0..(8 - skip)).rev())
        };

        for (i, pixel) in range.enumerate() {
            // println!("{} + {} - 8 - skip {} - px {}", tile.x, i, skip, pixel);
            let x = tile.x + (i as u8 + skip) - 8;
            let lsb = tile_data.0 & (1 << pixel) > 0;
            let msb = tile_data.1 & (1 << pixel) > 0;

            let color_code = (msb as u8) << 1 | lsb as u8;
            if color_code == 0 {
                trace!("skiping transparent for sprite");
                continue;
            }

            let pixel_to_draw = y as usize * WIDTH + x as usize;
            if !tile.has_priority() && (self.screen[pixel_to_draw] != WHITE) {
                trace!("skiping not object priority");
                continue;
            }

            let color_code = use_palette(palette, color_code);
            let color = get_color(color_code);
            // trace!("({}, {}) Color: {:#x} All: {:#x}",x,y,color,y as usize * WIDTH + x as usize);
            if y as usize >= HEIGHT || x as usize >= WIDTH {
                continue;
            }

            self.screen[pixel_to_draw] = color
        }
    }

    pub fn new() -> Self {
        let screen_buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
        Buffer {
            screen: screen_buffer,
        }
    }
}

fn get_color(color_code: u8) -> u32 {
    if color_code == 0 {
        WHITE // white
    } else if color_code == 1 {
        LIGHT_GRAY // light gray
    } else if color_code == 2 {
        DARK_GRAY // dark gray
    } else {
        BLACK // black
    }
}

pub fn use_palette(palette: u8, id: u8) -> u8 {
    let bit = 1 << (id * 2);
    let l = ((palette & bit) != 0) as u8;
    let m = ((palette & (bit << 1)) != 0) as u8;

    m << 1 | l
}
