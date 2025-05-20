use channel1::Channel1;
use channel2::Channel2;
use channel4::Channel4;
use log::{debug, trace};
use target::SDL2Output;
use wave::Wave;

use super::memory_bus::MemoryAccessor;
mod channel1;
mod channel2;
mod channel4;
mod pulse_wave;
mod target;
mod wave;

const HW_FREQUENCY: i32 = 4194304;
const AUDIO_SAMPLE_RATE: i32 = 44100;
const SAMPLING_FREQUENCY: u32 = HW_FREQUENCY as u32 / AUDIO_SAMPLE_RATE as u32; // 95

const MAX_VOL: f32 = 7.0;
const CHANNELS: f32 = 2.0; // TODO 4 channels at the end
const VOLUME_ADJUST: f32 = 1000.0; // TODO just a random thingy. Find proper value

pub struct Speaker {
    output_target: Box<dyn target::AudioTarget>,
    clock: u32,
    channel1: Channel1,
    channel2: Channel2,
    channel4: Channel4,
    /// FF26 — NR52: Audio master control
    ///
    /// 7            | 6 5 4 | 3 2 1 0
    ///
    /// Audio on/off |       | CH4 on?	CH3 on?	CH2 on?	CH1 on?
    audio_master: u8, // TODO implement bits 0-3
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
            todo!("should somehow not affect div-counter...");
            return;
        }

        //A “DIV-APU” counter is increased every time DIV’s bit 4 (5 in double-speed mode) goes from 1 to 0,
        // therefore at a frequency of 512 Hz (regardless of whether double-speed is active). Thus, the counter
        // can be made to increase faster by writing to DIV while its relevant bit is set (which clears DIV, and
        // triggers the falling edge).

        //own notes: div is once every 256 dots. So div-apu is once every 8*256?
        todo!("the comment above");

        self.channel1.step(steps);
        self.channel2.step(steps);

        // todo!("Implement sound");
        self.clock += steps;
        if self.clock < SAMPLING_FREQUENCY {
            return;
        }

        self.clock -= SAMPLING_FREQUENCY;
        let mut sample = [0.0, 0.0];

        let (vol_left, vol_right) = self.get_volume();
        let ch1 = self.channel1.sample();
        let (pan_left, pan_right) = self.get_panning(1);

        sample[0] += (ch1 * pan_left as f32 * vol_left as f32) / (MAX_VOL * CHANNELS);
        sample[1] += (ch1 * pan_right as f32 * vol_right as f32) / (MAX_VOL * CHANNELS);
        if sample[0] > 1.0 {
            println!("{},{},{}", pan_left, vol_left, MAX_VOL);
            panic!("BBBBB");
        }
        let ch2 = self.channel2.sample();
        let (pan_left, pan_right) = self.get_panning(2);
        sample[0] += (ch2 * pan_left as f32 * vol_left as f32) / (MAX_VOL * CHANNELS);
        sample[1] += (ch2 * pan_right as f32 * vol_right as f32) / (MAX_VOL * CHANNELS);

        // if sample[0] != 0.0 {
        //     println!("{:?}", sample);
        // }
        if sample[0] > 1.0 {
            println!("{},{},{}", pan_left, vol_left, MAX_VOL);
            panic!("ADASD");
        }

        self.output_target.play(sample[0], sample[1])
    }

    pub fn start(&mut self) {
        self.output_target.start();
    }

    pub fn new() -> Self {
        let audio_target = SDL2Output::new();
        let channel1 = channel1::Channel1::default();
        let channel2 = channel2::Channel2::default();
        let channel4 = channel4::Channel4::default();

        Speaker {
            output_target: Box::new(audio_target),
            channel1,
            channel2,
            channel4,
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
        let right = (self.sound_panning >> (channel - 1)) & 1;
        let left = (self.sound_panning >> (4 + channel - 1)) & 1;
        (left, right)
    }
}

impl MemoryAccessor for Speaker {
    fn get(&self, location: usize) -> u8 {
        debug!("Read speaker memory: {:#x}", location);
        match location {
            0xff10..=0xff14 => self.channel1.get(location),
            0xff15..=0xff19 => self.channel2.get(location),
            0xff1a..=0xff23 => 0, // todo
            0xff24 => self.master_volume,
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
            0xff15..=0xff19 => self.channel2.write(location, value),
            0xff1a..=0xff23 => {
                todo!()
                // print!("{:#b}", value);
                // panic!("{:#x}", location)
            }
            0xff24 => self.master_volume = value,
            0xff25 => self.sound_panning = value,
            0xff26 => {
                self.audio_master = value & 1 << 7;
                if self.audio_master == 0 {
                    // clears all APU registers and makes them read-only until turned back on, except NR52
                    // Turning the APU off, however, does not affect Wave RAM, which can always be read/written,
                    // nor the DIV-APU counter.
                    todo!("implement the above notes.. Commented out code below looks relevant..");
                    todo!("maybe recreate all the channels instead?");
                    todo!("disabling should not affect div-apu counter..");
                    //     self.channel1.reset();
                }
            }

            _ => {
                panic!(
                    "speaker register location write: {:#x} - {:#x}",
                    location, value
                )
            }
        }
    }
}
