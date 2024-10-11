use crate::gameboy::memory_bus::MemoryAccessor;

use super::{pulse_wave::Pulse, wave::Wave};

const MEMORY_BASE: usize = 0xff15;
pub(crate) struct Channel2 {
    pulse: Pulse,
}

impl MemoryAccessor for Channel2 {
    fn get(&self, location: usize) -> u8 {
        if location == 0xff15 {
            panic!("channel2 has no sweep");
        }
        self.pulse.get(location - MEMORY_BASE)
    }

    fn write(&mut self, location: usize, value: u8) {
        if location == 0xff15 {
            panic!("channel2 has no sweep");
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
}

impl Default for Channel2 {
    fn default() -> Self {
        Self {
            pulse: Default::default(),
        }
    }
}
