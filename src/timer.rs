use log::{debug, info, trace};

pub struct Timer {
    /// FF04
    /// This register is incremented at a rate of 16384Hz (~16779Hz on SGB).
    /// Writing any value to this register resets it to $00. Additionally,
    /// this register is reset when executing the stop instruction, and only
    /// begins ticking again once stop mode ends.
    div: u8,
    /// FF05
    /// This timer is incremented at the clock frequency specified by the TAC
    /// register ($FF07). When the value overflows (exceeds $FF) it is reset to
    /// the value specified in TMA (FF06) and an interrupt is requested, as
    /// described below.
    tima: u8,
    /// FF06
    tma: u8,
    /// FF07
    tac: u8,

    // temporary:
    div_counter: u32,
    tima_counter: u32,
    tima_clock: u32,
}

impl Timer {
    fn tima_enabled(&self) -> bool {
        (self.tac & (1 << 2)) as u8 > 0
    }

    fn clock(&self) -> u32 {
        let selected = self.tac & 0x3;
        info!("selected {}", selected);

        match selected {
            0 => 1024,
            1 => 16,
            2 => 64,
            3 => 256,
            _clock => panic!("unknown tima clock: {}", _clock),
        }
    }

    pub fn step_timer(&mut self, ticks: u32) -> bool {
        // a dot is: 4194000 Hz
        // div step:   16384 Hz
        // so a div is stepped every 255.981445313 dots
        self.div_counter += ticks;
        if self.div_counter >= 256 {
            self.div_counter -= 256;
            self.div = self.div.wrapping_add(1);
            // debug!("div: {}", self.div)
        }

        if !self.tima_enabled() {
            return false;
        }

        self.tima_counter += ticks;
        let clock = self.tima_clock;

        if self.tima_counter >= clock {
            self.tima_counter -= clock;
            self.tima = self.tima.wrapping_add(1);
            // debug!("tima: {}", self.tima);

            if self.tima == 0 {
                self.tima = self.tma;
                return true;
                // todo!("trigger timer interrupt");
            }
        }
        return false;
    }

    pub fn get(&self, location: usize) -> u8 {
        trace!("Read: {:#x}", location);
        match location {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => panic!("timer register location read: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        trace!("Writting to Timer Register: {:#x}: {:#b}", location, value);
        match location {
            0xFF04 => self.div = 0, // writing any value resets it
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => {
                self.tac = value;
                self.tima_clock = self.clock();
            }
            _ => panic!(
                "timer register location write: {:#x} - {:#x}",
                location, value
            ),
        }
    }

    pub fn default() -> Timer {
        Timer {
            div: 0xab,
            tima: 0,
            tma: 0,
            tac: 0xf8,
            // helpers
            div_counter: 0,
            tima_counter: 0,
            tima_clock: 0,
        }
    }
}
