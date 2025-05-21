use log::{trace, warn};

use crate::gameboy::memory_bus::MemoryAccessor;

use super::wave::Wave;

const MAX_ENVELOPE_VOL: f32 = 15.0;
const MAX_LENGTH: u8 = 64;
const AUDIO_STEP_FREQUENCY: u32 = 4194304 / 512;
const DUTIES: [[i8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 00 (0x0)
    [0, 0, 0, 0, 0, 0, 1, 1], // 01 (0x1)
    [0, 0, 0, 0, 1, 1, 1, 1], // 10 (0x2)
    [0, 1, 1, 1, 1, 1, 1, 0], // 11 (0x3)
];
// Originally copied fron other emu
// const DUTIES: [[u8; 8]; 4] = [
//     [0, 0, 0, 0, 0, 0, 0, 1], // 00 (0x0)
//     [1, 0, 0, 0, 0, 0, 0, 1], // 01 (0x1)
//     [1, 0, 0, 0, 0, 1, 1, 1], // 10 (0x2)
//     [0, 1, 1, 1, 1, 1, 1, 0], // 11 (0x3)
// ];

pub(crate) struct Pulse {
    enabled: bool,
    pub has_sweep: bool,

    // State of the Channel
    volume: u8,
    audio_step_counter: u32,
    /// Frame of the audio. 1-8
    audio_step_state: u8,

    sweep_pace_index: u8,
    sweep_enabled: bool,
    env_pace_index: u8,
    duty_index: u8,
    current_period: u16,

    length_counter: u8,
    // FF10 — NR10: Channel 1 sweep
    // This register controls CH1’s period sweep functionality.
    // 7	| 6	5 4 | 3	            | 2	1	0
    //        Pace	  Direction	    Individual step
    sweep_pace: u8,
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
    env_pace: u8,

    // FF13 — NR13: Channel 1 period low [write-only]
    // FF14 — NR14: Channel 1 period high & control
    // 7	    | 6	         | 5 4 3 | 2	1	0
    // Trigger	Length enable		   Period
    length_enabled: bool,
    period: u16,
    trigger: bool,
}

impl Wave for Pulse {
    /// 512 Hz timer clocking sweep, envelope and length functions of the channels.
    /// - Length at 256Hz         - or every 2nd run
    /// - Volume Envelope at 64Hz - or every 8th run
    /// - Sweep at 128Hz          - or every 4th
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
                        // Disable ff14
                    }
                }

                if self.audio_step_state == 7 {
                    self.update_volume();
                }

                if self.has_sweep && self.audio_step_state % 4 == 3 {
                    self.update_sweep()
                }
                self.audio_step_state = (self.audio_step_state + 1) % 8;
            }
        }

        // Should clock every 4th dot
        // The period divider of pulse and wave channels is an up counter. Each time it is clocked, its
        // value increases by 1; when it overflows (being clocked when it’s already 2047, or $7FF), its
        // value is set from the contents of NR13 and NR14.
        for _ in 0..(step / 4) {
            self.current_period += 1;
            if self.current_period == 2048 {
                trace!("Changing duty_index: {}", self.duty_index);
                self.current_period = self.period;

                // the “duty step” increments at the channel’s sample rate, which is 8 times the channel’s frequency).
                self.duty_index = (self.duty_index + 1) % 8;
            }
        }
    }

    fn sample(&self) -> f32 {
        if !self.enabled {
            // println!("samping with disabled channel");
            return 0.0;
        }
        if DUTIES[self.wave_duty as usize][self.duty_index as usize] as f32 * self.volume as f32
            == 0.0
        {
            trace!("{}-{}-{}", self.wave_duty, self.duty_index, self.volume)
        }
        DUTIES[self.wave_duty as usize][self.duty_index as usize] as f32 * self.volume as f32
            / MAX_ENVELOPE_VOL // todo magic volume adjuster
    }
}
impl Pulse {
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

    fn update_sweep(&mut self) {
        // Note 1
        // On each sweep iteration, the period in NR13 and NR14 is modified and written back.
        // In addition mode, if the period value would overflow (i.e. is strictly more than $7FF),
        // the channel is turned off instead. This occurs even if sweep iterations are disabled by
        // the pace being 0.

        // Note 2
        // When the “sweep timer” is clocked if the “enabled flag” is set and the sweep pace is not zero,
        // a new frequency is calculated and the overflow check is performed. If the new frequency is 2047
        // or less and the individual step is not zero, this new frequency is written back to the
        // “shadow register” and CH1 frequency in NR13 and NR14, then frequency calculation and overflow check
        // are run again immediately using this new value, but this second new frequency is not written back.

        // TODO
        // CH1 frequency can be modified via NR13 and NR14 while sweep is active, but the “shadow register”
        // won’t be affected so the next time the “sweep timer” updates the channel’s frequency, this modification
        // will be lost. This can be avoided by triggering the channel.

        if self.sweep_pace == 0 {
            if self.period >= 2048 {
                self.enabled = false;
            }
            return;
        }

        if self.individual_step == 0 {
            println!("how to handle step being zero?");
            return;
        }

        self.sweep_pace_index += 1;
        if self.sweep_pace_index != self.sweep_pace {
            return;
        }
        self.sweep_pace_index -= self.sweep_pace;

        if !self.sweep_enabled {
            return;
        }

        let new_period = if self.sweep_direction {
            self.period + (self.period >> self.individual_step)
        } else {
            // TODO this might underflow
            self.period - (self.period >> self.individual_step)
        };

        if new_period < 2048 {
            self.period = new_period;
        } else {
            // Frequency sweep overflowing the frequency disables the channel
            self.enabled = false;
        }
    }

    pub fn reset(&mut self) {
        self.audio_step_counter = 0;
        self.duty_index = 0;
        // TODO "The “duty step” counter cannot be reset, except by turning the APU off, which sets both back to 0.
        // Was this reset meant to be used for the APU-turning off, and another should be used for the Trigger?
        self.current_period = self.period;
        self.sweep_pace_index = 0;
        self.env_pace_index = 0;
        self.volume = self.initial_volume;
        self.length_counter = self.initial_length_timer;
    }
}

impl Default for Pulse {
    fn default() -> Self {
        let period = 0xff | (0x7 << 8);
        Self {
            enabled: false,
            has_sweep: false,
            sweep_enabled: true,
            volume: 0xf, // todo is this right?
            current_period: period,
            duty_index: 0,
            sweep_pace_index: 0,
            env_pace_index: 0,
            audio_step_state: 0,
            audio_step_counter: 0,
            length_counter: 0x3f,
            length_enabled: false,
            period: period,
            trigger: true,
            // FF10 - default 0x80 (unused bit 7 set)
            sweep_pace: 0,
            sweep_direction: false,
            individual_step: 0,
            // FF11 - default 0xbf
            wave_duty: 0b10,
            initial_length_timer: 0x3f,
            // FF12 - default 0xf3
            initial_volume: 0xf,
            env_dir: false,
            env_pace: 3,
        }
    }
}

impl MemoryAccessor for Pulse {
    fn get(&self, location: usize) -> u8 {
        match location {
            0x0 => {
                // 7	| 6	5 4 | 3	            | 2	1	0
                //        Pace	  Direction	    Individual step
                let step = self.individual_step;
                let direction = (self.sweep_direction as u8) << 3;
                let pace = self.sweep_pace << 4;

                pace | direction | step
            }

            0x1 => {
                // 7	6	    | 5	4	3	2	1	0
                // Wave duty	Initial length timer
                let timer = self.initial_length_timer;
                let duty = self.wave_duty << 6;
                timer | duty
            }

            0x2 => {
                // 7	6	5	4	| 3	        |2	1	0
                // Initial volume	Env dir     Sweep pace
                let pace = self.env_pace;
                let dir = (self.env_dir as u8) << 3;
                let volume = self.initial_volume << 4;
                pace | dir | volume
            }

            0x3 => (self.period & 0xff) as u8,

            0x4 => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                let trigger = (self.trigger as u8) << 7;
                let length_enable = (self.length_enabled as u8) << 6;
                let period = (self.period >> 8) as u8;

                trigger | length_enable | period
            }

            _ => panic!("missing pulse_wave get: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        match location {
            0x0 => {
                // 7	| 6	5 4 | 3	            | 2	1	0
                //        Pace	  Direction	    Individual step
                self.individual_step = value & 0x7;
                self.sweep_direction = value & (1 << 3) > 0;
                // Note that the value written to this field is not re-read by the hardware until a
                // sweep iteration completes, or the channel is (re)triggered.
                // However, if 0 is written to this field, then iterations are instantly disabled (but see below), and it will be reloaded as soon as it’s set to something else.
                // TODO this needs to not affect current iterations!
                self.sweep_pace = (value >> 4) & 0x7
            }

            0x1 => {
                // 7	6	    | 5	4	3	2	1	0
                // Wave duty	Initial length timer
                self.initial_length_timer = value & 0b00111111;
                self.wave_duty = value >> 6;
            }

            0x2 => {
                // 7	6	5	4	| 3	        |2	1	0
                // Initial volume	Env dir     Sweep pace
                self.initial_volume = value >> 4;
                self.env_dir = value & (1 << 3) > 0;
                self.env_pace = value & 0x7;

                // Setting bits 3-7 of this register all to 0 (initial volume = 0, envelope = decreasing)
                // turns the DAC off (and thus, the channel as well)
                if self.initial_volume == 0 && !self.env_dir {
                    self.enabled = false
                }
            }
            0x3 => self.period = (self.period & 0x700) | value as u16,

            0x4 => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                self.trigger = value >> 7 > 0;
                self.length_enabled = value & (1 << 6) > 0;
                self.period = (self.period & 0xff) | ((value as u16 & 7) << 8);
                if self.length_enabled {
                    // todo!("should this be 'If length timer expired it is reset.'");
                    // TODO "todo 2.. why do I reset it here? sounds wrong"
                    self.length_counter = self.initial_length_timer;
                }
                if self.trigger {
                    // Channel is enabled.
                    self.enabled = true;
                    // The period divider is set to the contents of NR13 and NR14.
                    self.current_period = self.period;
                    // Volume is set to contents of NR12 initial volume.
                    self.volume = self.initial_volume;
                    // Envelope timer is reset.
                    self.env_pace_index = 0;
                    // If length timer expired it is reset.
                    if self.length_counter >= MAX_LENGTH {
                        self.length_counter = self.initial_length_timer;
                    }
                    // Sweep does several things.
                    if self.has_sweep {
                        //During a trigger event, several things occur:
                        // CH1 period value is copied to the “shadow register”.
                        // The “sweep timer” is reset.
                        self.sweep_pace_index = 0;
                        // The “enabled flag” is set if either the sweep pace or individual step are non-zero, cleared otherwise.
                        self.sweep_enabled = self.sweep_pace != 0 || self.individual_step != 0;
                        // todo!("enable the above; breaks sound")
                        if !self.sweep_enabled {
                            warn!(
                                "########################  {} - {}",
                                self.sweep_pace, self.individual_step
                            )
                        }
                        // If the individual step is non-zero, frequency calculation and overflow check are performed immediately.
                    }
                }
            }

            _ => panic!("missing pulse_wave write: {:#x}", location),
        }
    }
}
