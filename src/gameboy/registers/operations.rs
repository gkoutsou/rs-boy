use super::set_flag;
use crate::gameboy::cpu::Flag;

pub trait Operations {
    fn xor(&mut self, b: u8) -> u8;
    fn or(&mut self, b: u8) -> u8;
    fn and(&mut self, b: u8) -> u8;
    fn add(&mut self, b: u8) -> u8;
    fn adc(&mut self, b: u8, c: bool) -> u8;
    fn sub(&mut self, b: u8) -> u8;
    fn sbc(&mut self, b: u8, c: bool) -> u8;
    fn cp(self, b: u8) -> u8;

    fn inc(&mut self, f: &mut u8);
    fn dec(&mut self, f: &mut u8);
    fn complement(&mut self, f: u8) -> u8;

    fn set_bit(&mut self, bit: u8, value: bool);
    fn has_flag(self, flag: Flag) -> bool;

    fn rl(&mut self, f: &mut u8);
    fn rr(&mut self, f: &mut u8);

    // cb operations
    // TODO leftover f returns..
    fn bit(self, bit: u8, f: u8) -> u8;
    fn rlc(&mut self) -> u8;
    fn rrc(&mut self) -> u8;

    fn sla(&mut self) -> u8;
    fn sra(&mut self) -> u8;
    fn srl(&mut self) -> u8;

    fn swap(&mut self) -> u8;
}

impl Operations for u8 {
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

    fn inc(&mut self, f: &mut u8) {
        // let inc = *self + 1;
        let inc = (*self).wrapping_add(1);
        let mut new_f = set_flag(*f, Flag::H, (*self & 0x0F) + 1 > 0x0F);
        new_f = set_flag(new_f, Flag::Z, inc == 0);
        new_f = set_flag(new_f, Flag::N, false);
        *self = inc;
        *f = new_f;
    }

    fn dec(&mut self, f: &mut u8) {
        let mut new_f = set_flag(*f, Flag::H, (*self & 0x0f) == 0);
        let dec = self.wrapping_sub(1);
        new_f = set_flag(new_f, Flag::Z, dec == 0);
        new_f = set_flag(new_f, Flag::N, true);
        *self = dec;
        *f = new_f;
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

    fn rr(&mut self, f: &mut u8) {
        let mut a = *self;
        let new_c = a & 0x01;
        let old_c = (f.has_flag(Flag::C) as u8) << 7;
        a = a >> 1 | old_c;
        let new_f = set_flag(0x0, Flag::C, new_c == 1);
        *f = set_flag(new_f, Flag::Z, a == 0);
        *self = a;
    }

    fn rl(&mut self, f: &mut u8) {
        let mut a = *self;
        let new_c = a & (1 << 7) > 0;
        let old_c = f.has_flag(Flag::C);
        a = a << 1 | old_c as u8;
        let new_f = set_flag(0, Flag::Z, a == 0);
        *f = set_flag(new_f, Flag::C, new_c);
        *self = a;
    }
}
