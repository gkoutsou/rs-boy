// type Register = u8;
#[derive(Copy, Clone)]
pub enum Flag {
    /// carry
    C = 0b00010000,
    /// half-carry
    H = 0b00100000,
    /// substraction
    N = 0b01000000,
    /// zero - indicates that result was zero
    Z = 0b10000000,
}

pub trait RegisterOperation {
    fn xor(&mut self, b: u8) -> u8;
    fn or(&mut self, b: u8) -> u8;
    fn and(&mut self, b: u8) -> u8;
    fn add(&mut self, b: u8) -> u8;
    fn adc(&mut self, b: u8, c: bool) -> u8;
    fn sub(&mut self, b: u8) -> u8;
    fn sbc(&mut self, b: u8, c: bool) -> u8;
    fn cp(self, b: u8) -> u8;

    fn inc(&mut self, f: u8) -> u8;
    fn dec(&mut self, f: u8) -> u8;
    fn complement(&mut self, f: u8) -> u8;

    fn set_bit(&mut self, bit: u8, value: bool);
    fn has_flag(self, flag: Flag) -> bool;

    // cb operations
    fn bit(self, bit: u8, f: u8) -> u8;
    fn rlc(&mut self) -> u8;
    fn rrc(&mut self) -> u8;

    fn sla(&mut self) -> u8;
    fn sra(&mut self) -> u8;
    fn srl(&mut self) -> u8;

    fn swap(&mut self) -> u8;
}

impl RegisterOperation for u8 {
    fn or(&mut self, b: u8) -> u8 {
        *self |= b;
        
        set_flag(0x0, Flag::Z, *self == 0)
    }

    fn and(&mut self, b: u8) -> u8 {
        *self &= b;
        let mut f = set_flag(0x0, Flag::Z, *self == 0);
        f = set_flag(f, Flag::H, true);
        f
    }

    fn xor(&mut self, b: u8) -> u8 {
        *self ^= b;
        
        set_flag(0x0, Flag::Z, *self == 0)
    }

    fn inc(&mut self, f: u8) -> u8 {
        // let inc = *self + 1;
        let inc = (*self).wrapping_add(1);
        let mut f = set_flag(f, Flag::H, (*self & 0x0F) + 1 > 0x0F);
        f = set_flag(f, Flag::Z, inc == 0);
        f = set_flag(f, Flag::N, false);
        *self = inc;
        f
    }

    fn dec(&mut self, f: u8) -> u8 {
        let mut f = set_flag(f, Flag::H, (*self & 0x0f) == 0);
        let dec = self.wrapping_sub(1);
        f = set_flag(f, Flag::Z, dec == 0);
        f = set_flag(f, Flag::N, true);
        *self = dec;
        f
    }

    fn complement(&mut self, f: u8) -> u8 {
        *self = !*self;
        let mut f = set_flag(f, Flag::H, true);
        f = set_flag(f, Flag::N, true);
        f
    }

    fn cp(self, b: u8) -> u8 {
        let a = self;
        let mut f = set_flag(0, Flag::C, a < b);
        f = set_flag(f, Flag::H, (b & 0x0f) > (a & 0x0f));
        f = set_flag(f, Flag::Z, a == b);
        f = set_flag(f, Flag::N, true);
        f
    }

    fn add(&mut self, b: u8) -> u8 {
        let a = *self;
        let result = a.wrapping_add(b);
        let mut f = set_flag(0x0, Flag::Z, result == 0);
        f = set_flag(f, Flag::N, false);
        f = set_flag(f, Flag::H, (a & 0xF) + (b & 0xF) > 0xF);
        f = set_flag(f, Flag::C, (a as u16) + (b as u16) > 0xFF);
        *self = result;
        f
    }

    fn adc(&mut self, b: u8, c: bool) -> u8 {
        let a = *self;
        let result = a.wrapping_add(b).wrapping_add(c as u8);
        let mut f = set_flag(0x0, Flag::Z, result == 0);
        f = set_flag(f, Flag::N, false);
        f = set_flag(f, Flag::H, (a & 0xF) + (b & 0xF) + c as u8 > 0xF);
        f = set_flag(f, Flag::C, (a as u16) + (b as u16) + c as u16 > 0xFF);
        *self = result;
        f
    }

    fn sub(&mut self, b: u8) -> u8 {
        let a = *self;
        let mut f = set_flag(0x0, Flag::C, a < b);
        f = set_flag(f, Flag::H, (b & 0x0f) > (a & 0x0f));
        let result = a.wrapping_sub(b);
        f = set_flag(f, Flag::Z, result == 0);
        f = set_flag(f, Flag::N, true);
        *self = result;
        f
    }

    fn sbc(&mut self, b: u8, c: bool) -> u8 {
        let a = *self;
        let mut f = set_flag(0x0, Flag::C, (a as u16) < b as u16 + c as u16);
        f = set_flag(f, Flag::H, (b & 0x0f) + c as u8 > (a & 0x0f));
        let result = a.wrapping_sub(b).wrapping_sub(c as u8);
        f = set_flag(f, Flag::Z, result == 0);
        f = set_flag(f, Flag::N, true);
        *self = result;
        f
    }

    fn set_bit(&mut self, bit: u8, value: bool) {
        if value {
            *self |= 1 << bit
        } else {
            *self &= !(1 << bit)
        }
    }

    fn has_flag(self, flag: Flag) -> bool {
        (self & (flag as u8)) > 0
    }

    fn bit(self, bit: u8, f: u8) -> u8 {
        let res = (1 << bit) & self;
        let mut f = set_flag(f, Flag::Z, res == 0);
        f = set_flag(f, Flag::N, false);
        f = set_flag(f, Flag::H, true);
        f
    }

    fn rlc(&mut self) -> u8 {
        let mut a = *self;
        let new_c = a & (1 << 7) > 0;
        a = a << 1 | (new_c as u8);
        let mut f = set_flag(0, Flag::C, new_c);
        f = set_flag(f, Flag::Z, a == 0);
        *self = a;
        f
    }

    fn rrc(&mut self) -> u8 {
        let mut a = *self;
        let new_c = a & 1 > 0;
        a = a >> 1 | ((new_c as u8) << 7);
        let mut f = set_flag(0, Flag::C, new_c);
        f = set_flag(f, Flag::Z, a == 0);
        *self = a;
        f
    }

    fn sla(&mut self) -> u8 {
        let mut a = *self;
        let c = a & (1 << 7) > 0;
        a <<= 1;
        let mut f = set_flag(0, Flag::Z, a == 0);
        f = set_flag(f, Flag::C, c);
        *self = a;
        f
    }

    fn sra(&mut self) -> u8 {
        let mut a = *self;
        let new_c = a & 0x01 > 0;
        let msb = a & (1 << 7);
        a = a >> 1 | msb;
        let mut f = set_flag(0x0, Flag::C, new_c);
        f = set_flag(f, Flag::Z, a == 0);
        *self = a;
        f
    }

    fn swap(&mut self) -> u8 {
        let mut a = *self;
        a = (a >> 4) | (a << 4);
        let f = set_flag(0x0, Flag::Z, a == 0);
        *self = a;
        f
    }

    fn srl(&mut self) -> u8 {
        let mut a = *self;
        let c = a & 0x01;
        a >>= 1;
        let f = set_flag(0x0, Flag::C, c == 1);
        let f = set_flag(f, Flag::Z, a == 0);
        *self = a;
        f
    }
}

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
        // println!("H={:#x}, L={:#x}", self.h, self.l);
        // println!("HL={:#x}", (self.h as u16) << 8 | self.l as u16);
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

    pub fn rr(register: &mut u8, register_f: &mut u8) {
        let mut a = *register;

        let new_c = a & 0x01;
        let old_c = (register_f.has_flag(Flag::C) as u8) << 7;
        a = a >> 1 | old_c;
        let f = set_flag(0x0, Flag::C, new_c == 1);
        *register_f = set_flag(f, Flag::Z, a == 0);
        *register = a;
    }

    pub fn rl(register: &mut u8, register_f: &mut u8) {
        let mut a = *register;
        let new_c = a & (1 << 7) > 0;
        let old_c = register_f.has_flag(Flag::C);
        a = a << 1 | old_c as u8;
        let f = set_flag(0, Flag::Z, a == 0);
        *register_f = set_flag(f, Flag::C, new_c);
        *register = a;
    }

    pub fn default() -> Registers {
        Registers {
            // Classic
            pc: 0x100,
            sp: 0xFFFE,
            a: 0xff,
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
