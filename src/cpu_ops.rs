#[derive(Copy, Clone)]
pub enum CpuFlag {
    /// carry
    C = 0b00010000,
    /// half-carry
    H = 0b00100000,
    /// substraction
    N = 0b01000000,
    /// zero - indicates that result was zero
    Z = 0b10000000,
}

// TODO remove file
pub fn set_flag(f: u8, flag: CpuFlag, value: bool) -> u8 {
    if value {
        f | flag as u8
    } else {
        f & !(flag as u8)
    }
}
