use sdl2::audio::{AudioQueue, AudioSpecDesired};

use super::AUDIO_SAMPLE_RATE;

const BUFFER_SIZE: usize = 512;
pub(crate) trait AudioTarget {
    fn play(&mut self, left: f32, right: f32);
    fn start(&self);
}
pub struct SDL2Output {
    queue: AudioQueue<f32>,

    buffer: Vec<f32>,
    index: usize,
}

impl AudioTarget for SDL2Output {
    fn play(&mut self, left: f32, right: f32) {
        self.buffer[self.index] = left;
        self.buffer[self.index + 1] = right;
        self.index += 2;

        // If it's full queue the audio
        if self.index + 2 >= BUFFER_SIZE {
            // while self.queue.size() > 4096 * 4 {
            //     println!("ohnoes");
            // }
            self.index = 0;
            self.queue.queue_audio(&self.buffer).unwrap();
        }
    }

    fn start(&self) {
        self.queue.resume();
    }
}

impl SDL2Output {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(AUDIO_SAMPLE_RATE),
            channels: Some(2), // stereo
            samples: None,     // default sample size
        };

        let queue: AudioQueue<f32> = audio_subsystem.open_queue(None, &desired_spec).unwrap();
        SDL2Output {
            queue,
            buffer: vec![0.0; BUFFER_SIZE],
            index: 0,
        }
    }
}
