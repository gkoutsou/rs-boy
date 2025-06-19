use log::{info, trace};

use crate::gameboy::memory_bus::MemoryAccessor;

use super::wave::Wave;

const MAX_ENVELOPE_VOL: f32 = 15.0;
const MAX_LENGTH: u16 = 256;
const AUDIO_STEP_FREQUENCY: u32 = 4194304 / 512;
const NUM_WAVE_SAMPLES: usize = 16 * 2;

// TODO do I care about the bits I don't track?
pub(crate) struct Channel3 {
    enabled: bool,

    audio_step_counter: u32,
    /// Frame of the audio. 1-8
    audio_step_state: u8,
    wave_index: u8,
    next_sample: u8,

    // Wave Ram 0xff30-0xff3f
    wave_ram: Vec<u8>,

    // FF1A — NR30: Channel 3 DAC enable
    // 7		| 6	5	4	3	2	1	0
    // DAC off/on
    dac_on: bool,

    length_counter: u16,
    // FF1B — NR31: Channel 3 length timer [write-only]
    // 7	6	5	4	3	2	1	0
    // Initial length timer
    initial_length_timer: u8,

    // FF1C — NR32: Channel 3 output level
    // 7	|	 6			5 	|	4	3	2	1	0
    //         output level
    output_level: u8,

    period_divider: u16,
    // FF1D — NR33: Channel 3 period low [write-only]
    // FF1E — NR34: Channel 3 period high & control
    // 7	    | 6	         | 5 4 3 | 2	1	0
    // Trigger	Length enable		   Period
    length_enabled: bool,
    period: u16,
}

impl MemoryAccessor for Channel3 {
    fn get(&self, location: usize) -> u8 {
        match location {
            0xff1a => (self.dac_on as u8) << 7 | 0x7f,
            0xff1b => {
                // self.initial_length_timer,
                0xff // write-only
            }
            0xff1c => self.output_level << 5 | 0b10011111,
            0xff1d => {
                //(self.period & 0xff) as u8
                0xff // it's write only field
            }
            0xff1e => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                let trigger = 1 << 7;
                let length_enable = (self.length_enabled as u8) << 6;
                // let period = (self.period >> 8) as u8;
                let period = 0x7; // write only

                trigger | length_enable | 0b111000 | period
            }
            0xff30..=0xff3f => self.wave_ram[location - 0xff30],
            _ => panic!("missing channel 4 get: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0xff1a => {
                self.dac_on = value >> 7 > 0;
                if !self.dac_on {
                    // Disabling DAC disables the channel
                    self.enabled = false;
                }
            }
            0xff1b => {
                self.initial_length_timer = value;
                // Writing a byte to NRx1 loads the counter with 64-data (256-data for wave channel). The counter can be reloaded at any time.
                self.length_counter = MAX_LENGTH - self.initial_length_timer as u16;
            }

            0xff1c => self.output_level = (value & 0b01100000) >> 5,

            0xff1d => self.period = (self.period & 0x700) | value as u16,

            0xff1e => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                let trigger = value >> 7 > 0;
                self.length_enabled = value & (1 << 6) > 0;
                self.period = (self.period & 0xff) | ((value as u16 & 7) << 8);

                if trigger {
                    info!("Triggering channel 3");
                    // Channel is enabled.
                    self.enabled = self.dac_on; // TODO verify this

                    // TODO rest
                    // If the length timer expired it is reset.
                    if self.length_counter == 0 {
                        self.length_counter = MAX_LENGTH;
                    }
                    // The period divider is set to the contents of NR33 and NR34.
                    self.period_divider = self.period;
                    // TODO Volume is set to contents of NR32 initial volume.
                    // TODO Wave RAM index is reset, but its not refilled.
                    self.wave_index = 0;
                }
            }

            0xff30..=0xff3f => self.wave_ram[location - 0xff30] = value,

            _ => panic!("missing channel 4 write: {:#x}", location),
        }
    }
}

impl Wave for Channel3 {
    fn step(&mut self, step: u32) {
        for _ in 0..step {
            self.audio_step_counter += 1; // NOTE: += step if I ever remove the loop..
            if self.audio_step_counter == AUDIO_STEP_FREQUENCY {
                self.audio_step_counter = 0;

                // TODO should this happen once per step?
                if self.length_enabled && self.length_counter > 0 && self.audio_step_state % 2 == 0
                {
                    self.length_counter -= 1;
                    if self.length_counter == 0 {
                        // disable channel if its length timer expiring
                        self.enabled = false;
                        info!("Disabling ch3 due to length");
                        // Disable ff14
                    }
                }

                self.audio_step_state = (self.audio_step_state + 1) % 8;
            }
        }

        // Double frequency than the square ch
        for _ in 0..(step / 2) {
            self.period_divider += 1;
            // todo!("cross-check this");
            if self.period_divider == 2048 {
                trace!("Changing wave_index: {}", self.wave_index);
                self.period_divider = self.period;

                self.wave_index = (self.wave_index + 1) % NUM_WAVE_SAMPLES as u8;
                let byte_pos = self.wave_index / 2;
                let upper_nimble = self.wave_index % 2 == 0;
                let byte = self.wave_ram[byte_pos as usize];

                self.next_sample = if upper_nimble { byte >> 4 } else { byte & 0xF }
            }
        }
    }

    fn sample(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        let sample = self.next_sample << self.get_volume_shift();
        (0.5 - (sample as f32) / MAX_ENVELOPE_VOL) * 2.0
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        info!("TODO reset ch3 should reset internals");
        // Since dac_on becomes false, we disable the channel. TODO crosscheck
        self.enabled = false;

        // FF1A — NR30: Channel 3 DAC enable
        self.dac_on = false;

        // FF1B — NR31: Channel 3 length timer [write-only]
        self.initial_length_timer = 0;

        // FF1C — NR32: Channel 3 output level
        self.output_level = 0;

        // FF1D — NR33: Channel 3 period low [write-only]
        // FF1E — NR34: Channel 3 period high & control
        self.length_enabled = false;
        self.period = 0;

        // TODO verify:
        self.audio_step_counter = 0;
        self.length_counter = MAX_LENGTH - self.initial_length_timer as u16;
        self.period_divider = self.period;
    }
}

impl Channel3 {
    fn get_volume_shift(&self) -> u8 {
        match self.output_level {
            0 => 4,
            1 => 0,
            2 => 1,
            3 => 2,
            _ => {
                panic!("unknown output level: {}", self.output_level)
            }
        }
    }
}

impl Default for Channel3 {
    fn default() -> Self {
        let period = 0xff | (0x7 << 8);
        Self {
            enabled: false,
            audio_step_state: 0,
            audio_step_counter: 0,
            wave_index: 0,
            next_sample: 0,
            length_counter: MAX_LENGTH - 0xff,
            period_divider: period,

            wave_ram: vec![0; 16],
            dac_on: false, // 0x7f
            initial_length_timer: 0xff,
            output_level: 0, // 0x9f
            length_enabled: false,
            period: period,
        }
    }
}
