use log::trace;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 160;
const HEIGHT: usize = 140;

pub struct Display {
    screen: Vec<u32>,
    window: Window,
}
impl Display {
    pub fn wipe_screen(&mut self) {
        for elem in self.screen.iter_mut() {
            *elem = 0xffffff;
        }
    }

    pub fn draw_tile(&mut self, x: u8, y: u8, lsb_byte: u8, msb_byte: u8, is_sprite: bool) {
        if is_sprite {
            println!("DRAWING: ({},{}) {:#x} {:#x}", x, y, lsb_byte, msb_byte)
        }
        for pixel in (0..8).rev() {
            let x = x + 7 - pixel;
            let lsb = lsb_byte & (1 << pixel) > 0;
            let msb = msb_byte & (1 << pixel) > 0;

            let color_code = (msb as u8) << 1 | lsb as u8;
            if is_sprite && color_code == 0 {
                trace!("skiping transparent for sprite");
                continue;
            }

            let color = if color_code == 1 {
                0xCDC392
            } else if color_code == 2 {
                0xE8E5DA
            } else if color_code == 3 {
                0x9EB7E5
            } else {
                0xffffff
            };
            if is_sprite {
                println!(
                    "({}, {}) Color: {:#x} All: {:#x}",
                    x,
                    y,
                    color,
                    y as usize * WIDTH + x as usize
                );
            }
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
        window.limit_update_rate(Some(std::time::Duration::from_nanos(119714)));
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
}
