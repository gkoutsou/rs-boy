use crate::gameboy::memory_bus::MemoryAccessor;

use super::wave::Wave;

// TODO do I care about the bits I don't track?
pub(crate) struct Channel4 {
    enabled: bool,
    length_counter: u8,
    lfsr: u16,

    // FF20 — NR41: Channel 4 length timer
    // 7	6	| 5	4	3	2	1	0
    //             Initial length timer
    initial_length_timer: u8,

    // FF21 — NR42: Channel 4 volume & envelope
    // 7	6	5	4	| 	3	|	2	1	0
    // Initial volume	Env dir     Sweep pace
    initial_volume: u8,
    env_dir: bool,
    sweep_pace: u8, //todo rename to env_pace

    // FF22 — NR43: Channel 4 frequency & randomness
    // 7	6	5	4	|		3	| 2	1	0
    //   Clock shift      LFSR width	Clock divider
    clock_shift: u8,
    lfsr_7_width: bool,
    clock_divider: u8,

    // FF23 — NR44: Channel 4 control
    // 7    	|       6	    | 5	4	3	2	1	0
    //  Trigger   Length enable
    length_enable: bool,
}

impl MemoryAccessor for Channel4 {
    fn get(&self, location: usize) -> u8 {
        match location {
            0xff20 => self.initial_length_timer,
            0xff21 => self.initial_volume << 4 | (self.env_dir as u8) << 3 | self.sweep_pace,
            0xff22 => self.clock_shift << 4 | (self.lfsr_7_width as u8) << 3 | self.clock_divider,
            0xff23 => 0xff & (self.length_enable as u8) << 6,
            _ => panic!("missing channel 4 get: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0xff20 => self.initial_length_timer = value & 0x3F,
            0xff21 => {
                self.initial_volume = value >> 4;
                self.env_dir = value & (1 << 3) > 0;
                self.sweep_pace = value & 0x7;

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
                self.length_enable = value & (1 << 6) > 0;
                // Writing to bit 7:

                if value & (1 << 7) > 0 {
                    // Ch4 is enabled.
                    // If the length timer expired it is reset.
                    // Envelope timer is reset.
                    // Volume is set to contents of NR42 initial volume.
                    // LFSR bits are reset.
                    self.enabled = true;
                    self.lfsr = 0;
                    todo!()
                }
            }

            _ => panic!("missing channel 4 write: {:#x}", location),
        }
    }
}

impl Wave for Channel4 {
    fn step(&mut self, step: u32) {
        todo!()
        // self.pulse.step(step)
    }

    fn sample(&self) -> f32 {
        todo!();
        2.0
        // self.pulse.sample()
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Channel4 {
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

        self.lfsr = self.lfsr >> 1;
        todo!("sample = 0 or self.volume")
    }
}

impl Default for Channel4 {
    fn default() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            lfsr: 0,

            // ff20
            initial_length_timer: 0x3f,

            // ff21
            initial_volume: 0,
            env_dir: false,
            sweep_pace: 0,

            // ff22
            clock_shift: 0,
            lfsr_7_width: false,
            clock_divider: 0,

            // ff23
            length_enable: false,
        }
    }
}
