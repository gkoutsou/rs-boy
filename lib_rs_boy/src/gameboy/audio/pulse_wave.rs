use super::wave::Wave;
use crate::gameboy::memory_bus::MemoryAccessor;
use log::{info, trace};

const MAX_ENVELOPE_VOL: f32 = 15.0;
const MAX_LENGTH: u8 = 64;
const AUDIO_STEP_FREQUENCY: u32 = 4194304 / 512;
const DUTIES: [[i8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 00 (0x0)
    [1, 0, 0, 0, 0, 0, 0, 1], // 01 (0x1)
    [1, 0, 0, 0, 0, 1, 1, 1], // 10 (0x2)
    [0, 1, 1, 1, 1, 1, 1, 0], // 11 (0x3)
];

pub(crate) struct Pulse {
    enabled: bool,
    pub has_sweep: bool,

    // State of the Channel
    volume: u8,
    audio_step_counter: u32,
    /// Frame of the audio. 1-8
    audio_step_state: u8,

    duty_index: u8,
    /// The actual timer that ends up updating the duty_index
    period_divider: u16,

    length_counter: u8,

    sweep_enabled: bool,
    sweep_pace_remaining: u8,
    /// Shadow register so that period changes don't affect mid-sweep
    sweep_shadow_period: u16,
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

    /// The index used to step the volume / envelope
    env_pace_index: u8,
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

                if self.length_enabled && self.length_counter > 0 && self.audio_step_state % 2 == 0
                {
                    self.length_counter -= 1;
                    if self.length_counter == 0 {
                        // disable channel if its length timer expiring
                        self.enabled = false;
                    }
                }

                if self.audio_step_state == 7 {
                    self.update_volume();
                }

                if self.has_sweep && self.audio_step_state % 4 == 2 {
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
            self.period_divider += 1;
            if self.period_divider == 2048 {
                trace!("Changing duty_index: {}", self.duty_index);
                self.period_divider = self.period;

                // the “duty step” increments at the channel’s sample rate, which is 8 times the channel’s frequency).
                self.duty_index = (self.duty_index + 1) % 8;
            }
        }
    }

    fn sample(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        let sample = DUTIES[self.wave_duty as usize][self.duty_index as usize] as f32;
        // If a DAC is enabled, the digital range $0 to $F is linearly translated to the analog range -1 to 1,
        // in arbitrary units. Importantly, the slope is negative: “digital 0” maps to “analog 1”, not “analog -1”.
        // ((sample * 2.0) - 1.0) * self.volume as f32 / MAX_ENVELOPE_VOL
        ((0.5 - sample) * 2.0) * self.volume as f32 / MAX_ENVELOPE_VOL
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn reset(&mut self) {
        // NR10
        self.sweep_pace = 0;
        self.sweep_direction = false;
        self.individual_step = 0;

        // NR11
        self.wave_duty = 0;
        self.initial_length_timer = 0;

        // NR12
        self.initial_volume = 0;
        self.env_dir = false;
        self.env_pace = 0;
        // since initial_volume & env_dir is 0, we disable the DAC, thus the channel
        self.enabled = false;

        // NR14
        self.length_enabled = false;
        self.period = 0;

        self.period_divider = self.period;
        self.sweep_pace_remaining = self.sweep_pace;
        if self.sweep_pace_remaining == 0 {
            self.sweep_pace_remaining = 8
        };

        self.env_pace_index = 0;
        self.volume = self.initial_volume;
    }

    fn reset_frame(&mut self) {
        self.duty_index = 0;
        self.audio_step_state = 0;
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

        // Note 3
        // CH1 frequency can be modified via NR13 and NR14 while sweep is active, but the “shadow register”
        // won’t be affected so the next time the “sweep timer” updates the channel’s frequency, this modification
        // will be lost. This can be avoided by triggering the channel.

        if self.sweep_pace_remaining > 0 {
            self.sweep_pace_remaining -= 1;
        }

        if self.sweep_pace_remaining != 0 {
            return;
        }
        // Sweep Timer Clocked
        self.sweep_pace_remaining = self.sweep_pace;
        if self.sweep_pace_remaining == 0 {
            self.sweep_pace_remaining = 8
        };
        if !self.sweep_enabled || self.sweep_pace == 0 {
            return;
        }

        let new_period = self.calculate_new_frequency();
        if (!self.sweep_direction && new_period >= 2048) || (self.sweep_direction && new_period == 0) {
            // this should also be ok (reaches #5)
            info!("Disabling - 5 {}", new_period);
            self.enabled = false;
            return;
        }

        // TODO why is this check for individual step performed? it makes sense on trigger but not here?
        if self.individual_step > 0 {
            self.sweep_shadow_period = new_period;
            self.period = new_period;

            // Perform a new overflow check, but ditch the frequency
            let _period = self.calculate_new_frequency();
            // TODO not tested yet
            if (!self.sweep_direction && _period >= 2048) || (self.sweep_direction && _period == 0) {
                info!("Disabling - ?? {}", _period);
                self.enabled = false;
            }
        }
    }

    fn calculate_new_frequency(&self) -> u16 {
        // false means addition
        let new_period = if self.sweep_direction {
            self.sweep_shadow_period - (self.sweep_shadow_period >> self.individual_step)
        } else {
            self.sweep_shadow_period + (self.sweep_shadow_period >> self.individual_step)
        };
        new_period
    }
}

impl Default for Pulse {
    fn default() -> Self {
        let period = 0xff | (0x7 << 8);
        Self {
            enabled: false,
            has_sweep: false,
            sweep_enabled: false, // sweep-pace and individual-steps are false
            volume: 0xf,
            period_divider: period,
            duty_index: 0,
            sweep_pace_remaining: 8, // treating sweep_pace 0 as 8
            env_pace_index: 0,
            audio_step_state: 0,
            audio_step_counter: 0,
            length_counter: MAX_LENGTH - 0x3f,
            length_enabled: false,
            period: period,
            sweep_shadow_period: period,
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

                1 << 7 | pace | direction | step
            }

            0x1 => {
                // 7	6	    | 5	4	3	2	1	0
                // Wave duty	Initial length timer
                // let timer = self.initial_length_timer;
                let timer = 0b00111111; // Timer is read only
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

            0x3 => {
                //(self.period & 0xff) as u8
                0xff // it's write only field
            }

            0x4 => {
                // 7	    | 6	         | 5 4 3 | 2	1	0
                // Trigger	Length enable		   Period
                let trigger = 1 << 7;
                let length_enable = (self.length_enabled as u8) << 6;
                // let period = (self.period >> 8) as u8;
                let period = 0x7; // write only

                trigger | length_enable | 0b111000 | period
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
                if value == 0x1F {
                    info!("Wrote 1F to NR10");
                }
                if value == 0x18 {
                    info!("Wrote 18 to NR10");
                }
                if value == 0x10 {
                    info!("Wrote 10 to NR10");
                }
                if value == 0xF {
                    info!("Wrote F to NR10");
                }
                if value == 0x79 {
                    info!("Wrote 79 to NR10");
                }
                if self.sweep_direction != (value & (1 << 3) > 0) {
                    info!("changing sweep_direction to {:?}", (value & (1 << 3) > 0));
                }
                self.sweep_direction = value & (1 << 3) > 0;
                // Note that the value written to this field is not re-read by the hardware until a
                // sweep iteration completes, or the channel is (re)triggered.
                self.sweep_pace = (value >> 4) & 0x7;
            }

            0x1 => {
                // 7	6	    | 5	4	3	2	1	0
                // Wave duty	Initial length timer
                self.initial_length_timer = value & 0b00111111;
                // Writing a byte to NRx1 loads the counter with 64-data (256-data for wave channel). The counter can be reloaded at any time.
                self.length_counter = MAX_LENGTH - self.initial_length_timer;
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
                let trigger = value >> 7 > 0;
                let old_length_enabled = self.length_enabled;
                self.length_enabled = value & (1 << 6) > 0;
                let enabling_length = !old_length_enabled && self.length_enabled;
                self.period = (self.period & 0xff) | ((value as u16 & 7) << 8);

                // Extra length clocking occurs when writing to NRx4 when the frame sequencer's next
                // step is one that doesn't clock the length counter. In this case, if the length
                // counter was PREVIOUSLY disabled and now enabled and the length counter is not zero,
                // it is decremented. If this decrement makes it zero and trigger is clear, the
                // channel is disabled.
                if enabling_length && self.length_counter != 0 && self.audio_step_state % 2 == 1 {
                    self.length_counter -= 1;
                    if self.length_counter == 0 && !trigger {
                        self.enabled = false
                    }
                }

                if trigger {
                    // Channel is enabled.
                    // Channel x’s DAC is enabled if and only if [NRx2] & $F8 != 0.
                    self.enabled = self.env_dir || self.initial_volume > 0;
                    // The period divider is set to the contents of NR13 and NR14.
                    self.period_divider = self.period;
                    // Volume is set to contents of NR12 initial volume.
                    self.volume = self.initial_volume;
                    // Envelope timer is reset.
                    self.env_pace_index = 0;

                    // If length timer expired it is reset.
                    if self.length_counter == 0 {
                        self.length_counter = MAX_LENGTH;
                        // If a channel is triggered when the frame sequencer's next step is one
                        // that doesn't clock the length counter and the length counter is now enabled
                        // and length is being set to 64 (256 for wave channel) because it was
                        // previously zero, it is set to 63 instead (255 for wave channel)
                        if self.length_enabled && self.audio_step_state % 2 == 1 {
                            self.length_counter -= 1;
                        }
                    }
                    // Sweep does several things.
                    if self.has_sweep {
                        // CH1 period value is copied to the “shadow register”.
                        self.sweep_shadow_period = self.period;
                        // The “sweep timer” is reset.
                        self.sweep_pace_remaining = self.sweep_pace;
                        if self.sweep_pace_remaining == 0 {
                            self.sweep_pace_remaining = 8
                        };

                        // The “enabled flag” is set if either the sweep pace or individual step are non-zero, cleared otherwise.
                        self.sweep_enabled = self.sweep_pace != 0 || self.individual_step != 0;
                        // If the individual step is non-zero, frequency calculation and overflow check are performed immediately.
                        if self.individual_step != 0 { // This should be right reaches #6
                            let _freq = self.calculate_new_frequency();
                            if (!self.sweep_direction && _freq >= 2048) || (self.sweep_direction && _freq == 0) {
                                info!("Disabling - 6 {}", _freq);
                                self.enabled = false;
                            }
                        }
                    }
                }
            }

            _ => panic!("missing pulse_wave write: {:#x}", location),
        }
    }
}
