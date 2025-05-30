use log::{info, trace};

use crate::gameboy::{memory_bus::MemoryAccessor, registers::set_flag};

use super::wave::Wave;

const MAX_ENVELOPE_VOL: f32 = 15.0;
const MAX_LENGTH: u8 = 64;
const AUDIO_STEP_FREQUENCY: u32 = 4194304 / 512;

// TODO do I care about the bits I don't track?
pub(crate) struct Channel3 {
    enabled: bool,

    // Wave Ram 0xff30-0xff3f
    wave_ram: Vec<u8>,

    // FF1A — NR30: Channel 3 DAC enable
    // 7		| 6	5	4	3	2	1	0
    // DAC off/on
    dac_on: bool,

    // FF1B — NR31: Channel 3 length timer [write-only]
    // 7	6	5	4	3	2	1	0
    // Initial length timer
    initial_length_timer: u8,

    // FF1C — NR32: Channel 3 output level
    // 7	|	 6			5 	|	4	3	2	1	0
    //         output level
    output_level: u8,

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
            0xff1b => self.initial_length_timer = value,

            0xff1c => self.output_level = (value & 0b01100000) >> 5,

            0xff1e => self.period = (self.period & 0x700) | value as u16,

            0xff1d => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                let trigger = value >> 7 > 0;
                self.length_enabled = value & (1 << 6) > 0;
                self.period = (self.period & 0xff) | ((value as u16 & 7) << 8);

                if trigger {
                    // Channel is enabled.
                    self.enabled = true;
                    // TODO rest
                }
                //     // The period divider is set to the contents of NR13 and NR14.
                //     self.period_divider = self.period;
                //     // Volume is set to contents of NR12 initial volume.
                //     self.volume = self.initial_volume;
                //     // Envelope timer is reset.
                //     self.env_pace_index = 0;
                //     // If length timer expired it is reset.
                //     if self.length_counter >= MAX_LENGTH {
                //         self.length_counter = self.initial_length_timer;
                //     }
                //     // Sweep does several things.
                //     if self.has_sweep {
                //         // CH1 period value is copied to the “shadow register”.
                //         self.sweep_shadow_period = self.period;
                //         // The “sweep timer” is reset.
                //         self.sweep_pace_remaining = self.sweep_pace;
                //         // The “enabled flag” is set if either the sweep pace or individual step are non-zero, cleared otherwise.
                //         self.sweep_enabled = self.sweep_pace != 0 || self.individual_step != 0;
                //         // If the individual step is non-zero, frequency calculation and overflow check are performed immediately.
                //         if self.individual_step != 0 && self.calculate_new_frequency() >= 2048 {
                //             self.enabled = false;
                //             warn!("I think this is how it should be.. but better check");
                //         }
                //     }
                // }
            }

            0xff30..=0xff3f => self.wave_ram[location - 0xff30] = value,

            _ => panic!("missing channel 4 write: {:#x}", location),
        }
    }
}

impl Wave for Channel3 {
    fn step(&mut self, step: u32) {
        ()
    }

    fn sample(&self) -> f32 {
        0.0
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
    }
}

impl Channel3 {}

impl Default for Channel3 {
    fn default() -> Self {
        let period = 0xff | (0x7 << 8);
        Self {
            enabled: false,

            wave_ram: vec![0; 16],
            dac_on: false, // 0x7f
            initial_length_timer: 0xff,
            output_level: 0, // 0x9f
            length_enabled: false,
            period: period,
        }
    }
}
