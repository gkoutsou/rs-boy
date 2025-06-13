use super::cpu::Flag;
pub(crate) mod operations;

pub fn set_flag(f: u8, flag: Flag, value: bool) -> u8 {
    if value {
        f | flag as u8
    } else {
        f & !(flag as u8)
    }
}

pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
}

impl Registers {
    pub fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = (v & 0x00FF) as u8;
    }

    pub fn get_hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    pub fn set_af(&mut self, v: u16) {
        self.a = (v >> 8) as u8;
        self.f = (v & 0x00FF) as u8;
    }

    pub fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
    }

    pub fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = (v & 0x00FF) as u8;
    }

    pub fn get_bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    pub fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = (v & 0x00FF) as u8;
    }

    pub fn get_de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    pub fn step_pc(&mut self) -> usize {
        let current = self.pc;
        self.pc += 1;
        current as usize
    }

    pub fn set_pc(&mut self, loc: u16) {
        self.pc = loc;
    }

    pub fn add(a: u16, b: u16, f: u8) -> (u16, u8) {
        let result = a.wrapping_add(b);

        let mut f = set_flag(f, Flag::H, (a & 0x07FF) + (b & 0x07FF) > 0x07FF);
        f = set_flag(f, Flag::C, a > 0xFFFF - b);
        f = set_flag(f, Flag::N, false);
        (result, f)
    }

    pub fn new() -> Self {
        Registers {
            // Classic
            pc: 0x100,
            sp: 0xFFFE,
            a: 0x01, // 0xFF for GameBoy Pocket
            l: 0x4d,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xd8,
            h: 0x01,
        }
    }
}
