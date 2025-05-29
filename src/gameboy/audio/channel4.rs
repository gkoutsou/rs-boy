use log::{info, trace};

use crate::gameboy::memory_bus::MemoryAccessor;

use super::wave::Wave;

const MAX_ENVELOPE_VOL: f32 = 15.0;
const MAX_LENGTH: u8 = 64;
const AUDIO_STEP_FREQUENCY: u32 = 4194304 / 512;

// TODO do I care about the bits I don't track?
pub(crate) struct Channel4 {
    enabled: bool,

    volume: u8,
    audio_step_counter: u32,
    /// Frame of the audio. 1-8
    audio_step_state: u8,

    length_counter: u8,
    trigger_counter: f32,
    lfsr: u16,
    next_sample: u8,

    // FF20 — NR41: Channel 4 length timer
    // 7	6	| 5	4	3	2	1	0
    //             Initial length timer
    initial_length_timer: u8,

    /// The index used to step the volume / envelope
    env_pace_index: u8,
    // FF21 — NR42: Channel 4 volume & envelope
    // 7	6	5	4	| 	3	|	2	1	0
    // Initial volume	Env dir     Sweep pace
    initial_volume: u8,
    env_dir: bool,
    env_pace: u8,

    // FF22 — NR43: Channel 4 frequency & randomness
    // 7	6	5	4	|		3	| 2	1	0
    //   Clock shift      LFSR width	Clock divider
    clock_shift: u8,
    lfsr_7_width: bool,
    clock_divider: u8,

    // FF23 — NR44: Channel 4 control
    // 7    	|       6	    | 5	4	3	2	1	0
    //  Trigger   Length enable
    length_enabled: bool,
}

impl MemoryAccessor for Channel4 {
    fn get(&self, location: usize) -> u8 {
        match location {
            0xff20 => self.initial_length_timer,
            0xff21 => self.initial_volume << 4 | (self.env_dir as u8) << 3 | self.env_pace,
            0xff22 => self.clock_shift << 4 | (self.lfsr_7_width as u8) << 3 | self.clock_divider,
            0xff23 => 0xff & (self.length_enabled as u8) << 6,
            _ => panic!("missing channel 4 get: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0xff20 => self.initial_length_timer = value & 0x3F,
            0xff21 => {
                self.initial_volume = value >> 4;
                self.env_dir = value & (1 << 3) > 0;
                self.env_pace = value & 0x7;

                // Setting bits 3-7 of this register all to 0 (initial volume = 0, envelope = decreasing)
                // turns the DAC off (and thus, the channel as well)
                if self.initial_volume == 0 && !self.env_dir {
                    self.enabled = false
                }
            }

            0xff22 => {
                self.clock_shift = value >> 4;
                self.lfsr_7_width = value & (1 << 3) > 0;
                self.clock_divider = value & 0x7;
            }

            0xff23 => {
                self.length_enabled = value & (1 << 6) > 0;
                // Writing to bit 7:

                if value & (1 << 7) > 0 {
                    // Ch4 is enabled.
                    self.enabled = true;
                    // If the length timer expired it is reset.
                    if self.length_counter >= MAX_LENGTH {
                        self.length_counter = self.initial_length_timer;
                    }
                    // Envelope timer is reset.
                    self.env_pace_index = 0;
                    // Volume is set to contents of NR42 initial volume.
                    self.volume = self.initial_volume;
                    // LFSR bits are reset.
                    self.lfsr = 0;
                    // todo!()
                }
            }

            _ => panic!("missing channel 4 write: {:#x}", location),
        }
    }
}

impl Wave for Channel4 {
    fn step(&mut self, step: u32) {
        for _ in 0..step {
            self.audio_step_counter += 1; // NOTE: += step if I ever remove the loop..
            if self.audio_step_counter == AUDIO_STEP_FREQUENCY {
                self.audio_step_counter = 0;

                // TODO should this happen once per step?
                if self.length_enabled
                    && self.length_counter < MAX_LENGTH
                    && self.audio_step_state % 2 == 0
                {
                    self.length_counter += 1;
                    if self.length_counter >= MAX_LENGTH {
                        // disable channel if its length timer expiring
                        self.enabled = false;
                    }
                }

                if self.audio_step_state == 7 {
                    self.update_volume();
                }

                self.audio_step_state = (self.audio_step_state + 1) % 8;
            }
        }

        // Every 16 ticks evaluate
        // The frequency at which the LFSR is clocked is 262144 / divider × 2^shift Hz.
        for _ in 0..(step) {
            let divider: f32 = if self.clock_divider > 0 {
                self.clock_divider as f32
            } else {
                0.5
            };
            self.trigger_counter += 1.0;
            // 16 for it fits 16 * 262144 in the application-freq
            if self.trigger_counter >= 16 as f32 * divider * (1 << self.clock_shift) as f32 {
                self.trigger_counter = 0.0;
                self.step_lfsr();
            }
        }
    }

    fn sample(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        // info!("{}", self.volume);
        ((0.5 - self.next_sample as f32) * 2.0) * self.volume as f32 / MAX_ENVELOPE_VOL
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        todo!("reset ch4");
    }
}

impl Channel4 {
    /// The envelope ticks at 64 Hz, and the channel’s envelope will be increased / decreased
    /// every Sweep pace of those ticks. A setting of 0 disables the envelope.
    fn update_volume(&mut self) {
        if self.env_pace == 0 {
            return;
        }
        self.env_pace_index += 1;

        if self.env_pace_index != self.env_pace {
            return;
        }
        self.env_pace_index -= self.env_pace;

        if self.volume == 0 && self.env_dir == false {
            trace!("vol going minus");
            return;
        }
        if self.volume == 15 && self.env_dir == true {
            trace!("vol overloading");
            return;
        }
        if self.env_dir {
            self.volume += 1
        } else {
            self.volume -= 1
        }
    }

    fn step_lfsr(&mut self) {
        //The result of  LFSR0 == LFSR1 is written to bit 15.
        // If “short mode” was selected in NR43, then bit 15 is copied to bit 7 as well.
        // Finally, the entire LFSR is shifted right, and bit 0 selects between 0 and the chosen volume.
        let bit0 = self.lfsr & 1;
        let bit1 = self.lfsr & 2;

        let xnor = bit0 == bit1;
        self.lfsr &= (xnor as u16) << 15;
        if self.lfsr_7_width {
            self.lfsr &= (xnor as u16) << 7;
        }

        self.next_sample = (self.lfsr & 1) as u8;
        self.lfsr = self.lfsr >> 1;
    }
}

impl Default for Channel4 {
    fn default() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            lfsr: 0,

            volume: 0xf,
            audio_step_counter: 0,
            audio_step_state: 0,
            trigger_counter: 0.0,
            next_sample: 0,
            env_pace_index: 0,

            // ff20
            initial_length_timer: 0x3f,

            // ff21
            initial_volume: 0,
            env_dir: false,
            env_pace: 0,

            // ff22
            clock_shift: 0,
            lfsr_7_width: false,
            clock_divider: 0,

            // ff23
            length_enabled: false,
        }
    }
}
