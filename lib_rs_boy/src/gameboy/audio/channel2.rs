use log::warn;

use crate::gameboy::memory_bus::MemoryAccessor;

use super::{pulse_wave::Pulse, wave::Wave};

const MEMORY_BASE: usize = 0xff15;
pub(crate) struct Channel2 {
    pulse: Pulse,
}

impl MemoryAccessor for Channel2 {
    fn get(&self, location: usize) -> u8 {
        if location == 0xff15 {
            warn!("channel2 has no sweep");
            return 0xff;
        }
        self.pulse.get(location - MEMORY_BASE)
    }

    fn write(&mut self, location: usize, value: u8) {
        if location == 0xff15 {
            warn!("channel2 has no sweep");
            return;
        }
        self.pulse.write(location - MEMORY_BASE, value)
    }
}

impl Wave for Channel2 {
    fn step(&mut self, step: u32) {
        self.pulse.step(step)
    }

    fn sample(&self) -> f32 {
        self.pulse.sample()
    }

    fn is_enabled(&self) -> bool {
        self.pulse.is_enabled()
    }

    fn reset(&mut self) {
        self.pulse.reset();
    }

    fn reset_frame(&mut self) {
        self.pulse.reset_frame();
    }
}

impl Default for Channel2 {
    fn default() -> Self {
        Self {
            pulse: Default::default(),
        }
    }
}
