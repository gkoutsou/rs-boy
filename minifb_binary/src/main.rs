use env_logger::Env;
use io::keys::Key as EngineKey;
use lib_rs_boy::gameboy::GameBoy;
use lib_rs_boy::io;
use log::info;
use minifb::{Key, Window};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{env, panic, path};

const WIDTH: usize = 160;
const HEIGHT: usize = 144;
const FRAMERATE: usize = 60;

#[inline]
fn get_pressed_keys(keys: Vec<Key>) -> Vec<EngineKey> {
    let mut output: Vec<EngineKey> = Vec::with_capacity(keys.len());
    for key in keys {
        let new_key: EngineKey = match key {
            Key::Enter => EngineKey::Start,
            Key::Backspace => EngineKey::Select,
            Key::Down => EngineKey::Down,
            Key::Up => EngineKey::Up,
            Key::Left => EngineKey::Left,
            Key::Right => EngineKey::Right,
            Key::X => EngineKey::A,
            Key::Z => EngineKey::B,
            _ => EngineKey::Start, // Random key, go!
        };

        output.push(new_key);
    }
    output
}

struct SaveOnDrop {
    gameboy: GameBoy,
    save_file: Option<PathBuf>,
}

impl Drop for SaveOnDrop {
    fn drop(&mut self) {
        if let Some(filepath) = &self.save_file {
            info!("Saving save file to {:?}", filepath);
            let mut file = File::create(filepath).unwrap();
            let res = file.write_all(self.gameboy.cartridge.get_ram());
            if res.is_err() {
                panic!("{:?}", res);
            }
        }
    }
}

fn main() {
    let window_opts = minifb::WindowOptions {
        scale: minifb::Scale::X2,
        ..Default::default()
    };

    let mut window = Window::new("Rs-Boy", WIDTH, HEIGHT, window_opts).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(FRAMERATE);

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
    let result = io::files::load_file(rom_path.as_ref());

    let rom = result.unwrap();

    let mut gb = GameBoy::new();
    gb.load_rom(rom);

    // Check if there is save file stored in filesystem
    let save_file = if gb.cartridge.get_ram().len() > 0 {
        Some(path::PathBuf::from(rom_path).with_extension("gbsave"))
    } else {
        None
    };

    if let Some(file_path) = &save_file {
        if file_path.exists() {
            gb.cartridge
                .get_ram()
                .copy_from_slice(io::files::load_file(file_path).unwrap().as_slice());
        }
    };

    if !use_speakers {
        todo!("implement me");
    }
    let sdl_context = sdl2::init().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(lib_rs_boy::gameboy::audio::AUDIO_SAMPLE_RATE),
        channels: Some(2), // stereo
        samples: None,
    };

    let audio: AudioQueue<i16> = audio_subsystem.open_queue(None, &desired_spec).unwrap();
    audio.resume();

    let mut gb_with_save_on_drop = SaveOnDrop {
        gameboy: gb,
        save_file,
    };

    loop {
        let render = gb_with_save_on_drop.gameboy.step();
        if render {
            if window.is_open() /*&& !window.is_key_down(Key::Escape)*/ {
                window
                    .update_with_buffer(
                        &gb_with_save_on_drop.gameboy.display.engine.screen,
                        WIDTH,
                        HEIGHT,
                    )
                    .unwrap();
            } else {
                break;
            }

            let keys = window.get_keys();
            gb_with_save_on_drop
                .gameboy
                .set_pressed_keys(get_pressed_keys(keys));

            if gb_with_save_on_drop.gameboy.speaker.is_buffer_full() {
                audio
                    .queue_audio(&gb_with_save_on_drop.gameboy.speaker.samples)
                    .unwrap();
                gb_with_save_on_drop.gameboy.speaker.empty_buffer();
            }
        }
    }
}
