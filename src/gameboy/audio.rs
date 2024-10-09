use channel1::Channel1;
use log::{debug, trace};
use sdl2::audio::{AudioCallback, AudioDevice, AudioQueue, AudioSpecDesired};

use super::memory_bus::MemoryAccessor;
mod channel1;

const SAMPLING_FREQUENCY: u32 = 95; // 4.194.304 / 44100

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub struct Speaker {
    // device: AudioDevice<SquareWave>,
    queue: AudioQueue<f32>,
    clock: u32,
    channel1: Channel1,
    /// FF26 — NR52: Audio master control
    ///
    /// 7            | 6 5 4 | 3 2 1 0
    ///
    /// Audio on/off |       | CH4 on?	CH3 on?	CH2 on?	CH1 on?
    audio_master: u8,
    /// FF25 — NR51: Sound panning
    ///
    /// 7	6	5	4	3	2	1	0
    ///
    /// CH4 left	CH3 left	CH2 left	CH1 left	CH4 right	CH3 right	CH2 right	CH1 right
    sound_panning: u8,
    /// FF24 — NR50: Master volume & VIN panning
    ///
    /// 7            | 6 5 4	   | 3         | 2	1 0
    ///
    /// VIN left     | Left volume | VIN right | Right volume
    master_volume: u8,
}

impl Speaker {
    pub fn step(&mut self, steps: u32) {
        if !self.is_audio_enabled() {
            return;
        }
        // todo!("Implement sound");
        self.clock += steps;
        if self.clock < SAMPLING_FREQUENCY {
            return;
        }

        self.clock -= SAMPLING_FREQUENCY;

        let (vol_left, vol_right) = self.get_volume();
        let ch1 = self.channel1.sample();
        let (pan_left, pan_right) = self.get_panning(1);

        let test = [
            (ch1 * pan_left as f32 * vol_left as f32),
            (ch1 * pan_right as f32 * vol_right as f32),
        ];

        println!("{:?}", test);

        self.queue.queue_audio(&test).unwrap();
    }

    pub fn start(&mut self) {
        // self.device.resume();
        self.queue.resume();
    }

    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(2), // stereo
            samples: None,     // default sample size
        };

        // let device = audio_subsystem
        //     .open_playback(None, &desired_spec, |spec| {
        //         // initialize the audio callback
        //         SquareWave {
        //             phase_inc: 440.0 / spec.freq as f32,
        //             phase: 0.0,
        //             volume: 0.25,
        //         }
        //     })
        //     .unwrap();

        let queue: AudioQueue<f32> = audio_subsystem.open_queue(None, &desired_spec).unwrap();

        let channel1 = channel1::Channel1::default();

        Speaker {
            queue,
            channel1,
            clock: 0,
            master_volume: 0x77,
            sound_panning: 0xf3,
            audio_master: 0xf1,
        }
    }

    fn is_audio_enabled(&self) -> bool {
        return self.audio_master & (1 << 7) > 0;
    }

    fn get_volume(&self) -> (u8, u8) {
        let left = (self.master_volume & (7 << 4)) >> 4;
        let right = self.master_volume & 7;
        (left, right)
    }

    fn get_panning(&self, channel: u8) -> (u8, u8) {
        let right = self.sound_panning & (1 << (channel - 1));
        let left = self.sound_panning & (1 << (4 + channel - 1));
        (left, right)
    }
}

impl MemoryAccessor for Speaker {
    fn get(&self, location: usize) -> u8 {
        debug!("Read speaker memory: {:#x}", location);
        match location {
            0xff10..=0xff14 => self.channel1.get(location),
            0xff15..=0xff23 => 0, // todo
            0xdd24 => self.master_volume,
            0xff25 => self.sound_panning,
            0xff26 => self.audio_master, // TODO low bits are read-only
            _ => panic!("speaker register location read: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        trace!(
            "Writting to speaker Register: {:#x}: {:#b}",
            location,
            value
        );
        match location {
            0xff10..=0xff14 => self.channel1.write(location, value),
            0xff15..=0xff23 => {
                // print!("{:#b}", value);
                // panic!("{:#x}", location)
            }
            0xff24 => self.master_volume = value,
            0xff25 => self.sound_panning = value,
            0xff26 => self.audio_master = value & 1 << 7,

            _ => {
                panic!(
                    "speaker register location write: {:#x} - {:#x}",
                    location, value
                )
            }
        }
    }
}
