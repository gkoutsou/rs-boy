use channel1::Channel1;
use channel2::Channel2;
use channel3::Channel3;
use channel4::Channel4;
use log::{debug, info};
use wave::Wave;

use super::memory_bus::MemoryAccessor;
mod channel1;
mod channel2;
mod channel3;
mod channel4;
mod pulse_wave;
mod wave;

const HW_FREQUENCY: i32 = 4194304;
pub const AUDIO_SAMPLE_RATE: i32 = 44100;
const FRAMERATE: f64 = 60.0; // 59.7275; //TODO doesn't really fit here
const SAMPLE_COUNT: usize = (AUDIO_SAMPLE_RATE as f64 / FRAMERATE as f64 * 2.0) as usize;
const SAMPLING_FREQUENCY: u32 = HW_FREQUENCY as u32 / AUDIO_SAMPLE_RATE as u32; // 95

const VOL_DIVIDER: f32 = 25.0; // Used to lower the max volume

pub struct Speaker {
    pub samples: Vec<i16>,
    clock: u32,
    channel1: Channel1,
    channel2: Channel2,
    channel3: Channel3,
    channel4: Channel4,
    /// FF26 — NR52: Audio master control
    ///
    /// 7            | 6 5 4 | 3 2 1 0
    ///
    /// Audio on/off |       | CH4 on?	CH3 on?	CH2 on?	CH1 on?
    audio_master: bool,
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

    // Only for debugging purposes
    debug_channel_1_enabled: bool,
    debug_channel_2_enabled: bool,
    debug_channel_3_enabled: bool,
    debug_channel_4_enabled: bool,
}

impl Speaker {
    pub fn step(&mut self, steps: u32) {
        if !self.is_audio_enabled() {
            // todo!("should somehow not affect div-counter...");
            return;
        }

        // A “DIV-APU” counter is increased every time DIV’s bit 4 (5 in double-speed mode) goes from 1 to 0,
        // therefore at a frequency of 512 Hz (regardless of whether double-speed is active). Thus, the counter
        // can be made to increase faster by writing to DIV while its relevant bit is set (which clears DIV, and
        // triggers the falling edge).
        // TODO clearing DIV should also affect this.. sigh..

        self.channel1.step(steps);
        self.channel2.step(steps);
        self.channel3.step(steps);
        self.channel4.step(steps);

        self.clock += steps;
        if self.clock < SAMPLING_FREQUENCY {
            return;
        }

        self.clock -= SAMPLING_FREQUENCY;
        let mut sample = [0.0, 0.0];

        if self.debug_channel_1_enabled {
            let ch1 = self.channel1.sample();
            let (pan_left, pan_right) = self.get_panning(1);
            sample[0] += ch1 * pan_left as f32;
            sample[1] += ch1 * pan_right as f32;
        }

        if self.debug_channel_2_enabled {
            let ch2 = self.channel2.sample();
            let (pan_left, pan_right) = self.get_panning(2);
            sample[0] += ch2 * pan_left as f32;
            sample[1] += ch2 * pan_right as f32;
        }

        if self.debug_channel_3_enabled {
            let ch3 = self.channel3.sample();
            let (pan_left, pan_right) = self.get_panning(3);
            sample[0] += ch3 * pan_left as f32;
            sample[1] += ch3 * pan_right as f32;
        }

        if self.debug_channel_4_enabled {
            let ch4 = self.channel4.sample();
            let (pan_left, pan_right) = self.get_panning(4);
            sample[0] += ch4 * pan_left as f32;
            sample[1] += ch4 * pan_right as f32;
        }

        let (vol_left, vol_right) = self.get_volume();
        let left = sample[0] * vol_left / VOL_DIVIDER;
        let right = sample[1] * vol_right / VOL_DIVIDER;

        self.samples.push((left * 32767.0).round() as i16); // TODO temp conversion
        self.samples.push((right * 32767.0).round() as i16);
    }

    pub fn set_channel_state(&mut self, channel: usize, enabled: bool) {
        match channel {
            1 => self.debug_channel_1_enabled = enabled,
            2 => self.debug_channel_2_enabled = enabled,
            3 => self.debug_channel_3_enabled = enabled,
            4 => self.debug_channel_4_enabled = enabled,
            _ => panic!("Unsupported channel {}", channel),
        }
    }

    pub fn get_samples(&self) -> &Vec<i16> {
        &self.samples
    }

    pub fn empty_buffer(&mut self) {
        // TODO ensure this is done ok
        self.samples.clear()
    }

    pub fn is_buffer_full(&self) -> bool {
        self.samples.len() >= SAMPLE_COUNT
    }

    pub fn new() -> Self {
        let channel1 = Channel1::default();
        let channel2 = Channel2::default();
        let channel3 = Channel3::default();
        let channel4 = Channel4::default();

        Speaker {
            samples: Vec::with_capacity(SAMPLE_COUNT),
            channel1,
            channel2,
            channel3,
            channel4,
            clock: 0,
            master_volume: 0x77,
            sound_panning: 0xf3,
            audio_master: true,

            debug_channel_1_enabled: true,
            debug_channel_2_enabled: true,
            debug_channel_3_enabled: true,
            debug_channel_4_enabled: true,
        }
    }

    fn is_audio_enabled(&self) -> bool {
        return self.audio_master;
    }

    /// Returns the scaling done for the left and right channels.
    ///
    /// A value of 0 is treated as a volume of 1 (very quiet), and a value of 7 is treated as a volume of 8
    /// (no volume reduction). Importantly, the amplifier never mutes a non-silent input.
    fn get_volume(&self) -> (f32, f32) {
        let left = (self.master_volume & (7 << 4)) >> 4;
        let right = self.master_volume & 7;
        ((left + 1) as f32 / 8.0, (right + 1) as f32 / 8.0)
    }

    fn get_panning(&self, channel: u8) -> (u8, u8) {
        let right = (self.sound_panning >> (channel - 1)) & 1;
        let left = (self.sound_panning >> (4 + channel - 1)) & 1;
        (left, right)
    }
}

impl MemoryAccessor for Speaker {
    fn get(&self, location: usize) -> u8 {
        // info!("Read speaker memory: {:#x}", location);
        match location {
            0xff10..=0xff14 => self.channel1.get(location),
            0xff15..=0xff19 => self.channel2.get(location),
            0xff1a..=0xff1e => self.channel3.get(location),
            0xff1f..=0xff23 => self.channel4.get(location),
            0xff24 => self.master_volume,
            0xff25 => self.sound_panning,
            0xff26 => {
                let ch1 = self.channel1.is_enabled() as u8;
                let ch2 = (self.channel2.is_enabled() as u8) << 1;
                let ch3 = (self.channel3.is_enabled() as u8) << 2;
                let ch4 = (self.channel4.is_enabled() as u8) << 3;
                // info!(
                //     "{:#b}",
                //     (self.audio_master as u8) << 7 | 0x70 | ch1 | ch2 | ch3 | ch4
                // );
                (self.audio_master as u8) << 7 | 0x70 | ch1 | ch2 | ch3 | ch4
            }
            0xff27..=0xff2f => 0xff, // Unused area
            0xff30..=0xff3f => self.channel3.get(location),
            _ => panic!("speaker register location read: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        // info!("Writing to speaker Register: {:#x}: {:#b}", location, value);

        //
        if !self.is_audio_enabled() && (location != 0xff26 || (0xff30..=0xff3f).contains(&location))
        {
            // TODO allow write to NRx1 in monochrome
            // makes them read-only until turned back on, except NR52
            // however, does not affect Wave RAM, which can always be read/written,
            // nor the DIV-APU counter.
            return;
        }
        match location {
            0xff10..=0xff14 => self.channel1.write(location, value),
            0xff15..=0xff19 => self.channel2.write(location, value),
            0xff1a..=0xff1e => self.channel3.write(location, value),
            0xff1f..=0xff23 => self.channel4.write(location, value),
            0xff24 => self.master_volume = value,
            0xff25 => self.sound_panning = value,
            0xff26 => {
                let old_state = self.audio_master;
                self.audio_master = value >> 7 > 0;
                let enabling_sound = !old_state && self.is_audio_enabled();

                debug!("PowerOff {}", self.audio_master);
                if !self.is_audio_enabled() {
                    self.channel1.reset();
                    self.channel2.reset();
                    self.channel3.reset();
                    self.channel4.reset();

                    self.sound_panning = 0;
                    self.master_volume = 0;
                    // Turning the APU off, however, does not affect the DIV-APU counter.
                    // todo!("disabling should not affect div-apu counter..");
                }

                if enabling_sound {
                    info!("Enabling_sound");
                    self.channel1.reset_frame();
                    self.channel2.reset_frame();
                    self.channel3.reset_frame();
                    self.channel4.reset_frame();
                }
            }
            0xff27..=0xff2f => (), // Unused area
            0xff30..=0xff3f => self.channel3.write(location, value),

            _ => {
                panic!(
                    "speaker register location write: {:#x} - {:#x}",
                    location, value
                )
            }
        }
    }
}
