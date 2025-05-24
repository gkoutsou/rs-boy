use crate::gameboy::memory_bus::MemoryAccessor;

use super::{pulse_wave::Pulse, wave::Wave};
const MEMORY_BASE: usize = 0xff10;

pub(crate) struct Channel1 {
    pulse: Pulse,
}

impl MemoryAccessor for Channel1 {
    fn get(&self, location: usize) -> u8 {
        self.pulse.get(location - MEMORY_BASE)
    }

    fn write(&mut self, location: usize, value: u8) {
        self.pulse.write(location - MEMORY_BASE, value)
    }
}

impl Wave for Channel1 {
    fn step(&mut self, step: u32) {
        self.pulse.step(step)
    }

    fn sample(&self) -> f32 {
        self.pulse.sample()
    }

    fn is_enabled(&self) -> bool {
        self.pulse.is_enabled()
    }
}

impl Default for Channel1 {
    fn default() -> Self {
        let mut pulse = Pulse::default();
        pulse.has_sweep = true;

        Self { pulse }
    }
}
