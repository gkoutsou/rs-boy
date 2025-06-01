use log::{info, trace};

use super::memory_bus::MemoryAccessor;

pub struct Timer {
    // FF04
    // This register is incremented at a rate of 16384Hz (~16779Hz on SGB).
    // Writing any value to this register resets it to $00. Additionally,
    // this register is reset when executing the stop instruction, and only
    // begins ticking again once stop mode ends.
    /// FF05
    /// This timer is incremented at the clock frequency specified by the TAC
    /// register ($FF07). When the value overflows (exceeds $FF) it is reset to
    /// the value specified in TMA (FF06) and an interrupt is requested, as
    /// described below.
    tima: u8,
    /// FF06
    /// When TIMA overflows, it is reset to the value in this register and an
    /// interrupt is requested.
    tma: u8,
    /// FF07
    tac: u8,

    // temporary:
    system_clock: u16,
    tima_counter: u8,
    tima_overflow_delay: bool,
}

impl Timer {
    fn tima_enabled(&self) -> bool {
        (self.tac & (1 << 2)) > 0
    }

    fn tima_clock_bit(&self) -> u8 {
        let selected = self.tac & 0x3;
        info!("selected {}", selected);

        match selected {
            0 => 9, // every 256 M-Ticks - or every 4th div
            1 => 3, // every 4 M-Ticks
            2 => 5, // every 16 M-Ticks
            3 => 7, // every 64 M-Ticks - or every div
            _clock => panic!("unknown tima clock: {}", _clock),
        }
    }

    pub fn step_timer(&mut self, dots: u32) -> bool {
        // a dot is: 4194000 Hz
        // div step:   16384 Hz
        // so a div is stepped every 255.981445313 dots
        // or 256/4 = 64 m_ticks
        let old_clock = self.system_clock;
        self.system_clock = self.system_clock.wrapping_add(dots as u16);

        // if self.tima_overflow_delay {
        //     self.tima_overflow_delay = false;
        //     return true;
        // }

        if !self.tima_enabled() {
            return false;
        }

        if Self::has_bit_gone_low(old_clock, self.system_clock, self.tima_clock_bit()) {
            println!(
                "{:#b} {:#b} - {}",
                old_clock,
                self.system_clock,
                self.system_clock >> 8,
            );
            // self.tima_overflow_delay = self.timer_tick();
            // return false;
            return self.timer_tick();
        }

        // TODO: On monochrome consoles, disabling the timer if the currently selected bit is set,
        //  will send a “Timer tick” once.
        false
    }

    fn has_bit_gone_low(old: u16, new: u16, bit: u8) -> bool {
        let original_bit = (old >> bit) & 1;
        let new_bit = (new >> bit) & 1;
        original_bit != new_bit && new_bit == 0 // TODO this has issue when stepping too much, right? for loop instead?
    }

    fn timer_tick(&mut self) -> bool {
        self.tima = self.tima.wrapping_add(1);

        if self.tima == 0 {
            self.tima = self.tma;
            return true;
        }
        return false;
    }

    pub fn new() -> Self {
        Timer {
            tima: 0,
            tma: 0,
            tac: 0xf8,
            // helpers
            system_clock: 0xab << 8,
            tima_counter: 0,
            tima_overflow_delay: false,
        }
    }
}

impl MemoryAccessor for Timer {
    fn get(&self, location: usize) -> u8 {
        trace!("Read Timer: {:#x}", location);
        match location {
            0xFF04 => (self.system_clock >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => panic!("timer register location read: {:#x}", location),
        }
    }

    fn write(&mut self, location: usize, value: u8) {
        trace!("Writing to Timer Register: {:#x}: {:#b}", location, value);
        match location {
            0xFF04 => {
                // writing any value resets it
                self.system_clock = 0;
                println!("RESET");
            }
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value,
            _ => panic!(
                "timer register location write: {:#x} - {:#x}",
                location, value
            ),
        }
    }
}
