use crate::gameboy::{memory_bus::MemoryAccessor, registers::operations::Operations};

const MAX_LENGTH: u32 = 64;
const AUDIO_STEP_FREQUENCY: u32 = 4194304 / 512;

pub(crate) struct Channel1 {
    enabled: bool,

    // State of the Channel
    volume: u8,
    audio_step_counter: u32,
    /// Frame of the audio. 1-8
    audio_step_state: u8,

    length_counter: u32,
    // FF10 — NR10: Channel 1 sweep
    // This register controls CH1’s period sweep functionality.
    // 7	| 6	5 4 | 3	            | 2	1	0
    //        Pace	  Direction	    Individual step
    pace: u8,
    sweep_direction: bool,
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
    /// 512 Hz timer clocking sweep, envelope and length functions of the channels.
    /// - Length at 256Hz
    /// - Volume Envelope at 64Hz
    /// - Sweep at 128Hz
    pub fn step(&mut self, step: u32) {
        self.audio_step_counter += step;
        if self.audio_step_counter < AUDIO_STEP_FREQUENCY {
            return;
        }

        self.audio_step_counter -= AUDIO_STEP_FREQUENCY;

        if self.length_enabled && self.audio_step_state % 2 == 0 {
            self.length_counter += 1;
            if self.length_counter >= MAX_LENGTH {
                self.enabled = false;
                // Disable ff14
            }
        }

        if self.audio_step_state == 7 {
            self.update_volume();
        }

        if self.audio_step_state % 4 == 3 {
            // TODO every 4th sweep
        }

        self.audio_step_state = (self.audio_step_state + 1) % 8;
    }

    pub fn sample(&self) -> f32 {
        if !self.enabled {
            println!("samping with disabled channel");
            return 0.0;
        }
        1.0 * self.volume as f32
    }

    fn update_volume(&mut self) {
        if self.volume == 0 && self.env_dir == false {
            println!("vol going minus");
            return;
        }
        if self.volume == 15 && self.env_dir == true {
            println!("vol overloading");
            return;
        }
        if self.env_dir {
            self.volume += 1
        } else {
            self.volume -= 1
        }
    }
}

impl Default for Channel1 {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0xf, // todo is this right?
            audio_step_state: 0,
            audio_step_counter: 0,
            length_counter: 0,
            length_enabled: false,
            period: 0xff | (0x7 << 8),
            trigger: true,
            // FF10 - default 0x80 (unused bit 7 set)
            pace: 0,
            sweep_direction: false,
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
                let direction = (self.sweep_direction as u8) << 3;
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
                self.sweep_direction = value & (1 << 3) > 0;
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

                // Setting bits 3-7 of this register all to 0 (initial volume = 0, envelope = decreasing)
                // turns the DAC off (and thus, the channel as well)
                if self.initial_volume == 0 && !self.env_dir {
                    self.enabled = false
                }
            }
            0xff13 => self.period = (self.period & 0x700) | value as u16,

            0xff14 => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                self.trigger = value >> 7 > 0;
                self.length_enabled = value & (1 << 6) > 0;
                self.period = (self.period & 0xff) | ((value as u16 & 7) << 8);
                if self.trigger {
                    self.enabled = true;
                    println!("handle triggering channel1")
                }
            }

            _ => panic!("missing channel1 write: {:#x}", location),
        }
    }
}
