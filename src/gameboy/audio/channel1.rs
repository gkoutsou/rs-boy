use crate::gameboy::memory_bus::MemoryAccessor;

pub(crate) struct Channel1 {
    enabled: bool,
    // FF10 — NR10: Channel 1 sweep
    // This register controls CH1’s period sweep functionality.

    // 7	| 6	5 4 | 3	            | 2	1	0
    //        Pace	  Direction	    Individual step

    // FF11 — NR11: Channel 1 length timer & duty cycle
    // 7	6	    | 5	4	3	2	1	0
    // Wave duty	Initial length timer

    // FF12 — NR12: Channel 1 volume & envelope
    // 7	6	5	4	| 3	        |2	1	0
    // Initial volume	Env dir     Sweep pace

    // FF13 — NR13: Channel 1 period low [write-only]

    // FF14 — NR14: Channel 1 period high & control
    // 7	    | 6	         | 5 4 3 | 2	1	0
    // Trigger	Length enable		   Period
}

impl Channel1 {
    pub fn sample(&self) -> f32 {
        1.0
    }
}

impl Default for Channel1 {
    fn default() -> Self {
        Self {
            // TODO I have no idea about defaults
            enabled: true,
        }
    }
}

impl MemoryAccessor for Channel1 {
    fn get(&self, location: usize) -> u8 {
        0
    }

    fn write(&mut self, location: usize, value: u8) {}
}
