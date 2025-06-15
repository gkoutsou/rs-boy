use env_logger::Env;
use io::game_engine::Key as EngineKey;
use lib_rs_boy::gameboy::GameBoy;
use lib_rs_boy::io;
use log::info;
use minifb::{Key, Window};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use std::env;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;
const FRAMERATE: usize = 60;

#[inline]
fn get_pressed_keys(keys: Vec<Key>) -> Vec<EngineKey> {
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
    if !use_speakers {
        todo!("implement me");
    }
    let audio = SDL2Output::new();
    audio.queue.resume();

    loop {
        let render = gb.step();
        if render {
            if window.is_open() && !window.is_key_down(Key::Escape) {
                window
                    .update_with_buffer(&gb.display.engine.screen, WIDTH, HEIGHT)
                    .unwrap();
            } else {
                panic!("window deado")
            }

            let keys = window.get_keys();
            gb.set_pressed_keys(get_pressed_keys(keys));

            if gb.speaker.is_buffer_full() {
                // info!(
                //     "buffer full {}, {}",
                //     gb.speaker.samples.len(),
                //     gb.speaker.samples.capacity()
                // );
                audio.queue.queue_audio(&gb.speaker.samples).unwrap();
                gb.speaker.empty_buffer();
            }
        }
    }
}

pub struct SDL2Output {
    queue: AudioQueue<i16>,
}

// impl io::audio_output::AudioTarget for SDL2Output {
//     fn play(&mut self, left: f32, right: f32) {
//         self.buffer[self.index] = left;
//         self.buffer[self.index + 1] = right;
//         self.index += 2;
//
//         // let mut min: f32 = 1000.0;
//         // let mut max: f32 = 0.0;
//
//         // If it's full queue the audio
//         if self.index >= AUDIO_BUFFER_SIZE {
//             // while self.queue.size() > 4096 * 4 {
//             //     println!("ohnoes");
//             // }
//             self.index = 0;
//             self.queue.queue_audio(&self.buffer).unwrap();
//
//             // for i in self.buffer.iter().step_by(2) {
//             //     if *i > max {
//             //         max = *i;
//             //     }
//             //     if *i < min {
//             //         min = *i;
//             //     }
//             // }
//             // if min < 0.0 || max > 0.0 {
//             //     // todo!("why do I get different min/max?");
//             //     info!("########### min {}, max {}", min, max)
//             // }
//         }
//     }
//
//     fn start(&self) {
//         self.queue.resume();
//     }
// }

impl SDL2Output {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(lib_rs_boy::gameboy::audio::AUDIO_SAMPLE_RATE),
            channels: Some(2), // stereo
            samples: None,
            // samples: Some(4096),
        };

        let queue: AudioQueue<i16> = audio_subsystem.open_queue(None, &desired_spec).unwrap();
        SDL2Output { queue }
    }
}
