use env_logger::Env;
use rs_boy::io;
use std::env;

use io::game_engine::Key as EngineKey;
use minifb::{Key, Window};
use rs_boy::gameboy::GameBoy;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

pub(crate) struct MiniFBScreen {
    window: Window,
}

impl io::game_engine::DrawingWindow for MiniFBScreen {
    fn refresh_buffer(&mut self, screen: &Vec<u32>) {
        if self.window.is_open() && !self.window.is_key_down(Key::Escape) {
            self.window
                .update_with_buffer(&screen, WIDTH, HEIGHT)
                .unwrap();
        } else {
            panic!("window deado")
        }
    }

    fn get_pressed_keys(&self) -> Vec<EngineKey> {
        let keys = self.window.get_keys();

        let mut output: Vec<EngineKey> = Vec::with_capacity(keys.len());
        for key in keys {
            let new_key: EngineKey = match key {
                Key::Enter => EngineKey::Enter,
                Key::Backspace => EngineKey::Backspace,
                Key::Down => EngineKey::Down,
                Key::Up => EngineKey::Up,
                Key::Left => EngineKey::Left,
                Key::Right => EngineKey::Right,
                Key::X => EngineKey::X,
                Key::Z => EngineKey::Z,
                _ => panic!("unknown key: {:?}", key),
            };

            output.push(new_key);
        }
        output
    }
}
fn main() {
    let window_opts = minifb::WindowOptions {
        scale: minifb::Scale::X2,
        ..Default::default()
    };

    let mut window =
        Window::new("Test - ESC to exit", WIDTH, HEIGHT, window_opts).unwrap_or_else(|e| {
            panic!("{}", e);
        });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16666)));

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .target(env_logger::Target::Stdout)
        .init();

    let mut use_speakers: bool = true;
    let mut rom_path: String = "".to_owned();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-na" | "--no-audio" => use_speakers = false,
            _ => {
                if rom_path.len() > 0 {
                    panic!("can only pass one rom file");
                }
                rom_path = arg;
            }
        }
    }

    if rom_path.len() == 0 {
        panic!("Please provide a rom");
    }

    let mut gb = GameBoy::new(&rom_path);
    gb.set_screen(Box::new(MiniFBScreen { window }));
    gb.start(use_speakers);
}
