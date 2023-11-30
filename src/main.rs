use std::{
    fs::File,
    io::{self, Read},
    str,
};

mod cpu_ops;
mod registers;
use cpu_ops::CpuFlag;
pub use registers::Registers;
mod gpu;
// pub use gpu::GPU;
mod memory;
pub use memory::Memory;

use crate::registers::RegisterOperation;

fn u16_to_u8s(input: u16) -> (u8, u8) {
    let hs = (input >> 8) as u8;
    let ls = (input & 0x00FF) as u8;
    (hs, ls)
}

fn u8s_to_u16(ls: u8, hs: u8) -> u16 {
    (hs as u16) << 8 | ls as u16
}

struct Cpu {
    registers: Registers,
    memory: Memory,
    cpu_cycles: u32,
    gpu_mode: gpu::Mode,

    halt: bool,

    /// Interrupt Master Enable
    ime: bool,

    // debug stuff
    debug_counter: i32,
}

fn load_rom() -> io::Result<Vec<u8>> {
    // let mut f = File::open("Adventure Island II - Aliens in Paradise (USA, Europe).gb")?;
    let mut f = File::open("PokemonRed.gb")?;
    let mut buffer = Vec::new();

    // read the whole file
    f.read_to_end(&mut buffer)?;

    Ok(buffer)
}

impl Cpu {
    fn step(&mut self) {
        if self.ime != false
            && self.memory.interrupt_enable & self.memory.io_registers.interrupt_flag > 0
        {
            let interrupts = self.memory.interrupt_enable & self.memory.io_registers.interrupt_flag;
            if interrupts & 0x1 > 0 {
                // vblank interrupt
                self.ime = false;
                self.memory.io_registers.interrupt_flag &= 0b11111110;
                self.push_stack(self.registers.pc);
                self.registers.set_pc(0x40);
                println!("VBlank Interrupt Handler");
                // panic!("asd");
                return;
            }

            println!("Interrupt enable: {:#8b}", self.memory.interrupt_enable);
            println!(
                "Interrupt flag: {:#8b}",
                self.memory.io_registers.interrupt_flag
            );
            self.memory.dump_tile_data();
            panic!("found interrupt")
        }

        if self.halt {
            return;
        }

        let location = self.registers.step_pc();
        println!("Running location {:#x}", location);

        if location >= 0xFF80 && location <= 0xFFFE {
            println!("Running code in HRAM!")
        } else if location > 0x7FFF {
            panic!("moving outside of bank 2")
        }

        let op = self.memory.get_rom(location);
        println!("operator: {:#x}", op);
        match op {
            0xcb => {
                let cb_op = self.get_u8();
                self.do_cb(cb_op);
                self.cpu_cycles += cpu_ops::get_cb_ticks(cb_op);
            }

            _ => {
                self.run_instruction(op);
                self.cpu_cycles += cpu_ops::get_ticks(op);
            }
        }

        // todo this is not CPU steps but w/e for now
        self.gpu_step();
    }

    fn gpu_step(&mut self) {
        if !self.memory.io_registers.lcd_enabled() {
            println!("LCD disabled!");
            self.cpu_cycles = 0;
            return;
        }

        match self.gpu_mode {
            gpu::Mode::Two => {
                let line = self.memory.io_registers.scanline;
                println!("OAM Scan: line {} ({})", line, self.cpu_cycles);
                // 80 dots
                if self.cpu_cycles >= 80 / 4 {
                    // scan pixels TODO ideally I should follow the ticks, not do it at once
                    self.cpu_cycles -= 20;
                    self.gpu_mode = gpu::Mode::Three;
                    let mut object_counter = 0;
                    for i in 0..40 {
                        let tile = self.memory.get_oam_object(i);
                        let double_size = self.memory.io_registers.has_lcd_flag(2);
                        if tile.object_in_scanline(line, double_size) {
                            object_counter += 1;
                            panic!("found object {:?}", tile)
                        }
                    }
                }
            }
            gpu::Mode::One => {
                println!(
                    "VBlank: line {} ({})",
                    self.memory.io_registers.scanline, self.cpu_cycles
                );

                if self.cpu_cycles >= 456 / 4 {
                    self.memory.io_registers.scanline += 1;
                    self.cpu_cycles -= 456 / 4;
                }

                if self.memory.io_registers.scanline > 153 {
                    self.gpu_mode = gpu::Mode::Two;
                    self.memory.io_registers.scanline = 0
                    // self.memory.dump_tile_data();
                    // panic!("tadadaaa")
                }
            }
            gpu::Mode::Zero => {
                println!(
                    "Horrizontal Blank: line {} ({})",
                    self.memory.io_registers.scanline, self.cpu_cycles
                );
                if self.cpu_cycles >= 204 / 4 {
                    self.cpu_cycles -= 204 / 4;
                    if self.memory.io_registers.scanline == 143 {
                        //todo should this be 143?
                        self.memory.io_registers.enable_video_interrupt();
                        self.gpu_mode = gpu::Mode::One;
                    } else {
                        self.gpu_mode = gpu::Mode::Two;
                    }
                }
            }
            gpu::Mode::Three => {
                println!(
                    "Drawing Pixels: line {} ({})",
                    self.memory.io_registers.scanline, self.cpu_cycles
                );
                //todo hack

                if self.cpu_cycles >= 172 / 4 {
                    self.gpu_mode = gpu::Mode::Zero;
                    self.cpu_cycles -= 172 / 4;
                    self.memory.io_registers.scanline += 1;
                }
            }
        }
    }

    fn pop_stack(&mut self) -> u16 {
        let ls = self.memory.get(self.registers.sp as usize);
        self.registers.sp += 1;
        let hs = self.memory.get(self.registers.sp as usize);
        self.registers.sp += 1;
        return u8s_to_u16(ls, hs);
    }

    fn push_stack(&mut self, value: u16) {
        let (hs, ls) = u16_to_u8s(value);
        self.registers.sp -= 1;
        self.memory.write(self.registers.sp as usize, hs);
        self.registers.sp -= 1;
        self.memory.write(self.registers.sp as usize, ls);
    }

    fn get_u16(&mut self) -> u16 {
        let location = self.registers.step_pc();
        let v1 = self.memory.get_rom(location) as u16;
        let location = self.registers.step_pc();
        let v2 = self.memory.get_rom(location) as u16;
        v2 << 8 | v1
    }

    fn get_u8(&mut self) -> u8 {
        let location = self.registers.step_pc();
        self.memory.get_rom(location)
    }

    fn run_instruction(&mut self, op: u8) {
        match op {
            0x0 => println!("NOP"),

            0xc3 => {
                let v = self.get_u16();
                self.registers.set_pc(v);
                println!("JP nn --> {:#x}", v);
            }

            // JR n
            0x18 => {
                let steps = self.get_u8() as i16;
                let new_location = self.registers.pc as i32 + steps as i32;
                self.registers.set_pc(new_location as u16);
                println!("JR n (jump {} -> {:#x})", steps, new_location);
            }

            // JP NZ,nn
            0xc2 => {
                let new_loc = self.get_u16();
                println!("JP NZ,nn --> {:#x}", new_loc);
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    println!("Making the jump!");
                    self.registers.set_pc(new_loc);
                }
            }
            // JP Z,nn CA 12
            0xca => {
                let new_loc = self.get_u16();
                println!("JP Z,nn --> {:#x}", new_loc);
                if self.registers.f.has_flag(registers::Flag::Z) {
                    println!("Making the jump!");
                    self.registers.set_pc(new_loc);
                }
            }
            // JP NC,nn
            0xd2 => {
                let new_loc = self.get_u16();
                println!("JP NC,nn --> {:#x}", new_loc);
                if !self.registers.f.has_flag(registers::Flag::C) {
                    println!("Making the jump!");
                    self.registers.set_pc(new_loc);
                }
            }
            // JP C,nn
            0xda => {
                let new_loc = self.get_u16();
                println!("JP C,nn --> {:#x}", new_loc);
                if self.registers.f.has_flag(registers::Flag::C) {
                    println!("Making the jump!");
                    self.registers.set_pc(new_loc);
                }
            }

            // JR cc,n
            0x20 => {
                let steps = self.get_u8() as i8 as i32;
                println!(
                    "JR NZ,n --> {} - {:#x}",
                    steps,
                    self.registers.pc as i32 + steps
                );
                println!("############");
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    println!(
                        "Current location: {:#x}, next: {:#x}",
                        self.registers.pc, new_location
                    );
                    self.registers.set_pc(new_location);
                    // panic!("untested jump");
                }
            }
            0x28 => {
                println!("JR Z,n");
                let steps = self.get_u8() as i8 as i32;
                println!("{:#b}", self.registers.f);
                if self.registers.f.has_flag(registers::Flag::Z) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    println!(
                        "Current location: {}, next: {}",
                        self.registers.pc, new_location
                    );
                    self.registers.set_pc(new_location);
                }
            }
            0x30 => {
                println!("JR NC,n");
                let steps = self.get_u8() as i8 as i32;
                if !self.registers.f.has_flag(registers::Flag::C) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    println!(
                        "Current location: {:#x}, next: {:#x}",
                        self.registers.pc, new_location
                    );
                    self.registers.set_pc(new_location);
                    // panic!("untested jump NC");
                }
            }

            0x38 => {
                println!("JR C,n");
                let steps = self.get_u8() as i8 as i32;
                if self.registers.f.has_flag(registers::Flag::C) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    println!(
                        "Current location: {:#x}, next: {:#x}",
                        self.registers.pc, new_location
                    );
                    self.registers.set_pc(new_location);
                    // panic!("untested jump C");
                }
            }

            // JP (HL)
            0xe9 => {
                println!("JP (HL)");
                self.registers.set_pc(self.registers.get_hl());
            }

            // LD n,nn
            0x01 => {
                println!("LD n,BC");
                let v = self.get_u16();
                self.registers.set_bc(v)
            }
            0x11 => {
                println!("LD n,DE");
                let v = self.get_u16();
                self.registers.set_de(v)
            }
            0x21 => {
                println!("LD n,HL");
                let v = self.get_u16();
                self.registers.set_hl(v)
            }
            0x31 => {
                let v = self.get_u16();
                println!("LD n,SP -> {:#x}", v);
                self.registers.sp = v
            }

            // LD NN, A
            0x02 => {
                println!("LD (BC), A");
                self.memory
                    .write(self.registers.get_bc() as usize, self.registers.a);
            }
            0x12 => {
                println!("LD (DE), A");
                self.memory
                    .write(self.registers.get_de() as usize, self.registers.a);
            }
            0xea => {
                println!("LD (nn),A");
                let target = self.get_u16();
                self.memory.write(target as usize, self.registers.a);
            }

            // LDH (n),A
            0xe0 => {
                let steps = self.get_u8();
                println!("LDH (n),A --> {} value: {}", steps, self.registers.a);
                self.memory.write(0xff00 + steps as usize, self.registers.a);
            }

            // LDH A,(n)
            0xf0 => {
                let steps = self.get_u8();
                println!("LDH A,(n) --> {}", steps);
                self.registers.a = self.memory.get_ffxx(steps as usize);
            }

            // LDI (HL), A
            0x22 => {
                println!(
                    "LDI (HL), A {:#x} => {:#x}",
                    self.registers.get_hl(),
                    self.registers.a
                );
                // if self.registers.get_hl() == 0xc300 {
                //     panic!("tmp")
                // }
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.a);
                self.registers.set_hl(self.registers.get_hl() + 1)
            }

            // LDD A, (HL)
            0x3a => {
                println!("LDD A, (HL)");
                self.registers.a = self.memory.get(self.registers.get_hl() as usize);
                self.registers.set_hl(self.registers.get_hl() - 1)
            }
            // LDI A, (HL)
            0x2a => {
                println!("LDI A, (HL)");
                self.registers.a = self.memory.get(self.registers.get_hl() as usize);
                self.registers.set_hl(self.registers.get_hl() + 1)
            }

            // LD A,n
            0x7f => {}
            0x78 => {
                println!("LD A, B");
                self.registers.a = self.registers.b
            }
            0x79 => {
                println!("LD A, C");
                self.registers.a = self.registers.c
            }
            0x7a => {
                println!("LD A, D");
                self.registers.a = self.registers.d
            }
            0x7b => {
                println!("LD A, E");
                self.registers.a = self.registers.e
            }
            0x7c => {
                println!("LD A, H");
                self.registers.a = self.registers.h
            }
            0x7d => {
                println!("LD A, L");
                self.registers.a = self.registers.l
            }
            0x0a => {
                println!("LD A, (BC)");
                self.registers.a = self.memory.get(self.registers.get_bc() as usize);
            }
            0x1a => {
                println!("LD A, (DE)");
                self.registers.a = self.memory.get(self.registers.get_de() as usize);
            }
            0x7e => {
                println!("LD A, (HL)");
                self.registers.a = self.memory.get(self.registers.get_hl() as usize);
            }
            0x3e => {
                let value = self.get_u8();
                println!("LD A, n -> {}", value);
                self.registers.a = value;
            }

            // B
            0x47 => {
                println!("LD B, A");
                self.registers.b = self.registers.a;
            }
            0x40 => {}
            0x41 => {
                println!("LD B, C");
                self.registers.b = self.registers.c
            }
            0x42 => {
                println!("LD B, D");
                self.registers.b = self.registers.d
            }
            0x43 => {
                println!("LD B, E");
                self.registers.b = self.registers.e
            }
            0x44 => {
                println!("LD B, H");
                self.registers.b = self.registers.h
            }
            0x45 => {
                println!("LD B, L");
                self.registers.b = self.registers.l
            }
            0x46 => {
                println!("LD B, (HL)");
                self.registers.b = self.memory.get(self.registers.get_hl() as usize);
            }
            0x06 => {
                let value = self.get_u8();
                println!("LD B, n -> {}", value);
                self.registers.b = value;
            }

            // C
            0x4f => {
                println!("LD C, A");
                self.registers.c = self.registers.a;
            }
            0x48 => {
                println!("LD C, B");
                self.registers.c = self.registers.b
            }
            0x49 => {}
            0x4a => {
                println!("LD C, D");
                self.registers.c = self.registers.d
            }
            0x4b => {
                println!("LD C, E");
                self.registers.c = self.registers.e
            }
            0x4c => {
                println!("LD C, H");
                self.registers.c = self.registers.h
            }
            0x4d => {
                println!("LD C, L");
                self.registers.c = self.registers.l
            }
            0x4e => {
                println!("LD C, (HL)");
                self.registers.c = self.memory.get(self.registers.get_hl() as usize);
            }
            0x0e => {
                let value = self.get_u8();
                println!("LD C, n -> {}", value);
                self.registers.c = value;
            }

            // D
            0x57 => {
                println!("LD D, A");
                self.registers.d = self.registers.a;
            }
            0x50 => {
                println!("LD D, B");
                self.registers.d = self.registers.b
            }
            0x51 => {
                println!("LD D, C");
                self.registers.d = self.registers.c
            }
            0x52 => {}
            0x53 => {
                println!("LD D, E");
                self.registers.d = self.registers.e
            }
            0x54 => {
                println!("LD D, H");
                self.registers.d = self.registers.h
            }
            0x55 => {
                println!("LD D, L");
                self.registers.d = self.registers.l
            }
            0x56 => {
                println!("LD D, (HL)");
                self.registers.d = self.memory.get(self.registers.get_hl() as usize);
            }
            0x16 => {
                let value = self.get_u8();
                println!("LD D, n -> {}", value);
                self.registers.d = value;
            }

            // E
            0x5f => {
                println!("LD E, A");
                self.registers.e = self.registers.a;
            }
            0x58 => {
                println!("LD E, B");
                self.registers.e = self.registers.b
            }
            0x59 => {
                println!("LD E, C");
                self.registers.e = self.registers.c
            }
            0x5a => {
                println!("LD E, D");
                self.registers.e = self.registers.d
            }
            0x5b => {}
            0x5c => {
                println!("LD E, H");
                self.registers.e = self.registers.h
            }
            0x5d => {
                println!("LD E, L");
                self.registers.e = self.registers.l
            }
            0x5e => {
                println!("LD E, (HL)");
                self.registers.e = self.memory.get(self.registers.get_hl() as usize);
            }
            0x1e => {
                let value = self.get_u8();
                println!("LD E, n -> {}", value);
                self.registers.e = value;
            }

            // H
            0x67 => {
                println!("LD H, A");
                self.registers.h = self.registers.a;
            }
            0x60 => {
                println!("LD H, B");
                self.registers.h = self.registers.b
            }
            0x61 => {
                println!("LD H, C");
                self.registers.h = self.registers.c
            }
            0x62 => {
                println!("LD H, D");
                self.registers.h = self.registers.d
            }
            0x63 => {
                println!("LD H, E");
                self.registers.h = self.registers.e
            }
            0x64 => {}
            0x65 => {
                println!("LD H, L");
                self.registers.h = self.registers.l
            }
            0x66 => {
                println!("LD H, (HL)");
                self.registers.h = self.memory.get(self.registers.get_hl() as usize);
            }
            0x26 => {
                let value = self.get_u8();
                println!("LD H, n -> {}", value);
                self.registers.h = value;
            }

            // L
            0x6f => {
                println!("LD L, A");
                self.registers.l = self.registers.a;
            }
            0x68 => {
                println!("LD L, B");
                self.registers.l = self.registers.b
            }
            0x69 => {
                println!("LD L, C");
                self.registers.l = self.registers.c
            }
            0x6A => {
                println!("LD L, D");
                self.registers.l = self.registers.d
            }
            0x6B => {
                println!("LD L, E");
                self.registers.l = self.registers.e
            }
            0x6C => {
                println!("LD L, H");
                self.registers.l = self.registers.h
            }
            0x6D => {}
            0x6E => {
                println!("LD L, (HL)");
                self.registers.l = self.memory.get(self.registers.get_hl() as usize);
            }
            0x2e => {
                let value = self.get_u8();
                println!("LD L, n -> {}", value);
                self.registers.l = value;
            }

            // (HL)
            0x77 => {
                println!("LD (HL), A");
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.a);
            }
            0x70 => {
                println!("LD (HL), B");
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.b);
            }
            0x71 => {
                println!("LD (HL), C");
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.c);
            }
            0x72 => {
                println!("LD (HL), D");
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.d);
            }
            0x73 => {
                println!("LD (HL), E");
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.e);
            }
            0x74 => {
                println!("LD (HL), H");
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.h);
            }
            0x75 => {
                println!("LD (HL), L");
                self.memory
                    .write(self.registers.get_hl() as usize, self.registers.l);
            }
            0x36 => {
                println!("LD (HL), n");
                let v = self.get_u8();
                self.memory.write(self.registers.get_hl() as usize, v);
            }

            0xfa => {
                println!("LD A, nn");
                let source = self.get_u16();
                self.registers.a = self.memory.get(source as usize);
            }

            // LD A, (C)
            0xf2 => {
                println!("LD A, (C)");
                self.registers.a = self.memory.get_ffxx(self.registers.c as usize);
            }

            // LD (C), A
            0xe2 => {
                println!("LD (C), A");
                self.memory.write_ffxx(self.registers.c, self.registers.a);
            }

            // ADD
            0x87 => {
                println!("ADD A, A");
                self.registers.f = self.registers.a.add(self.registers.a);
            }
            0x80 => {
                println!("ADD A, B");
                self.registers.f = self.registers.a.add(self.registers.b);
            }
            0x81 => {
                println!("ADD A, C");
                self.registers.f = self.registers.a.add(self.registers.c);
            }
            0x82 => {
                println!("ADD A, D");
                self.registers.f = self.registers.a.add(self.registers.d);
            }
            0x83 => {
                println!("ADD A, E");
                self.registers.f = self.registers.a.add(self.registers.e);
            }
            0x84 => {
                println!("ADD A, H");
                self.registers.f = self.registers.a.add(self.registers.h);
            }
            0x85 => {
                println!("ADD A, L");
                self.registers.f = self.registers.a.add(self.registers.l);
            }
            0x86 => {
                println!("ADD A, (HL)");
                let v = self.memory.get(self.registers.get_hl() as usize);
                self.registers.f = self.registers.a.add(v);
            }
            0xc6 => {
                println!("ADD A, #");
                let v = self.get_u8();
                self.registers.f = self.registers.a.add(v);
            }

            0x09 => {
                println!("ADD HL, BC");
                let hl;
                (hl, self.registers.f) = Registers::add(
                    self.registers.get_hl(),
                    self.registers.get_bc(),
                    self.registers.f,
                );
                self.registers.set_hl(hl);
            }
            0x19 => {
                println!("ADD HL, DE");
                let hl;
                (hl, self.registers.f) = Registers::add(
                    self.registers.get_hl(),
                    self.registers.get_de(),
                    self.registers.f,
                );
                self.registers.set_hl(hl);
            }
            0x29 => {
                println!("ADD HL, HL");
                let hl;
                (hl, self.registers.f) = Registers::add(
                    self.registers.get_hl(),
                    self.registers.get_hl(),
                    self.registers.f,
                );
                self.registers.set_hl(hl);
            }
            0x39 => {
                println!("ADD HL, SP");
                let hl;
                (hl, self.registers.f) =
                    Registers::add(self.registers.get_hl(), self.registers.sp, self.registers.f);
                self.registers.set_hl(hl);
            }

            // ADC
            0x8f => {
                println!("ADC A, A");
                self.registers.f = self
                    .registers
                    .a
                    .add(self.registers.a + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x88 => {
                println!("ADC A, B");
                self.registers.f = self
                    .registers
                    .a
                    .add(self.registers.b + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x89 => {
                println!("ADC A, C");
                self.registers.f = self
                    .registers
                    .a
                    .add(self.registers.c + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x8a => {
                println!("ADC A, D");
                self.registers.f = self
                    .registers
                    .a
                    .add(self.registers.d + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x8b => {
                println!("ADC A, E");
                self.registers.f = self
                    .registers
                    .a
                    .add(self.registers.e + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x8c => {
                println!("ADC A, H");
                self.registers.f = self
                    .registers
                    .a
                    .add(self.registers.h + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x8d => {
                println!("ADC A, L");
                self.registers.f = self
                    .registers
                    .a
                    .add(self.registers.l + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x8e => {
                println!("ADC A, (HL)");
                let v = self.memory.get(self.registers.get_hl() as usize);
                self.registers.f = self
                    .registers
                    .a
                    .add(v + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0xce => {
                println!("ADC A, #");
                let v = self.get_u8();
                self.registers.f = self
                    .registers
                    .a
                    .add(v + self.registers.f.has_flag(registers::Flag::C) as u8);
            }

            // SUB n
            0x90 => {
                println!("SUB B");
                self.registers.f = self.registers.a.sub(self.registers.b);
            }

            0xd6 => {
                println!("SUB #");
                let v = self.get_u8();
                self.registers.f = self.registers.a.sub(v);
            }

            // SBC
            0x9f => {
                println!("SBC A, A");
                self.registers.f = self
                    .registers
                    .a
                    .sub(self.registers.a + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x98 => {
                println!("SBC A, B");
                self.registers.f = self
                    .registers
                    .a
                    .sub(self.registers.b + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x99 => {
                println!("SBC A, C");
                self.registers.f = self
                    .registers
                    .a
                    .sub(self.registers.c + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x9a => {
                println!("SBC A, D");
                self.registers.f = self
                    .registers
                    .a
                    .sub(self.registers.d + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x9b => {
                println!("SBC A, E");
                self.registers.f = self
                    .registers
                    .a
                    .sub(self.registers.e + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x9c => {
                println!("SBC A, H");
                self.registers.f = self
                    .registers
                    .a
                    .sub(self.registers.h + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x9d => {
                println!("SBC A, L");
                self.registers.f = self
                    .registers
                    .a
                    .sub(self.registers.l + self.registers.f.has_flag(registers::Flag::C) as u8);
            }
            0x9e => {
                println!("SBC A, (HL)");
                let v = self.memory.get(self.registers.get_hl() as usize);
                self.registers.f = self
                    .registers
                    .a
                    .sub(v + self.registers.f.has_flag(registers::Flag::C) as u8);
            }

            // INC nn
            0x03 => {
                self.registers.set_bc(self.registers.get_bc() + 1);
            }
            0x13 => {
                self.registers.set_de(self.registers.get_de() + 1);
            }
            0x23 => {
                println!("INC HL");
                self.registers.set_hl(self.registers.get_hl() + 1);
            }
            0x33 => {
                println!("INC SP");
                self.registers.sp += 1;
            }

            // DEC nn
            0x0B => {
                println!("DEC BC");
                self.registers.set_bc(self.registers.get_bc() - 1);
            }
            0x1B => {
                println!("DEC DE");
                self.registers.set_de(self.registers.get_de() - 1);
            }
            0x2B => {
                println!("DEC HL");
                self.registers.set_hl(self.registers.get_hl() - 1);
            }
            0x3B => {
                println!("DEC SP");
                self.registers.sp -= 1;
            }

            // INC n
            0x3c => {
                println!("INC A");
                self.registers.f = self.registers.a.inc(self.registers.f);
            }
            0x04 => {
                println!("INC B");
                self.registers.f = self.registers.b.inc(self.registers.f);
            }
            0x0c => {
                println!("INC C");
                self.registers.f = self.registers.c.inc(self.registers.f);
            }
            0x14 => {
                println!("INC D");
                self.registers.f = self.registers.d.inc(self.registers.f);
            }
            0x1c => {
                println!("INC E");
                self.registers.f = self.registers.e.inc(self.registers.f);
            }
            0x24 => {
                println!("INC H");
                self.registers.f = self.registers.h.inc(self.registers.f);
            }
            0x2c => {
                println!("INC L");
                self.registers.f = self.registers.l.inc(self.registers.f);
            }
            0x34 => {
                println!("INC (HL)");
                let location = self.registers.get_hl() as usize;
                let mut value = self.memory.get(location);
                let f = value.inc(self.registers.f);
                self.registers.f = f;
                self.memory.write(location, value);
            }

            // DEC
            0x3d => {
                println!("DEC A");
                self.registers.f = self.registers.a.dec(self.registers.f);
            }
            0x05 => {
                println!("DEC B");
                self.registers.f = self.registers.b.dec(self.registers.f);
            }
            0x0d => {
                println!("DEC C");
                self.registers.f = self.registers.c.dec(self.registers.f);
            }
            0x15 => {
                println!("DEC D");
                self.registers.f = self.registers.d.dec(self.registers.f);
            }
            0x1d => {
                println!("DEC E");
                self.registers.f = self.registers.e.dec(self.registers.f);
            }
            0x25 => {
                println!("DEC H");
                self.registers.f = self.registers.h.dec(self.registers.f);
            }
            0x2d => {
                println!("DEC L");
                self.registers.f = self.registers.l.dec(self.registers.f);
            }

            // AND n
            0xa7 => {
                println!("AND A");
                self.registers.f = self.registers.a.and(self.registers.a);
            }
            0xa0 => {
                println!("AND B");
                self.registers.f = self.registers.a.and(self.registers.b);
            }
            0xa1 => {
                println!("AND C");
                self.registers.f = self.registers.a.and(self.registers.c);
            }
            0xa2 => {
                println!("AND D");
                self.registers.f = self.registers.a.and(self.registers.d);
            }
            0xa3 => {
                println!("AND E");
                self.registers.f = self.registers.a.and(self.registers.e);
            }
            0xa4 => {
                println!("AND H");
                self.registers.f = self.registers.a.and(self.registers.h);
            }
            0xa5 => {
                println!("AND L");
                self.registers.f = self.registers.a.and(self.registers.l);
            }
            0xe6 => {
                let n = self.get_u8();
                println!("AND # -> {}", n);
                self.registers.f = self.registers.a.and(n);
            }

            // OR n
            0xb7 => {
                println!("OR A");
                self.registers.f = self.registers.a.or(self.registers.a);
            }
            0xb0 => {
                println!("OR B");
                self.registers.f = self.registers.a.or(self.registers.b);
            }
            0xb1 => {
                println!("OR C");
                self.registers.f = self.registers.a.or(self.registers.c);
            }
            0xb2 => {
                println!("OR D");
                self.registers.f = self.registers.a.or(self.registers.d);
            }
            0xb3 => {
                println!("OR E");
                self.registers.f = self.registers.a.or(self.registers.e);
            }
            0xb4 => {
                println!("OR H");
                self.registers.f = self.registers.a.or(self.registers.h);
            }
            0xb5 => {
                println!("OR L");
                self.registers.f = self.registers.a.or(self.registers.l);
            }

            0xf6 => {
                println!("OR #");
                let v = self.get_u8();
                self.registers.f = self.registers.a.or(v);
            }

            // XOR n
            0xaf => {
                println!("XOR A");
                self.registers.f = self.registers.a.xor(self.registers.a);
            }
            0xa8 => {
                println!("XOR B");
                self.registers.f = self.registers.a.xor(self.registers.b);
            }
            0xa9 => {
                println!("XOR C");
                self.registers.f = self.registers.a.xor(self.registers.c);
            }
            0xaa => {
                println!("XOR D");
                self.registers.f = self.registers.a.xor(self.registers.d);
            }
            0xab => {
                println!("XOR E");
                self.registers.f = self.registers.a.xor(self.registers.e);
            }
            0xac => {
                println!("XOR H");
                self.registers.f = self.registers.a.xor(self.registers.h);
            }
            0xad => {
                println!("XOR L");
                self.registers.f = self.registers.a.xor(self.registers.l);
            }

            // CP n
            0xbf => {
                println!("CP A");
                self.registers.f = self.registers.a.cp(self.registers.a);
            }
            0xb8 => {
                println!("CP B");
                self.registers.f = self.registers.a.cp(self.registers.b);
            }
            0xb9 => {
                println!("CP C");
                self.registers.f = self.registers.a.cp(self.registers.c);
            }
            0xba => {
                println!("CP D");
                self.registers.f = self.registers.a.cp(self.registers.d);
            }
            0xbb => {
                println!("CP E");
                self.registers.f = self.registers.a.cp(self.registers.e);
            }
            0xbc => {
                println!("CP H");
                self.registers.f = self.registers.a.cp(self.registers.h);
            }
            0xbd => {
                println!("CP L");
                self.registers.f = self.registers.a.cp(self.registers.l);
            }

            0xbe => {
                println!("CP (HL)");
                let mem_loc = self.registers.get_hl() as usize;
                self.registers.f = self.registers.a.cp(self.memory.get(mem_loc));
            }

            0xfe => {
                println!("CP #");
                self.registers.f = self.registers.a.cp(self.get_u8());
            }

            // Interrupts
            0xf3 => {
                // This instruction disables interrupts but not
                // immediately. Interrupts are disabled after
                // instruction after DI is executed.
                println!("Warning: DI");
                self.ime = false;
            }

            0xfb => {
                // This instruction disables interrupts but not
                // immediately. Interrupts are enabled after
                // instruction after DI is executed.
                println!("Warning: EI");
                self.ime = true;
            }

            // Calls
            0xcd => {
                let new_location = self.get_u16();
                println!(
                    "Call nn (from {:#x} to {:#x})",
                    self.registers.pc, new_location
                );
                self.push_stack(self.registers.pc);
                self.registers.set_pc(new_location);
            }

            0xc4 => {
                let new_location = self.get_u16();
                println!("CALL NZ,nn --> {:#x}", new_location);
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    println!("Making the jump!");
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }
            0xcc => {
                let new_location = self.get_u16();
                println!("CALL Z,nn --> {:#x}", new_location);
                if self.registers.f.has_flag(registers::Flag::Z) {
                    println!("Making the jump!");
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }
            0xd4 => {
                let new_location = self.get_u16();
                println!("CALL NC,nn --> {:#x}", new_location);
                if !self.registers.f.has_flag(registers::Flag::C) {
                    println!("Making the jump!");
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }
            0xdc => {
                let new_location = self.get_u16();
                println!("CALL C,nn --> {:#x}", new_location);
                if self.registers.f.has_flag(registers::Flag::C) {
                    println!("Making the jump!");
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }

            // RET
            0xc9 => {
                let new_loc = self.pop_stack();
                println!("RET to: {:#x}", new_loc);
                self.registers.set_pc(new_loc);
            }

            0xc0 => {
                println!("RET NZ");
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    let new_loc = self.pop_stack();
                    println!("Made the jump");
                    self.registers.set_pc(new_loc);
                }
            }
            0xc8 => {
                println!("RET Z");
                if self.registers.f.has_flag(registers::Flag::Z) {
                    let new_loc = self.pop_stack();
                    println!("Made the jump");
                    self.registers.set_pc(new_loc);
                }
            }
            0xd0 => {
                println!("RET NC");
                if !self.registers.f.has_flag(registers::Flag::C) {
                    let new_loc = self.pop_stack();
                    println!("Made the jump");
                    self.registers.set_pc(new_loc);
                }
            }
            0xd8 => {
                println!("RET C");
                if self.registers.f.has_flag(registers::Flag::C) {
                    let new_loc = self.pop_stack();
                    println!("Made the jump");
                    self.registers.set_pc(new_loc);
                }
            }

            // PUSH
            0xf5 => {
                println!("PUSH AF");
                self.push_stack(self.registers.get_af());
            }
            0xc5 => {
                println!("PUSH BC");
                self.push_stack(self.registers.get_bc());
            }
            0xd5 => {
                println!("PUSH DE");
                self.push_stack(self.registers.get_de());
            }
            0xe5 => {
                println!("PUSH HL");
                self.push_stack(self.registers.get_hl());
            }

            // POP
            0xf1 => {
                println!("POP AF");
                let v = self.pop_stack();
                self.registers.set_af(v);
            }

            0xc1 => {
                println!("POP BC");
                let v = self.pop_stack();
                self.registers.set_bc(v);
            }
            0xd1 => {
                println!("POP DE");
                let v = self.pop_stack();
                self.registers.set_de(v);
            }
            0xe1 => {
                println!("POP HL");
                let v = self.pop_stack();
                self.registers.set_hl(v);
            }

            // CPL
            0x2f => {
                println!("CPL");
                self.registers.f = self.registers.a.complement(self.registers.f);
            }

            // SCF
            0x37 => {
                println!("SCF");
                let mut f = cpu_ops::set_flag(self.registers.f, CpuFlag::C, true);
                f = cpu_ops::set_flag(f, CpuFlag::H, false);
                f = cpu_ops::set_flag(f, CpuFlag::N, false);
                self.registers.f = f;
            }

            // MISC
            0x76 => {
                self.halt = true;
                println!("HALT");
                println!("Interrupt enable: {:#8b}", self.memory.interrupt_enable)
            }

            0xd9 => {
                let new_loc = self.pop_stack();
                println!("RETI to: {:#x}", new_loc);
                self.registers.set_pc(new_loc);
                self.ime = true;
            }

            0xcb => {
                panic!("cb operation should not run through this");
            }

            _ => {
                println!("Info for debugging");
                self.memory.dump_tile_data();

                panic!("missing operator {:#x}", op);
            }
        };
    }

    fn do_cb(&mut self, cb_instruction: u8) {
        match cb_instruction {
            0x1a => {
                println!("RR");
                let new_c = self.registers.d & 0x01;
                let msb = (self.registers.f.has_flag(registers::Flag::C) as u8) << 7;
                let shifted = self.registers.d >> 1;

                println!("FROM: {:#b} - {:#b}", self.registers.d, self.registers.f);
                self.registers.d = shifted | msb;

                let mut f = cpu_ops::set_flag(self.registers.f, CpuFlag::C, new_c == 1); //
                f = cpu_ops::set_flag(f, CpuFlag::H, false);
                f = cpu_ops::set_flag(f, CpuFlag::Z, self.registers.d == 0);
                f = cpu_ops::set_flag(f, CpuFlag::N, false);
                self.registers.f = f;
                println!("  TO: {:#b} - {:#b}", self.registers.d, self.registers.f);
                // panic!("TEST RR")
            }

            0x37 => {
                println!("SWAP nimble A");
                self.registers.a = (self.registers.a >> 4) | (self.registers.a << 4);
                let mut f = cpu_ops::set_flag(self.registers.f, CpuFlag::Z, self.registers.a == 0);
                f = cpu_ops::set_flag(f, CpuFlag::N, false);
                f = cpu_ops::set_flag(f, CpuFlag::H, false);
                f = cpu_ops::set_flag(f, CpuFlag::C, false);
                self.registers.f = f;
            }

            // SRA n
            0x28 => {
                println!("SRA B");
                let c = self.registers.b & 0x01;
                let msb = self.registers.b | (1 << 7);
                let shifted = self.registers.b >> 1;

                println!("FROM: {:#x}", self.registers.b);

                let mut f = cpu_ops::set_flag(self.registers.f, CpuFlag::C, c == 1);
                f = cpu_ops::set_flag(f, CpuFlag::H, false);
                f = cpu_ops::set_flag(f, CpuFlag::Z, self.registers.b == 0);
                f = cpu_ops::set_flag(f, CpuFlag::N, false);
                self.registers.f = f;
                self.registers.b = shifted | msb;
                println!("TO: {:#x}", self.registers.b);
                panic!("not implememented SRA properly")
            }

            // SRL n
            0x3f => {
                println!("SRL A");
                let c = self.registers.a & 0x01;
                let shifted = self.registers.a >> 1;

                println!("FROM: {:#b}", self.registers.a);

                let mut f = cpu_ops::set_flag(0x0, CpuFlag::C, c == 1);
                f = cpu_ops::set_flag(f, CpuFlag::H, false);
                f = cpu_ops::set_flag(f, CpuFlag::Z, self.registers.a == 0);
                f = cpu_ops::set_flag(f, CpuFlag::N, false);
                self.registers.f = f;
                self.registers.a = shifted;
                println!("TO: {:#b} F: {:#b}", self.registers.a, self.registers.f);
            }

            // RES
            // 0 byte
            0x87 => {
                self.registers.a.set_bit(0, false);
            }
            0x80 => {
                self.registers.b.set_bit(0, false);
            }
            0x81 => {
                self.registers.c.set_bit(0, false);
            }
            0x82 => {
                self.registers.d.set_bit(0, false);
            }
            0x83 => {
                self.registers.e.set_bit(0, false);
            }
            0x84 => {
                self.registers.h.set_bit(0, false);
            }
            0x85 => {
                self.registers.l.set_bit(0, false);
            }

            // BIT b,r
            0x47 => self.registers.f = self.registers.a.bit(0, self.registers.f),
            0x40 => self.registers.f = self.registers.b.bit(0, self.registers.f),
            0x41 => self.registers.f = self.registers.c.bit(0, self.registers.f),
            0x42 => self.registers.f = self.registers.d.bit(0, self.registers.f),
            0x43 => self.registers.f = self.registers.e.bit(0, self.registers.f),
            0x44 => self.registers.f = self.registers.h.bit(0, self.registers.f),
            0x45 => self.registers.f = self.registers.l.bit(0, self.registers.f),

            0x4f => self.registers.f = self.registers.a.bit(1, self.registers.f),
            0x48 => self.registers.f = self.registers.b.bit(1, self.registers.f),
            0x49 => self.registers.f = self.registers.c.bit(1, self.registers.f),
            0x4a => self.registers.f = self.registers.d.bit(1, self.registers.f),
            0x4b => self.registers.f = self.registers.e.bit(1, self.registers.f),
            0x4c => self.registers.f = self.registers.h.bit(1, self.registers.f),
            0x4d => self.registers.f = self.registers.l.bit(1, self.registers.f),

            _ => {
                println!("Info for debugging");
                self.memory.dump_tile_data();

                panic!("Missing cb {:#x}", cb_instruction)
            }
        }
    }

    // fn dump_tile_map(&self) {
    //     for tile in 0..32 {
    //         let mut sum = 0i32;
    //         for i in 0..16 {
    //             sum += self.tile_data[tile * 16 + i] as i32;
    //         }
    //         if sum > 0 {
    //             for i in 0..16 {
    //                 print!("{:#04x} ", self.tile_data[tile * 16 + i]);
    //             }
    //             println!()
    //         }
    //     }
    // }
}

fn main() {
    println!("Hello, world!");

    let result = load_rom();

    let buffer = result.unwrap();
    // match result {
    //     Err(e) => panic!("Error: {}", e),
    //     Ok(buffer)  => {

    //     }
    // }
    if buffer.len() < 0x150 {
        panic!("Rom size to small");
    }

    let title = str::from_utf8(&buffer[0x134..0x142]).unwrap();

    // println!("Title = {}", title);

    println!("Type = {:#x}", buffer[0x143]);
    println!("GB/SGB Indicator = {:#x}", buffer[0x146]);
    println!("Cartridge type = {:#x}", buffer[0x147]);
    let rom_size = buffer[0x148];
    println!("ROM size = {:#x}", rom_size);
    println!("RAM size = {:#x}", buffer[0x149]);

    let expected_rom_size = 32 * (2u32.pow(rom_size as u32)) * 1024u32;

    if buffer.len() as u32 != expected_rom_size {
        panic!(
            "Wrong length found. Expected {} - Found {}",
            expected_rom_size,
            buffer.len()
        );
    } else {
        println!("ROM size Bytes = {}", expected_rom_size);
    }

    let mut cpu = Cpu {
        registers: Registers {
            // Classic
            pc: 0x100,
            sp: 0xFFFE,
            a: 0x01, // $01-GB/SGB, $FF-GBP, $11-GBC
            l: 0x4d,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xd8,
            h: 0x01,
        },
        memory: Memory::default_with_rom(buffer),

        ime: false,

        halt: false,
        cpu_cycles: 0,
        gpu_mode: gpu::Mode::Two,

        debug_counter: 0,
    };

    for _i in 0..2000000 {
        // println!("Iteration {}", _i);
        cpu.step();
    }
    cpu.memory.dump_tile_data();
}
