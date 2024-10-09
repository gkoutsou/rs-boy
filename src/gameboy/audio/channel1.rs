use crate::gameboy::{memory_bus::MemoryAccessor, registers::operations::Operations};

const MAX_LENGTH: u8 = 64u8;

pub(crate) struct Channel1 {
    enabled: bool,

    length_counter: u8,
    // FF10 — NR10: Channel 1 sweep
    // This register controls CH1’s period sweep functionality.
    // 7	| 6	5 4 | 3	            | 2	1	0
    //        Pace	  Direction	    Individual step
    pace: u8,
    direction: bool,
    individual_step: u8,

    // FF11 — NR11: Channel 1 length timer & duty cycle
    // 7	6	    | 5	4	3	2	1	0
    // Wave duty	Initial length timer
    wave_duty: u8,
    initial_length_timer: u8,

    // FF12 — NR12: Channel 1 volume & envelope
    // 7	6	5	4	| 3	        |2	1	0
    // Initial volume	Env dir     Sweep pace
    initial_volume: u8,
    env_dir: bool,
    sweep_pace: u8,

    // FF13 — NR13: Channel 1 period low [write-only]
    // FF14 — NR14: Channel 1 period high & control
    // 7	    | 6	         | 5 4 3 | 2	1	0
    // Trigger	Length enable		   Period
    length_enabled: bool,
    period: u16,
    trigger: bool,
}

impl Channel1 {
    pub fn step(&mut self, step: u8) {
        if self.length_enabled {
            self.length_counter += step;
            if self.length_counter >= MAX_LENGTH {
                self.enabled = false;
                // Disable ff14
            }
        }
    }
    pub fn sample(&self) -> f32 {
        1.0
    }
}

impl Default for Channel1 {
    fn default() -> Self {
        Self {
            enabled: true,
            length_counter: 0,
            length_enabled: false,
            period: 0xff | (0x7 << 8),
            trigger: true,
            // FF10 - default 0x80 (unused bit 7 set)
            pace: 0,
            direction: false,
            individual_step: 0,
            // FF11 - default 0xbf
            wave_duty: 0b10,
            initial_length_timer: 0x3f,
            // FF12 - default 0xf3
            initial_volume: 0xf,
            env_dir: false,
            sweep_pace: 3,
        }
    }
}

impl MemoryAccessor for Channel1 {
    fn get(&self, location: usize) -> u8 {
        match location {
            0xff10 => {
                // 7	| 6	5 4 | 3	            | 2	1	0
                //        Pace	  Direction	    Individual step
                let step = self.individual_step;
                let direction = (self.direction as u8) << 3;
                let pace = self.pace << 4;

                pace | direction | step
            }

            0xff11 => {
                // 7	6	    | 5	4	3	2	1	0
                // Wave duty	Initial length timer
                let timer = self.initial_length_timer;
                let duty = self.wave_duty << 6;
                timer | duty
            }

            0xff12 => {
                // 7	6	5	4	| 3	        |2	1	0
                // Initial volume	Env dir     Sweep pace
                let pace = self.sweep_pace;
                let dir = (self.env_dir as u8) << 3;
                let volume = self.initial_volume << 4;
                pace | dir | volume
            }

            0xff13 => (self.period & 0xff) as u8,

            0xff14 => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                let trigger = (self.trigger as u8) << 7;
                let length_enable = (self.length_enabled as u8) << 6;
                let period = (self.period >> 8) as u8;

                trigger | length_enable | period
            }

            _ => panic!("missing channel1 get: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0xff10 => {
                // 7	| 6	5 4 | 3	            | 2	1	0
                //        Pace	  Direction	    Individual step
                self.individual_step = value & 0x7;
                self.direction = value & (1 << 3) > 0;
                self.pace = (value >> 4) & 0x7
            }

            0xff11 => {
                // 7	6	    | 5	4	3	2	1	0
                // Wave duty	Initial length timer
                self.initial_length_timer = value & 0b00111111;
                self.wave_duty = value >> 6;
            }

            0xff12 => {
                // 7	6	5	4	| 3	        |2	1	0
                // Initial volume	Env dir     Sweep pace
                self.initial_volume = value >> 4;
                self.env_dir = value & (1 << 3) > 0;
                self.sweep_pace = value & 0x7;
            }
            0xff13 => self.period = (self.period & 0x700) | value as u16,

            0xff14 => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                self.trigger = value >> 7 > 0;
                self.length_enabled = value & (1 << 6) > 0;
                self.period = (self.period & 0xff) | ((value as u16 & 7) << 8);
            }

            _ => panic!("missing channel1 write: {:#x}", location),
        }
    }
}
