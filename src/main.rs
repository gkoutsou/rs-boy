use std::{
    thread::{self, panicking},
    time,
};

mod cartridge;
mod controls;
mod cpu_ops;
mod display;
mod gpu;
mod interrupts;
mod memory;
mod registers;
mod timer;

use cartridge::Cartridge;
use controls::Joypad;
use cpu_ops::CpuFlag;
use display::Display;
use env_logger::Env;
use log::{debug, info, trace};
use memory::Memory;
use registers::Registers;
use timer::Timer;

use crate::registers::RegisterOperation;

fn u16_to_u8s(input: u16) -> (u8, u8) {
    let hs = (input >> 8) as u8;
    let ls = (input & 0x00FF) as u8;
    (hs, ls)
}

fn u8s_to_u16(ls: u8, hs: u8) -> u16 {
    (hs as u16) << 8 | ls as u16
}

struct GameBoy {
    cartridge: Cartridge,
    display: Display,
    joypad: Joypad,
    registers: Registers,
    memory: Memory,

    timer: Timer,

    cpu_cycles: u32,
    gpu_mode: gpu::Mode,

    halt: bool,

    // lcd_prev_state: bool,
    /// Interrupt Master Enable
    ime: bool,
    set_ei: bool,

    // debug stuff
    debug_counter: i32,
}

impl GameBoy {
    fn step(&mut self) {
        if self.interrupt_step() {
            self.cpu_cycles += 20; // todo 16 or 12?
            return;
            // todo should an interrupt still run gpu?
        }

        let current_cpu_cycles = self.cpu_cycles;
        self.cpu_step();
        let next_timer_step = self.cpu_cycles - current_cpu_cycles;

        self.timer_step(next_timer_step);

        self.gpu_step();
    }

    fn cpu_step(&mut self) {
        if !self.halt {
            self.run_cpu_instruction();
        } else {
            self.cpu_cycles += 4;
        }
    }

    fn timer_step(&mut self, ticks: u32) {
        if self.timer.step_timer(ticks) {
            println!("enabling timer interrupt");
            self.memory.io_registers.enable_timer_interrupt();
        }
    }

    fn interrupt_step(&mut self) -> bool {
        if self.set_ei {
            self.ime = true;
            self.set_ei = false;
            return false;
        }
        let interrupts = self.memory.interrupt_enable & self.memory.io_registers.interrupt_flag;
        if interrupts == 0 {
            return false;
        }
        self.halt = false;

        if self.ime == false {
            return false;
        }

        self.ime = false;
        self.push_stack(self.registers.pc);

        if interrupts & interrupts::VBLANK > 0 {
            self.memory.io_registers.interrupt_flag &= !interrupts::VBLANK;
            debug!("VBlank Interrupt Handler from: {:#x}", self.registers.pc);
            self.registers.set_pc(0x40);
            return true;
        }

        if interrupts & interrupts::TIMER > 0 {
            self.memory.io_registers.interrupt_flag &= !interrupts::TIMER;
            println!("Timer Interrupt Handler from: {:#x}", self.registers.pc);
            self.registers.set_pc(0x50);
            return true;
        }

        println!("Interrupt enable: {:#8b}", self.memory.interrupt_enable);
        println!(
            "Interrupt flag: {:#8b}",
            self.memory.io_registers.interrupt_flag
        );
        self.memory.dump_tile_data();
        panic!("found interrupt")
    }

    fn run_cpu_instruction(&mut self) {
        let location = self.registers.step_pc();

        let op = self.memory_read(location);
        debug!("operator: {:#x} ({:#x})", op, location);
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
    }

    fn gpu_step(&mut self) {
        if !self.memory.io_registers.lcd_enabled() {
            trace!("LCD disabled!");
            self.cpu_cycles = 0;
            self.memory.io_registers.ly = 0;
            // self.gpu_mode = gpu::Mode::Two;
            // self.lcd_prev_state = false;
            // todo this probably needs to wipe_screen
            return;
            // } else if !self.lcd_prev_state {
            // self.cpu_cycles = 4;
            // self.lcd_prev_state = true;
        }

        match self.gpu_mode {
            gpu::Mode::Two => {
                let line = self.memory.io_registers.ly;
                // trace!("OAM Scan: line {} ({})", line, self.cpu_cycles);
                // 80 dots
                if self.cpu_cycles >= 80 {
                    // scan pixels TODO ideally I should follow the ticks, not do it at once
                    self.cpu_cycles -= 80;
                    self.set_gpu_mode(gpu::Mode::Three);
                }
            }
            gpu::Mode::One => {
                // trace!(
                //     "VBlank: line {} ({})",
                //     self.memory.io_registers.ly,
                //     self.cpu_cycles
                // );

                if self.cpu_cycles >= 456 {
                    self.memory.io_registers.ly += 1;
                    self.cpu_cycles -= 456;

                    if self.memory.io_registers.ly > 153 {
                        self.set_gpu_mode(gpu::Mode::Two);
                        self.memory.io_registers.ly = 0;
                        // self.memory.dump_tile_data();
                    }
                    // debug!("line: {}", self.memory.io_registers.ly);
                }
            }
            gpu::Mode::Zero => {
                // trace!(
                //     "Horrizontal Blank: line {} ({})",
                //     self.memory.io_registers.ly,
                //     self.cpu_cycles
                // );
                if self.cpu_cycles >= 204 {
                    self.cpu_cycles -= 204;

                    self.memory.io_registers.ly += 1;
                    // debug!("line: {}", self.memory.io_registers.ly);

                    if self.memory.io_registers.ly == 144 {
                        //todo should this be 143?
                        self.memory.io_registers.enable_video_interrupt();

                        self.display.refresh_buffer();

                        let keys = self.display.get_pressed_keys();
                        self.joypad.key_pressed(keys);

                        self.set_gpu_mode(gpu::Mode::One);
                    } else {
                        self.set_gpu_mode(gpu::Mode::Two);
                    }
                }
            }
            gpu::Mode::Three => {
                let line = self.memory.io_registers.ly;
                // trace!(
                //     "Drawing Pixels: line {} ({})",
                //     self.memory.io_registers.ly,
                //     self.cpu_cycles
                // );
                //todo hack

                if self.cpu_cycles >= 172 {
                    self.display.wipe_line(line);
                    self.draw_background();
                    self.draw_sprites(line);

                    self.set_gpu_mode(gpu::Mode::Zero);
                    self.cpu_cycles -= 172;
                }
            }
        }
    }

    fn draw_sprites(&mut self, line: u8) {
        let double_size = self
            .memory
            .io_registers
            .has_lcd_flag(gpu::LcdStatusFlag::ObjectSize);

        let mut object_counter = 0;
        for i in 0..40 {
            let tile = self.memory.get_oam_object(i);

            if tile.object_in_scanline(line, double_size) {
                object_counter += 1;
                debug!("{}: found object {:?}", line, tile);
                if object_counter > 10 {
                    info!("too many sprites on the line. Is it a bug?");
                    break;
                }

                if tile.x == 0 || tile.x >= 168 {
                    debug!("sprite's x is outside of bounds. ignoring");
                    continue;
                }

                let index = if double_size {
                    if line + 16 - tile.y < 8 {
                        tile.tile_index & 0xfe
                    } else {
                        tile.tile_index | 0x01
                    }
                } else {
                    tile.tile_index
                };

                // if not double size or the top tile for double
                // let y_pos = if !double_size || line + 16 - tile.y < 8 {
                //     16 + line as usize - tile.y as usize
                // } else {
                //     16 + line as usize - (tile.y + 8) as usize
                // };
                let y_pos = 16 + line as usize - tile.y as usize;
                let final_y_pos = if !tile.is_y_flipped() {
                    y_pos % 8
                } else {
                    // TODO flipped - double is probably broken
                    8 - (y_pos % 8)
                };

                debug!("line: {} tile.y: {}", line, tile.y);
                let tile_data = self.memory.get_tile_data(0x8000, index, final_y_pos);

                let palette = if (tile.flags & (1 << 4)) > 0 {
                    self.memory.io_registers.obp1
                } else {
                    self.memory.io_registers.obp0
                };
                self.display.draw_tile(tile, line, tile_data, palette);
            }
        }
    }

    fn draw_background(&mut self) {
        let line = self.memory.io_registers.ly;
        if !self
            .memory
            .io_registers
            .has_lcd_flag(gpu::LcdStatusFlag::BgWindowEnabled)
        {
            trace!("bg/window is disabled. must draw white :sadge:");
            // todo we also wipe_line one up. probably uneccessary
            self.display.wipe_line(line);
            return;
        }

        let wx = self.memory.io_registers.wx;
        let wy = self.memory.io_registers.wy;
        let in_window = self
            .memory
            .io_registers
            .has_lcd_flag(gpu::LcdStatusFlag::WindowEnabled)
            && line >= wy;

        let y_pos = if !in_window {
            self.memory.io_registers.scy.wrapping_add(line)
        } else {
            line - wy
        };

        // which of the 8 vertical pixels of the current
        // tile is the scanline on?
        let tile_row = y_pos / 8;

        for x in 0..160u8 {
            let in_window = in_window && x + 7 >= wx;
            let tile_map = self.memory.io_registers.get_tile_map(in_window);

            // translate the current x pos to window space if necessary
            let x_pos = if in_window && x + 7 >= wx {
                x + 7 - wx
            } else {
                x.wrapping_add(self.memory.io_registers.scx)
            };

            let tile_col = x_pos / 8;

            let tile_id = self
                .memory
                .get(tile_map + tile_row as usize * 32 + tile_col as usize);

            let tile_data_baseline = self.memory.io_registers.get_tile_data_baseline();

            let tile_data =
                self.memory
                    .get_tile_data(tile_data_baseline, tile_id, y_pos as usize % 8);

            let palette = self.memory.io_registers.bgp;
            self.display
                .draw_bg_tile(x_pos, x, line, tile_data, palette);
        }
    }

    pub fn get_ffxx(&self, steps: usize) -> u8 {
        let location = 0xff00 + steps as usize;
        self.memory_read(location)
    }

    pub fn write_ffxx(&mut self, steps: u8, value: u8) {
        let location = 0xff00 + steps as usize;
        self.memory_write(location, value);
    }

    pub fn memory_read(&self, location: usize) -> u8 {
        match location {
            0x0000..=0x7FFF => self.cartridge.get(location),

            0xA000..=0xBFFF => self.cartridge.get(location),

            0xff04..=0xff07 => self.timer.get(location),

            controls::REGISTER_LOCATION => self.joypad.get(location),

            _ => self.memory.get(location),
        }
    }
    pub fn memory_write(&mut self, location: usize, value: u8) {
        match location {
            0x0000..=0x7FFF => self.cartridge.write(location, value),

            0xA000..=0xBFFF => self.cartridge.write(location, value),

            0xff04..=0xff07 => self.timer.write(location, value),

            controls::REGISTER_LOCATION => self.joypad.write(location, value),

            _ => self.memory.write(location, value),
        }
    }

    fn pop_stack(&mut self) -> u16 {
        let ls = self.memory_read(self.registers.sp as usize);
        self.registers.sp += 1;
        let hs = self.memory_read(self.registers.sp as usize);
        self.registers.sp += 1;
        return u8s_to_u16(ls, hs);
    }

    fn push_stack(&mut self, value: u16) {
        let (hs, ls) = u16_to_u8s(value);
        self.registers.sp -= 1;
        self.memory_write(self.registers.sp as usize, hs);
        self.registers.sp -= 1;
        self.memory_write(self.registers.sp as usize, ls);
    }

    fn get_u16(&mut self) -> u16 {
        let location = self.registers.step_pc();
        let v1 = self.memory_read(location) as u16;
        let location = self.registers.step_pc();
        let v2 = self.memory_read(location) as u16;
        v2 << 8 | v1
    }

    fn get_u8(&mut self) -> u8 {
        let location = self.registers.step_pc();
        self.memory_read(location)
    }

    fn run_instruction(&mut self, op: u8) {
        match op {
            0x0 => trace!("NOP"),

            0xc3 => {
                let v = self.get_u16();
                self.registers.set_pc(v);
                trace!("JP nn --> {:#x}", v);
            }

            // JR n
            0x18 => {
                let steps = self.get_u8() as i8;
                let new_location = self.registers.pc as i32 + steps as i32;
                self.registers.set_pc(new_location as u16);
                debug!("JR n (jump {} -> {:#x})", steps, new_location);
            }

            // JP NZ,nn
            0xc2 => {
                let new_loc = self.get_u16();
                trace!("JP NZ,nn --> {:#x}", new_loc);
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    trace!("Making the jump!");
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_loc);
                }
            }
            // JP Z,nn CA 12
            0xca => {
                let new_loc = self.get_u16();
                trace!("JP Z,nn --> {:#x}", new_loc);
                if self.registers.f.has_flag(registers::Flag::Z) {
                    trace!("Making the jump!");
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_loc);
                }
            }
            // JP NC,nn
            0xd2 => {
                let new_loc = self.get_u16();
                trace!("JP NC,nn --> {:#x}", new_loc);
                if !self.registers.f.has_flag(registers::Flag::C) {
                    trace!("Making the jump!");
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_loc);
                }
            }
            // JP C,nn
            0xda => {
                let new_loc = self.get_u16();
                trace!("JP C,nn --> {:#x}", new_loc);
                if self.registers.f.has_flag(registers::Flag::C) {
                    trace!("Making the jump!");
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_loc);
                }
            }

            // JR cc,n
            0x20 => {
                let steps = self.get_u8() as i8 as i32;
                trace!(
                    "JR NZ,n --> {} - {:#x}",
                    steps,
                    self.registers.pc as i32 + steps
                );
                trace!("############");
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    debug!(
                        "JUMP - Current location: {:#x}, next: {:#x}",
                        self.registers.pc, new_location
                    );
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_location);
                    // panic!("untested jump");
                }
            }
            0x28 => {
                trace!("JR Z,n");
                let steps = self.get_u8() as i8 as i32;
                trace!("{:#b}", self.registers.f);
                if self.registers.f.has_flag(registers::Flag::Z) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    trace!(
                        "Current location: {}, next: {}",
                        self.registers.pc,
                        new_location
                    );
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_location);
                }
            }
            0x30 => {
                trace!("JR NC,n");
                let steps = self.get_u8() as i8 as i32;
                if !self.registers.f.has_flag(registers::Flag::C) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    trace!(
                        "Current location: {:#x}, next: {:#x}",
                        self.registers.pc,
                        new_location
                    );
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_location);
                    // panic!("untested jump NC");
                }
            }

            0x38 => {
                trace!("JR C,n");
                let steps = self.get_u8() as i8 as i32;
                if self.registers.f.has_flag(registers::Flag::C) {
                    let new_location = (self.registers.pc as i32 + steps) as u16;
                    trace!(
                        "Current location: {:#x}, next: {:#x}",
                        self.registers.pc,
                        new_location
                    );
                    self.cpu_cycles += 4;
                    self.registers.set_pc(new_location);
                    // panic!("untested jump C");
                }
            }

            // JP (HL)
            0xe9 => {
                trace!("JP (HL)");
                self.registers.set_pc(self.registers.get_hl());
            }

            // LD n,nn
            0x01 => {
                trace!("LD n,BC");
                let v = self.get_u16();
                self.registers.set_bc(v)
            }
            0x11 => {
                trace!("LD n,DE");
                let v = self.get_u16();
                self.registers.set_de(v)
            }
            0x21 => {
                trace!("LD n,HL");
                let v = self.get_u16();
                self.registers.set_hl(v)
            }
            0x31 => {
                let v = self.get_u16();
                trace!("LD n,SP -> {:#x}", v);
                self.registers.sp = v
            }

            // LD NN, A
            0x02 => {
                trace!("LD (BC), A");
                self.memory_write(self.registers.get_bc() as usize, self.registers.a);
            }
            0x12 => {
                trace!("LD (DE), A");
                self.memory_write(self.registers.get_de() as usize, self.registers.a);
            }
            0xea => {
                trace!("LD (nn),A");
                let target = self.get_u16();
                self.memory_write(target as usize, self.registers.a);
            }

            // LD (nn), SP
            0x8 => {
                trace!("LD (nn), SP");
                let loc = self.get_u16() as usize;
                let (msb, lsb) = u16_to_u8s(self.registers.sp);
                self.memory_write(loc, lsb);
                self.memory_write(loc + 1, msb);
            }

            // LD SP, HL
            0xf9 => {
                trace!("LD SP, HL");
                self.registers.sp = self.registers.get_hl();
            }

            // LDH (n),A
            0xe0 => {
                let steps = self.get_u8();
                trace!("LDH (n),A --> {} value: {}", steps, self.registers.a);
                self.memory_write(0xff00 + steps as usize, self.registers.a);
            }

            // LDH A,(n)
            0xf0 => {
                let steps = self.get_u8();
                trace!("LDH A,(n) --> {}", steps);
                self.registers.a = self.get_ffxx(steps as usize);
            }

            // LDI (HL), A
            0x22 => {
                trace!(
                    "LDI (HL), A {:#x} => {:#x}",
                    self.registers.get_hl(),
                    self.registers.a
                );
                // if self.registers.get_hl() == 0xc300 {
                //     panic!("tmp")
                // }
                self.memory_write(self.registers.get_hl() as usize, self.registers.a);
                self.registers.set_hl(self.registers.get_hl() + 1)
            }
            // LDD (HL), A
            0x32 => {
                trace!(
                    "LDI (HL), A {:#x} => {:#x}",
                    self.registers.get_hl(),
                    self.registers.a
                );
                self.memory_write(self.registers.get_hl() as usize, self.registers.a);
                self.registers.set_hl(self.registers.get_hl() - 1)
            }

            // LDD A, (HL)
            0x3a => {
                trace!("LDD A, (HL)");
                self.registers.a = self.memory_read(self.registers.get_hl() as usize);
                self.registers.set_hl(self.registers.get_hl() - 1)
            }
            // LDI A, (HL)
            0x2a => {
                trace!("LDI A, (HL)");
                self.registers.a = self.memory_read(self.registers.get_hl() as usize);
                self.registers.set_hl(self.registers.get_hl() + 1)
            }

            0xf8 => {
                let steps = self.get_u8() as i8 as i16;
                trace!("LDHL SP,n -> {}", steps);
                let old_val = self.registers.sp;
                let new_val = old_val.wrapping_add_signed(steps);
                let steps = steps as u16;

                let mut f = cpu_ops::set_flag(
                    0,
                    CpuFlag::H,
                    (old_val & 0x000F) + (steps & 0x000F) > 0x000F,
                );
                f = cpu_ops::set_flag(
                    f,
                    CpuFlag::C,
                    (old_val & 0x00FF) + (steps & 0x00FF) > 0x00FF,
                );

                self.registers.set_hl(new_val);
                self.registers.f = f;
            }

            // LD A,n
            0x7f => {}
            0x78 => {
                trace!("LD A, B");
                self.registers.a = self.registers.b
            }
            0x79 => {
                trace!("LD A, C");
                trace!("A: {:#x} - C: {:#x}", self.registers.a, self.registers.c);
                self.registers.a = self.registers.c
            }
            0x7a => {
                trace!("LD A, D");
                self.registers.a = self.registers.d
            }
            0x7b => {
                trace!("LD A, E");
                self.registers.a = self.registers.e
            }
            0x7c => {
                trace!("LD A, H");
                self.registers.a = self.registers.h
            }
            0x7d => {
                trace!("LD A, L");
                self.registers.a = self.registers.l
            }
            0x0a => {
                trace!("LD A, (BC)");
                self.registers.a = self.memory_read(self.registers.get_bc() as usize);
            }
            0x1a => {
                trace!("LD A, (DE)");
                self.registers.a = self.memory_read(self.registers.get_de() as usize);
            }
            0x7e => {
                trace!("LD A, (HL)");
                self.registers.a = self.memory_read(self.registers.get_hl() as usize);
                debug!(
                    "LD A,(HL): {:#x} hl: {:#x}",
                    self.registers.a,
                    self.registers.get_hl()
                )
            }
            0x3e => {
                let value = self.get_u8();
                trace!("LD A, n -> {}", value);
                self.registers.a = value;
            }

            // B
            0x47 => {
                trace!("LD B, A");
                self.registers.b = self.registers.a;
            }
            0x40 => {}
            0x41 => {
                trace!("LD B, C");
                self.registers.b = self.registers.c
            }
            0x42 => {
                trace!("LD B, D");
                self.registers.b = self.registers.d
            }
            0x43 => {
                trace!("LD B, E");
                self.registers.b = self.registers.e
            }
            0x44 => {
                trace!("LD B, H");
                self.registers.b = self.registers.h
            }
            0x45 => {
                trace!("LD B, L");
                self.registers.b = self.registers.l
            }
            0x46 => {
                trace!("LD B, (HL)");
                self.registers.b = self.memory_read(self.registers.get_hl() as usize);
            }
            0x06 => {
                let value = self.get_u8();
                trace!("LD B, n -> {}", value);
                self.registers.b = value;
            }

            // C
            0x4f => {
                trace!("LD C, A");
                self.registers.c = self.registers.a;
            }
            0x48 => {
                trace!("LD C, B");
                self.registers.c = self.registers.b
            }
            0x49 => {}
            0x4a => {
                trace!("LD C, D");
                self.registers.c = self.registers.d
            }
            0x4b => {
                trace!("LD C, E");
                self.registers.c = self.registers.e
            }
            0x4c => {
                trace!("LD C, H");
                self.registers.c = self.registers.h
            }
            0x4d => {
                trace!("LD C, L");
                self.registers.c = self.registers.l
            }
            0x4e => {
                trace!("LD C, (HL)");
                self.registers.c = self.memory_read(self.registers.get_hl() as usize);
            }
            0x0e => {
                let value = self.get_u8();
                trace!("LD C, n -> {}", value);
                self.registers.c = value;
            }

            // D
            0x57 => {
                trace!("LD D, A");
                self.registers.d = self.registers.a;
            }
            0x50 => {
                trace!("LD D, B");
                self.registers.d = self.registers.b
            }
            0x51 => {
                trace!("LD D, C");
                self.registers.d = self.registers.c
            }
            0x52 => {}
            0x53 => {
                trace!("LD D, E");
                self.registers.d = self.registers.e
            }
            0x54 => {
                trace!("LD D, H");
                self.registers.d = self.registers.h
            }
            0x55 => {
                trace!("LD D, L");
                self.registers.d = self.registers.l
            }
            0x56 => {
                trace!("LD D, (HL)");
                self.registers.d = self.memory_read(self.registers.get_hl() as usize);
            }
            0x16 => {
                let value = self.get_u8();
                trace!("LD D, n -> {}", value);
                self.registers.d = value;
            }

            // E
            0x5f => {
                trace!("LD E, A");
                self.registers.e = self.registers.a;
            }
            0x58 => {
                trace!("LD E, B");
                self.registers.e = self.registers.b
            }
            0x59 => {
                trace!("LD E, C");
                self.registers.e = self.registers.c
            }
            0x5a => {
                trace!("LD E, D");
                self.registers.e = self.registers.d
            }
            0x5b => {}
            0x5c => {
                trace!("LD E, H");
                self.registers.e = self.registers.h
            }
            0x5d => {
                trace!("LD E, L");
                self.registers.e = self.registers.l
            }
            0x5e => {
                trace!("LD E, (HL)");
                self.registers.e = self.memory_read(self.registers.get_hl() as usize);
            }
            0x1e => {
                let value = self.get_u8();
                trace!("LD E, n -> {}", value);
                self.registers.e = value;
            }

            // H
            0x67 => {
                trace!("LD H, A");
                self.registers.h = self.registers.a;
            }
            0x60 => {
                trace!("LD H, B");
                self.registers.h = self.registers.b
            }
            0x61 => {
                trace!("LD H, C");
                self.registers.h = self.registers.c
            }
            0x62 => {
                trace!("LD H, D");
                self.registers.h = self.registers.d
            }
            0x63 => {
                trace!("LD H, E");
                self.registers.h = self.registers.e
            }
            0x64 => {}
            0x65 => {
                trace!("LD H, L");
                self.registers.h = self.registers.l
            }
            0x66 => {
                trace!("LD H, (HL)");
                self.registers.h = self.memory_read(self.registers.get_hl() as usize);
            }
            0x26 => {
                let value = self.get_u8();
                trace!("LD H, n -> {}", value);
                self.registers.h = value;
            }

            // L
            0x6f => {
                trace!("LD L, A");
                self.registers.l = self.registers.a;
            }
            0x68 => {
                trace!("LD L, B");
                self.registers.l = self.registers.b
            }
            0x69 => {
                trace!("LD L, C");
                self.registers.l = self.registers.c
            }
            0x6A => {
                trace!("LD L, D");
                self.registers.l = self.registers.d
            }
            0x6B => {
                trace!("LD L, E");
                self.registers.l = self.registers.e
            }
            0x6C => {
                trace!("LD L, H");
                self.registers.l = self.registers.h
            }
            0x6D => {}
            0x6E => {
                trace!("LD L, (HL)");
                self.registers.l = self.memory_read(self.registers.get_hl() as usize);
            }
            0x2e => {
                let value = self.get_u8();
                trace!("LD L, n -> {}", value);
                self.registers.l = value;
            }

            // (HL)
            0x77 => {
                trace!("LD (HL), A");
                self.memory_write(self.registers.get_hl() as usize, self.registers.a);
            }
            0x70 => {
                trace!("LD (HL), B");
                self.memory_write(self.registers.get_hl() as usize, self.registers.b);
            }
            0x71 => {
                trace!("LD (HL), C");
                self.memory_write(self.registers.get_hl() as usize, self.registers.c);
            }
            0x72 => {
                trace!("LD (HL), D");
                self.memory_write(self.registers.get_hl() as usize, self.registers.d);
            }
            0x73 => {
                trace!("LD (HL), E");
                self.memory_write(self.registers.get_hl() as usize, self.registers.e);
            }
            0x74 => {
                trace!("LD (HL), H");
                self.memory_write(self.registers.get_hl() as usize, self.registers.h);
            }
            0x75 => {
                trace!("LD (HL), L");
                self.memory_write(self.registers.get_hl() as usize, self.registers.l);
            }
            0x36 => {
                trace!("LD (HL), n");
                let v = self.get_u8();
                self.memory_write(self.registers.get_hl() as usize, v);
            }

            0xfa => {
                trace!("LD A, nn");
                let source = self.get_u16();
                self.registers.a = self.memory_read(source as usize);
            }

            // LD A, (C)
            0xf2 => {
                trace!("LD A, (C)");
                self.registers.a = self.get_ffxx(self.registers.c as usize);
            }

            // LD (C), A
            0xe2 => {
                trace!("LD (C), A");
                self.write_ffxx(self.registers.c, self.registers.a);
            }

            // ADD
            0x87 => {
                trace!("ADD A, A");
                self.registers.f = self.registers.a.add(self.registers.a);
            }
            0x80 => {
                trace!("ADD A, B");
                self.registers.f = self.registers.a.add(self.registers.b);
            }
            0x81 => {
                trace!("ADD A, C");
                self.registers.f = self.registers.a.add(self.registers.c);
            }
            0x82 => {
                trace!("ADD A, D");
                self.registers.f = self.registers.a.add(self.registers.d);
            }
            0x83 => {
                trace!("ADD A, E");
                self.registers.f = self.registers.a.add(self.registers.e);
            }
            0x84 => {
                trace!("ADD A, H");
                self.registers.f = self.registers.a.add(self.registers.h);
            }
            0x85 => {
                trace!("ADD A, L");
                trace!("A: {:#x} - L: {:#x}", self.registers.a, self.registers.l);
                self.registers.f = self.registers.a.add(self.registers.l);
            }
            0x86 => {
                trace!("ADD A, (HL)");
                let v = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = self.registers.a.add(v);
            }
            0xc6 => {
                trace!("ADD A, #");
                let v = self.get_u8();
                self.registers.f = self.registers.a.add(v);
            }

            0x09 => {
                trace!("ADD HL, BC");
                let hl;
                (hl, self.registers.f) = Registers::add(
                    self.registers.get_hl(),
                    self.registers.get_bc(),
                    self.registers.f,
                );
                self.registers.set_hl(hl);
            }
            0x19 => {
                trace!("ADD HL, DE");
                let hl;
                (hl, self.registers.f) = Registers::add(
                    self.registers.get_hl(),
                    self.registers.get_de(),
                    self.registers.f,
                );
                self.registers.set_hl(hl);
            }
            0x29 => {
                trace!("ADD HL, HL");
                let hl;
                (hl, self.registers.f) = Registers::add(
                    self.registers.get_hl(),
                    self.registers.get_hl(),
                    self.registers.f,
                );
                self.registers.set_hl(hl);
            }
            0x39 => {
                trace!("ADD HL, SP");
                let hl;
                (hl, self.registers.f) =
                    Registers::add(self.registers.get_hl(), self.registers.sp, self.registers.f);
                self.registers.set_hl(hl);
            }

            0xe8 => {
                trace!("ADD SP, n");
                let n = self.get_u8() as i8;
                let old_val = self.registers.sp;
                self.registers.sp = self.registers.sp.wrapping_add_signed(n as i16);

                let steps = n as u16;

                let f = cpu_ops::set_flag(
                    0,
                    CpuFlag::H,
                    (old_val & 0x000F) + (steps & 0x000F) > 0x000F,
                );
                self.registers.f = cpu_ops::set_flag(
                    f,
                    CpuFlag::C,
                    (old_val & 0x00FF) + (steps & 0x00FF) > 0x00FF,
                );
            }

            // ADC
            0x8f => {
                trace!("ADC A, A");
                self.registers.f = self.registers.a.adc(
                    self.registers.a,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x88 => {
                trace!("ADC A, B");
                self.registers.f = self.registers.a.adc(
                    self.registers.b,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x89 => {
                trace!("ADC A, C");
                self.registers.f = self.registers.a.adc(
                    self.registers.c,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x8a => {
                trace!("ADC A, D");
                self.registers.f = self.registers.a.adc(
                    self.registers.d,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x8b => {
                trace!("ADC A, E");
                self.registers.f = self.registers.a.adc(
                    self.registers.e,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x8c => {
                trace!("ADC A, H");
                self.registers.f = self.registers.a.adc(
                    self.registers.h,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x8d => {
                trace!("ADC A, L");
                self.registers.f = self.registers.a.adc(
                    self.registers.l,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x8e => {
                trace!("ADC A, (HL)");
                let v = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = self
                    .registers
                    .a
                    .adc(v, self.registers.f.has_flag(registers::Flag::C));
            }
            0xce => {
                trace!("ADC A, #");
                let v = self.get_u8();
                self.registers.f = self
                    .registers
                    .a
                    .adc(v, self.registers.f.has_flag(registers::Flag::C));
            }

            // SUB n
            0x97 => {
                trace!("SUB A");
                self.registers.f = self.registers.a.sub(self.registers.a);
            }
            0x90 => {
                trace!("SUB B");
                self.registers.f = self.registers.a.sub(self.registers.b);
            }
            0x91 => {
                trace!("SUB C");
                self.registers.f = self.registers.a.sub(self.registers.c);
            }
            0x92 => {
                trace!("SUB D");
                self.registers.f = self.registers.a.sub(self.registers.d);
            }
            0x93 => {
                trace!("SUB E");
                self.registers.f = self.registers.a.sub(self.registers.e);
            }
            0x94 => {
                trace!("SUB H");
                self.registers.f = self.registers.a.sub(self.registers.h);
            }
            0x95 => {
                trace!("SUB L");
                self.registers.f = self.registers.a.sub(self.registers.l);
            }
            0x96 => {
                trace!("SUB (HL)");
                let v = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = self.registers.a.sub(v);
            }

            0xd6 => {
                trace!("SUB #");
                let v = self.get_u8();
                self.registers.f = self.registers.a.sub(v);
            }

            // SBC
            0x9f => {
                trace!("SBC A, A");
                self.registers.f = self.registers.a.sbc(
                    self.registers.a,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x98 => {
                trace!("SBC A, B");
                self.registers.f = self.registers.a.sbc(
                    self.registers.b,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x99 => {
                trace!("SBC A, C");
                self.registers.f = self.registers.a.sbc(
                    self.registers.c,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x9a => {
                trace!("SBC A, D");
                self.registers.f = self.registers.a.sbc(
                    self.registers.d,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x9b => {
                trace!("SBC A, E");
                self.registers.f = self.registers.a.sbc(
                    self.registers.e,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x9c => {
                trace!("SBC A, H");
                self.registers.f = self.registers.a.sbc(
                    self.registers.h,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x9d => {
                trace!("SBC A, L");
                self.registers.f = self.registers.a.sbc(
                    self.registers.l,
                    self.registers.f.has_flag(registers::Flag::C),
                );
            }
            0x9e => {
                trace!("SBC A, (HL)");
                let v = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = self
                    .registers
                    .a
                    .sbc(v, self.registers.f.has_flag(registers::Flag::C));
            }

            0xde => {
                trace!("SBC #");
                let v = self.get_u8();
                self.registers.f = self
                    .registers
                    .a
                    .sbc(v, self.registers.f.has_flag(registers::Flag::C));
            }

            // INC nn
            0x03 => {
                self.registers
                    .set_bc(self.registers.get_bc().wrapping_add(1));
            }
            0x13 => {
                self.registers
                    .set_de(self.registers.get_de().wrapping_add(1));
            }
            0x23 => {
                trace!("INC HL");
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_add(1));
            }
            0x33 => {
                trace!("INC SP");
                self.registers.sp = self.registers.sp.wrapping_add(1);
            }

            // DEC nn
            0x0B => {
                trace!("DEC BC");
                self.registers
                    .set_bc(self.registers.get_bc().wrapping_sub(1));
            }
            0x1B => {
                trace!("DEC DE");
                self.registers
                    .set_de(self.registers.get_de().wrapping_sub(1));
            }
            0x2B => {
                trace!("DEC HL");
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));
            }
            0x3B => {
                trace!("DEC SP");
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }

            // INC n
            0x3c => {
                trace!("INC A");
                self.registers.f = self.registers.a.inc(self.registers.f);
            }
            0x04 => {
                trace!("INC B");
                self.registers.f = self.registers.b.inc(self.registers.f);
            }
            0x0c => {
                trace!("INC C");
                self.registers.f = self.registers.c.inc(self.registers.f);
            }
            0x14 => {
                trace!("INC D");
                self.registers.f = self.registers.d.inc(self.registers.f);
            }
            0x1c => {
                trace!("INC E");
                self.registers.f = self.registers.e.inc(self.registers.f);
            }
            0x24 => {
                trace!("INC H");
                self.registers.f = self.registers.h.inc(self.registers.f);
            }
            0x2c => {
                trace!("INC L");
                self.registers.f = self.registers.l.inc(self.registers.f);
            }
            0x34 => {
                trace!("INC (HL)");
                let location = self.registers.get_hl() as usize;
                let mut value = self.memory_read(location);
                let f = value.inc(self.registers.f);
                self.registers.f = f;
                self.memory_write(location, value);
            }

            // DEC
            0x3d => {
                trace!("DEC A");
                self.registers.f = self.registers.a.dec(self.registers.f);
            }
            0x05 => {
                trace!("DEC B");
                self.registers.f = self.registers.b.dec(self.registers.f);
            }
            0x0d => {
                trace!("DEC C");
                self.registers.f = self.registers.c.dec(self.registers.f);
            }
            0x15 => {
                trace!("DEC D");
                self.registers.f = self.registers.d.dec(self.registers.f);
            }
            0x1d => {
                trace!("DEC E");
                self.registers.f = self.registers.e.dec(self.registers.f);
            }
            0x25 => {
                trace!("DEC H");
                self.registers.f = self.registers.h.dec(self.registers.f);
            }
            0x2d => {
                trace!("DEC L");
                self.registers.f = self.registers.l.dec(self.registers.f);
            }

            0x35 => {
                trace!("DEC (HL)");
                let location = self.registers.get_hl() as usize;
                let mut value = self.memory_read(location);
                self.registers.f = value.dec(self.registers.f);
                self.memory_write(location, value);
            }

            // AND n
            0xa7 => {
                trace!("AND A");
                self.registers.f = self.registers.a.and(self.registers.a);
            }
            0xa0 => {
                trace!("AND B");
                self.registers.f = self.registers.a.and(self.registers.b);
            }
            0xa1 => {
                trace!("AND C");
                self.registers.f = self.registers.a.and(self.registers.c);
            }
            0xa2 => {
                trace!("AND D");
                self.registers.f = self.registers.a.and(self.registers.d);
            }
            0xa3 => {
                trace!("AND E");
                self.registers.f = self.registers.a.and(self.registers.e);
            }
            0xa4 => {
                trace!("AND H");
                self.registers.f = self.registers.a.and(self.registers.h);
            }
            0xa5 => {
                trace!("AND L");
                self.registers.f = self.registers.a.and(self.registers.l);
            }
            0xa6 => {
                trace!("AND (HL)");
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = self.registers.a.and(value);
            }
            0xe6 => {
                let n = self.get_u8();
                trace!("AND # -> {}", n);
                self.registers.f = self.registers.a.and(n);
            }

            // OR n
            0xb7 => {
                trace!("OR A");
                self.registers.f = self.registers.a.or(self.registers.a);
            }
            0xb0 => {
                trace!("OR B");
                self.registers.f = self.registers.a.or(self.registers.b);
            }
            0xb1 => {
                trace!("OR C");
                self.registers.f = self.registers.a.or(self.registers.c);
            }
            0xb2 => {
                trace!("OR D");
                self.registers.f = self.registers.a.or(self.registers.d);
            }
            0xb3 => {
                trace!("OR E");
                self.registers.f = self.registers.a.or(self.registers.e);
            }
            0xb4 => {
                trace!("OR H");
                self.registers.f = self.registers.a.or(self.registers.h);
            }
            0xb5 => {
                trace!("OR L");
                self.registers.f = self.registers.a.or(self.registers.l);
            }
            0xb6 => {
                trace!("OR (HL)");
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = self.registers.a.or(value);
            }
            0xf6 => {
                trace!("OR #");
                let v = self.get_u8();
                self.registers.f = self.registers.a.or(v);
            }

            // XOR n
            0xaf => {
                trace!("XOR A");
                self.registers.f = self.registers.a.xor(self.registers.a);
            }
            0xa8 => {
                trace!("XOR B");
                self.registers.f = self.registers.a.xor(self.registers.b);
            }
            0xa9 => {
                trace!("XOR C");
                self.registers.f = self.registers.a.xor(self.registers.c);
            }
            0xaa => {
                trace!("XOR D");
                self.registers.f = self.registers.a.xor(self.registers.d);
            }
            0xab => {
                trace!("XOR E");
                self.registers.f = self.registers.a.xor(self.registers.e);
            }
            0xac => {
                trace!("XOR H");
                self.registers.f = self.registers.a.xor(self.registers.h);
            }
            0xad => {
                trace!("XOR L");
                self.registers.f = self.registers.a.xor(self.registers.l);
            }
            0xae => {
                trace!("XOR (HL)");
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = self.registers.a.xor(value);
            }

            0xee => {
                trace!("XOR n");
                let value = self.get_u8();
                self.registers.f = self.registers.a.xor(value);
            }

            // CP n
            0xbf => {
                trace!("CP A");
                self.registers.f = self.registers.a.cp(self.registers.a);
            }
            0xb8 => {
                trace!("CP B");
                self.registers.f = self.registers.a.cp(self.registers.b);
            }
            0xb9 => {
                trace!("CP C");
                self.registers.f = self.registers.a.cp(self.registers.c);
            }
            0xba => {
                trace!("CP D");
                self.registers.f = self.registers.a.cp(self.registers.d);
            }
            0xbb => {
                trace!("CP E");
                self.registers.f = self.registers.a.cp(self.registers.e);
            }
            0xbc => {
                trace!("CP H");
                self.registers.f = self.registers.a.cp(self.registers.h);
            }
            0xbd => {
                trace!("CP L");
                self.registers.f = self.registers.a.cp(self.registers.l);
            }

            0xbe => {
                trace!("CP (HL)");
                let mem_loc = self.registers.get_hl() as usize;
                self.registers.f = self.registers.a.cp(self.memory_read(mem_loc));
            }

            0xfe => {
                trace!("CP #");
                self.registers.f = self.registers.a.cp(self.get_u8());
            }

            // Interrupts
            0xf3 => {
                // This instruction disables interrupts but not
                // immediately. Interrupts are disabled after
                // instruction after DI is executed.
                info!("Warning: DI");
                self.ime = false;
                self.set_ei = false;
            }

            0xfb => {
                // This instruction enables interrupts but not
                // immediately. Interrupts are enabled after
                // instruction after EI is executed.
                info!("Warning: EI");
                self.set_ei = true;
            }

            // Calls
            0xcd => {
                let new_location = self.get_u16();
                debug!(
                    "Call nn (from {:#x} to {:#x})",
                    self.registers.pc, new_location
                );
                self.push_stack(self.registers.pc);
                self.registers.set_pc(new_location);
            }

            0xc4 => {
                let new_location = self.get_u16();
                debug!("CALL NZ,nn --> {:#x}", new_location);
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    debug!("Making the jump!");
                    self.cpu_cycles += 12;
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }
            0xcc => {
                let new_location = self.get_u16();
                debug!("CALL Z,nn --> {:#x}", new_location);
                if self.registers.f.has_flag(registers::Flag::Z) {
                    debug!("Making the jump!");
                    self.cpu_cycles += 12;
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }
            0xd4 => {
                let new_location = self.get_u16();
                debug!("CALL NC,nn --> {:#x}", new_location);
                if !self.registers.f.has_flag(registers::Flag::C) {
                    debug!("Making the jump!");
                    self.cpu_cycles += 12;
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }
            0xdc => {
                let new_location = self.get_u16();
                debug!("CALL C,nn --> {:#x}", new_location);
                if self.registers.f.has_flag(registers::Flag::C) {
                    debug!("Making the jump!");
                    self.cpu_cycles += 12;
                    self.push_stack(self.registers.pc);
                    self.registers.set_pc(new_location);
                }
            }

            // RET
            0xc9 => {
                let new_loc = self.pop_stack();
                debug!("RET to: {:#x}", new_loc);
                self.registers.set_pc(new_loc);
            }

            0xc0 => {
                debug!("RET NZ");
                if !self.registers.f.has_flag(registers::Flag::Z) {
                    let new_loc = self.pop_stack();
                    debug!("Made the jump");
                    self.cpu_cycles += 12;
                    self.registers.set_pc(new_loc);
                }
            }
            0xc8 => {
                debug!("RET Z");
                if self.registers.f.has_flag(registers::Flag::Z) {
                    let new_loc = self.pop_stack();
                    debug!("Made the jump");
                    self.cpu_cycles += 12;
                    self.registers.set_pc(new_loc);
                }
            }
            0xd0 => {
                debug!("RET NC");
                if !self.registers.f.has_flag(registers::Flag::C) {
                    let new_loc = self.pop_stack();
                    debug!("Made the jump");
                    self.cpu_cycles += 12;
                    self.registers.set_pc(new_loc);
                }
            }
            0xd8 => {
                debug!("RET C");
                if self.registers.f.has_flag(registers::Flag::C) {
                    let new_loc = self.pop_stack();
                    debug!("Made the jump");
                    self.cpu_cycles += 12;
                    self.registers.set_pc(new_loc);
                }
            }

            // RST n
            0xc7 => {
                debug!("RST 00");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x00;
            }
            0xcf => {
                debug!("RST 08");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x08;
            }
            0xd7 => {
                debug!("RST 10");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x10;
            }
            0xdf => {
                debug!("RST 18");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x18;
            }
            0xe7 => {
                debug!("RST 20");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x20;
            }
            0xef => {
                debug!("RST 28");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x28;
            }
            0xf7 => {
                debug!("RST 30");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x30;
            }
            0xff => {
                debug!("RST 38");
                self.push_stack(self.registers.pc);
                self.registers.pc = 0x38;
            }

            // PUSH
            0xf5 => {
                trace!("PUSH AF");
                self.push_stack(self.registers.get_af());
            }
            0xc5 => {
                trace!("PUSH BC");
                self.push_stack(self.registers.get_bc());
            }
            0xd5 => {
                trace!("PUSH DE");
                self.push_stack(self.registers.get_de());
            }
            0xe5 => {
                trace!("PUSH HL");
                self.push_stack(self.registers.get_hl());
            }

            // POP
            0xf1 => {
                trace!("POP AF");
                let v = self.pop_stack();
                self.registers.set_af(v & 0xfff0);
            }

            0xc1 => {
                trace!("POP BC");
                let v = self.pop_stack();
                self.registers.set_bc(v);
            }
            0xd1 => {
                trace!("POP DE");
                let v = self.pop_stack();
                self.registers.set_de(v);
            }
            0xe1 => {
                trace!("POP HL");
                let v = self.pop_stack();
                self.registers.set_hl(v);
            }

            // CPL
            0x2f => {
                trace!("CPL");
                self.registers.f = self.registers.a.complement(self.registers.f);
            }

            // SCF
            0x37 => {
                trace!("SCF");
                let mut f = cpu_ops::set_flag(self.registers.f, CpuFlag::C, true);
                f = cpu_ops::set_flag(f, CpuFlag::H, false);
                f = cpu_ops::set_flag(f, CpuFlag::N, false);
                self.registers.f = f;
            }

            // MISC
            0x27 => {
                let mut a = self.registers.a;
                let mut adjust = 0x60 * self.registers.f.has_flag(registers::Flag::C) as u8;
                if self.registers.f.has_flag(registers::Flag::H) {
                    adjust |= 0x06;
                };
                if !self.registers.f.has_flag(registers::Flag::N) {
                    if a & 0x0F > 0x09 {
                        adjust |= 0x06;
                    };
                    if a > 0x99 {
                        adjust |= 0x60;
                    };
                    a = a.wrapping_add(adjust);
                } else {
                    a = a.wrapping_sub(adjust);
                }

                self.registers.f =
                    registers::set_flag(self.registers.f, registers::Flag::C, adjust >= 0x60);
                self.registers.f = registers::set_flag(self.registers.f, registers::Flag::H, false);
                self.registers.f =
                    registers::set_flag(self.registers.f, registers::Flag::Z, a == 0);
                self.registers.a = a;
            }
            0x76 => {
                self.halt = true;
                debug!("HALT");
                debug!("Interrupt enable: {:#8b}", self.memory.interrupt_enable)
            }

            0xd9 => {
                let new_loc = self.pop_stack();
                debug!("RETI to: {:#x}", new_loc);
                self.registers.set_pc(new_loc);
                self.ime = true;
            }

            0x07 => {
                trace!("RLCA");
                let new_c = self.registers.a & (1 << 7) > 0;
                self.registers.a = self.registers.a << 1 | (new_c as u8);
                self.registers.f = cpu_ops::set_flag(0, CpuFlag::C, new_c);
            }

            0x0f => {
                trace!("RRCA");
                let new_c = self.registers.a & 1 > 0;
                self.registers.a = self.registers.a >> 1 | ((new_c as u8) << 7);
                self.registers.f = cpu_ops::set_flag(0, CpuFlag::C, new_c);
            }

            0x17 => {
                trace!("RLA");
                let new_c = self.registers.a & (1 << 7) > 0;
                let old_c = self.registers.f.has_flag(registers::Flag::C);
                self.registers.a = self.registers.a << 1 | old_c as u8;
                self.registers.f = cpu_ops::set_flag(0, CpuFlag::C, new_c);
            }

            0x1f => {
                trace!("RRA");
                let old_c = self.registers.f.has_flag(registers::Flag::C);
                let new_c = self.registers.a & 1 > 0;
                self.registers.a = self.registers.a >> 1 | ((old_c as u8) << 7);
                self.registers.f = cpu_ops::set_flag(0, CpuFlag::C, new_c);
            }

            0x3f => {
                trace!("CCF");
                let c = !self.registers.f.has_flag(registers::Flag::C);
                let mut f = cpu_ops::set_flag(self.registers.f, CpuFlag::C, c);
                f = cpu_ops::set_flag(f, CpuFlag::N, false);
                f = cpu_ops::set_flag(f, CpuFlag::H, false);
                self.registers.f = f;
            }

            0xcb => {
                panic!("cb operation should not run through this");
            }

            _ => {
                debug!("Info for debugging");
                self.memory.dump_tile_data();

                let time = time::Duration::from_secs(5);
                thread::sleep(time);
                panic!("missing operator {:#x}", op);
            }
        };
    }

    fn do_cb(&mut self, cb_instruction: u8) {
        match cb_instruction {
            // RLC
            0x00 => self.registers.f = self.registers.b.rlc(),
            0x01 => self.registers.f = self.registers.c.rlc(),
            0x02 => self.registers.f = self.registers.d.rlc(),
            0x03 => self.registers.f = self.registers.e.rlc(),
            0x04 => self.registers.f = self.registers.h.rlc(),
            0x05 => self.registers.f = self.registers.l.rlc(),
            0x06 => {
                let mut v = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = v.rlc();
                self.memory_write(self.registers.get_hl() as usize, v);
            }
            0x07 => self.registers.f = self.registers.a.rlc(),

            // RRC
            0x08 => self.registers.f = self.registers.b.rrc(),
            0x09 => self.registers.f = self.registers.c.rrc(),
            0x0a => self.registers.f = self.registers.d.rrc(),
            0x0b => self.registers.f = self.registers.e.rrc(),
            0x0c => self.registers.f = self.registers.h.rrc(),
            0x0d => self.registers.f = self.registers.l.rrc(),
            0x0e => {
                let mut v = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = v.rrc();
                self.memory_write(self.registers.get_hl() as usize, v);
            }
            0x0f => self.registers.f = self.registers.a.rrc(),

            // RR
            0x18 => Registers::rr(&mut self.registers.b, &mut self.registers.f),
            0x19 => Registers::rr(&mut self.registers.c, &mut self.registers.f),
            0x1a => Registers::rr(&mut self.registers.d, &mut self.registers.f),
            0x1b => Registers::rr(&mut self.registers.e, &mut self.registers.f),
            0x1c => Registers::rr(&mut self.registers.h, &mut self.registers.f),
            0x1d => Registers::rr(&mut self.registers.l, &mut self.registers.f),
            0x1e => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                Registers::rr(&mut value, &mut self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x1f => Registers::rr(&mut self.registers.a, &mut self.registers.f),

            // RL
            0x10 => Registers::rl(&mut self.registers.b, &mut self.registers.f),
            0x11 => Registers::rl(&mut self.registers.c, &mut self.registers.f),
            0x12 => Registers::rl(&mut self.registers.d, &mut self.registers.f),
            0x13 => Registers::rl(&mut self.registers.e, &mut self.registers.f),
            0x14 => Registers::rl(&mut self.registers.h, &mut self.registers.f),
            0x15 => Registers::rl(&mut self.registers.l, &mut self.registers.f),
            0x16 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                Registers::rl(&mut value, &mut self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x17 => Registers::rl(&mut self.registers.a, &mut self.registers.f),

            // SWAP
            0x30 => self.registers.f = self.registers.b.swap(),
            0x31 => self.registers.f = self.registers.c.swap(),
            0x32 => self.registers.f = self.registers.d.swap(),
            0x33 => self.registers.f = self.registers.e.swap(),
            0x34 => self.registers.f = self.registers.h.swap(),
            0x35 => self.registers.f = self.registers.l.swap(),
            0x36 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.swap();
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x37 => self.registers.f = self.registers.a.swap(),

            // SLA
            0x20 => self.registers.f = self.registers.b.sla(),
            0x21 => self.registers.f = self.registers.c.sla(),
            0x22 => self.registers.f = self.registers.d.sla(),
            0x23 => self.registers.f = self.registers.e.sla(),
            0x24 => self.registers.f = self.registers.h.sla(),
            0x25 => self.registers.f = self.registers.l.sla(),
            0x26 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.sla();
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x27 => self.registers.f = self.registers.a.sla(),

            // SRA
            0x28 => self.registers.f = self.registers.b.sra(),
            0x29 => self.registers.f = self.registers.c.sra(),
            0x2a => self.registers.f = self.registers.d.sra(),
            0x2b => self.registers.f = self.registers.e.sra(),
            0x2c => self.registers.f = self.registers.h.sra(),
            0x2d => self.registers.f = self.registers.l.sra(),
            0x2e => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.sra();
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x2f => self.registers.f = self.registers.a.sra(),

            // SRL
            0x38 => self.registers.f = self.registers.b.srl(),
            0x39 => self.registers.f = self.registers.c.srl(),
            0x3a => self.registers.f = self.registers.d.srl(),
            0x3b => self.registers.f = self.registers.e.srl(),
            0x3c => self.registers.f = self.registers.h.srl(),
            0x3d => self.registers.f = self.registers.l.srl(),
            0x3e => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.srl();
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x3f => self.registers.f = self.registers.a.srl(),

            // RES
            // 0 byte
            0x87 => self.registers.a.set_bit(0, false),
            0x80 => self.registers.b.set_bit(0, false),
            0x81 => self.registers.c.set_bit(0, false),
            0x82 => self.registers.d.set_bit(0, false),
            0x83 => self.registers.e.set_bit(0, false),
            0x84 => self.registers.h.set_bit(0, false),
            0x85 => self.registers.l.set_bit(0, false),

            0x8f => self.registers.a.set_bit(1, false),
            0x88 => self.registers.b.set_bit(1, false),
            0x89 => self.registers.c.set_bit(1, false),
            0x8a => self.registers.d.set_bit(1, false),
            0x8b => self.registers.e.set_bit(1, false),
            0x8c => self.registers.h.set_bit(1, false),
            0x8d => self.registers.l.set_bit(1, false),

            0x97 => self.registers.a.set_bit(2, false),
            0x90 => self.registers.b.set_bit(2, false),
            0x91 => self.registers.c.set_bit(2, false),
            0x92 => self.registers.d.set_bit(2, false),
            0x93 => self.registers.e.set_bit(2, false),
            0x94 => self.registers.h.set_bit(2, false),
            0x95 => self.registers.l.set_bit(2, false),

            0x9f => self.registers.a.set_bit(3, false),
            0x98 => self.registers.b.set_bit(3, false),
            0x99 => self.registers.c.set_bit(3, false),
            0x9a => self.registers.d.set_bit(3, false),
            0x9b => self.registers.e.set_bit(3, false),
            0x9c => self.registers.h.set_bit(3, false),
            0x9d => self.registers.l.set_bit(3, false),

            0xa7 => self.registers.a.set_bit(4, false),
            0xa0 => self.registers.b.set_bit(4, false),
            0xa1 => self.registers.c.set_bit(4, false),
            0xa2 => self.registers.d.set_bit(4, false),
            0xa3 => self.registers.e.set_bit(4, false),
            0xa4 => self.registers.h.set_bit(4, false),
            0xa5 => self.registers.l.set_bit(4, false),

            0xaf => self.registers.a.set_bit(5, false),
            0xa8 => self.registers.b.set_bit(5, false),
            0xa9 => self.registers.c.set_bit(5, false),
            0xaa => self.registers.d.set_bit(5, false),
            0xab => self.registers.e.set_bit(5, false),
            0xac => self.registers.h.set_bit(5, false),
            0xad => self.registers.l.set_bit(5, false),

            0xb7 => self.registers.a.set_bit(6, false),
            0xb0 => self.registers.b.set_bit(6, false),
            0xb1 => self.registers.c.set_bit(6, false),
            0xb2 => self.registers.d.set_bit(6, false),
            0xb3 => self.registers.e.set_bit(6, false),
            0xb4 => self.registers.h.set_bit(6, false),
            0xb5 => self.registers.l.set_bit(6, false),

            0xbf => self.registers.a.set_bit(7, false),
            0xb8 => self.registers.b.set_bit(7, false),
            0xb9 => self.registers.c.set_bit(7, false),
            0xba => self.registers.d.set_bit(7, false),
            0xbb => self.registers.e.set_bit(7, false),
            0xbc => self.registers.h.set_bit(7, false),
            0xbd => self.registers.l.set_bit(7, false),

            0x86 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(0, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x8e => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(1, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x96 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(2, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x9e => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(3, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xa6 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(4, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xae => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(5, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xb6 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(6, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xbe => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(7, false);
                self.memory_write(self.registers.get_hl() as usize, value);
            }

            // SET
            0xc7 => self.registers.a.set_bit(0, true),
            0xc0 => self.registers.b.set_bit(0, true),
            0xc1 => self.registers.c.set_bit(0, true),
            0xc2 => self.registers.d.set_bit(0, true),
            0xc3 => self.registers.e.set_bit(0, true),
            0xc4 => self.registers.h.set_bit(0, true),
            0xc5 => self.registers.l.set_bit(0, true),

            0xcf => self.registers.a.set_bit(1, true),
            0xc8 => self.registers.b.set_bit(1, true),
            0xc9 => self.registers.c.set_bit(1, true),
            0xca => self.registers.d.set_bit(1, true),
            0xcb => self.registers.e.set_bit(1, true),
            0xcc => self.registers.h.set_bit(1, true),
            0xcd => self.registers.l.set_bit(1, true),

            0xd7 => self.registers.a.set_bit(2, true),
            0xd0 => self.registers.b.set_bit(2, true),
            0xd1 => self.registers.c.set_bit(2, true),
            0xd2 => self.registers.d.set_bit(2, true),
            0xd3 => self.registers.e.set_bit(2, true),
            0xd4 => self.registers.h.set_bit(2, true),
            0xd5 => self.registers.l.set_bit(2, true),

            0xdf => self.registers.a.set_bit(3, true),
            0xd8 => self.registers.b.set_bit(3, true),
            0xd9 => self.registers.c.set_bit(3, true),
            0xda => self.registers.d.set_bit(3, true),
            0xdb => self.registers.e.set_bit(3, true),
            0xdc => self.registers.h.set_bit(3, true),
            0xdd => self.registers.l.set_bit(3, true),

            0xe7 => self.registers.a.set_bit(4, true),
            0xe0 => self.registers.b.set_bit(4, true),
            0xe1 => self.registers.c.set_bit(4, true),
            0xe2 => self.registers.d.set_bit(4, true),
            0xe3 => self.registers.e.set_bit(4, true),
            0xe4 => self.registers.h.set_bit(4, true),
            0xe5 => self.registers.l.set_bit(4, true),

            0xef => self.registers.a.set_bit(5, true),
            0xe8 => self.registers.b.set_bit(5, true),
            0xe9 => self.registers.c.set_bit(5, true),
            0xea => self.registers.d.set_bit(5, true),
            0xeb => self.registers.e.set_bit(5, true),
            0xec => self.registers.h.set_bit(5, true),
            0xed => self.registers.l.set_bit(5, true),

            0xf7 => self.registers.a.set_bit(6, true),
            0xf0 => self.registers.b.set_bit(6, true),
            0xf1 => self.registers.c.set_bit(6, true),
            0xf2 => self.registers.d.set_bit(6, true),
            0xf3 => self.registers.e.set_bit(6, true),
            0xf4 => self.registers.h.set_bit(6, true),
            0xf5 => self.registers.l.set_bit(6, true),

            0xff => self.registers.a.set_bit(7, true),
            0xf8 => self.registers.b.set_bit(7, true),
            0xf9 => self.registers.c.set_bit(7, true),
            0xfa => self.registers.d.set_bit(7, true),
            0xfb => self.registers.e.set_bit(7, true),
            0xfc => self.registers.h.set_bit(7, true),
            0xfd => self.registers.l.set_bit(7, true),
            0xc6 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(0, true);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xce => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(1, true);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xd6 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(2, true);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xde => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(3, true);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xe6 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(4, true);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xee => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(5, true);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xf6 => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(6, true);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0xfe => {
                let mut value = self.memory_read(self.registers.get_hl() as usize);
                value.set_bit(7, true);
                self.memory_write(self.registers.get_hl() as usize, value);
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

            0x57 => self.registers.f = self.registers.a.bit(2, self.registers.f),
            0x50 => self.registers.f = self.registers.b.bit(2, self.registers.f),
            0x51 => self.registers.f = self.registers.c.bit(2, self.registers.f),
            0x52 => self.registers.f = self.registers.d.bit(2, self.registers.f),
            0x53 => self.registers.f = self.registers.e.bit(2, self.registers.f),
            0x54 => self.registers.f = self.registers.h.bit(2, self.registers.f),
            0x55 => self.registers.f = self.registers.l.bit(2, self.registers.f),

            0x5f => self.registers.f = self.registers.a.bit(3, self.registers.f),
            0x58 => self.registers.f = self.registers.b.bit(3, self.registers.f),
            0x59 => self.registers.f = self.registers.c.bit(3, self.registers.f),
            0x5a => self.registers.f = self.registers.d.bit(3, self.registers.f),
            0x5b => self.registers.f = self.registers.e.bit(3, self.registers.f),
            0x5c => self.registers.f = self.registers.h.bit(3, self.registers.f),
            0x5d => self.registers.f = self.registers.l.bit(3, self.registers.f),

            0x67 => self.registers.f = self.registers.a.bit(4, self.registers.f),
            0x60 => self.registers.f = self.registers.b.bit(4, self.registers.f),
            0x61 => self.registers.f = self.registers.c.bit(4, self.registers.f),
            0x62 => self.registers.f = self.registers.d.bit(4, self.registers.f),
            0x63 => self.registers.f = self.registers.e.bit(4, self.registers.f),
            0x64 => self.registers.f = self.registers.h.bit(4, self.registers.f),
            0x65 => self.registers.f = self.registers.l.bit(4, self.registers.f),

            0x6f => self.registers.f = self.registers.a.bit(5, self.registers.f),
            0x68 => self.registers.f = self.registers.b.bit(5, self.registers.f),
            0x69 => self.registers.f = self.registers.c.bit(5, self.registers.f),
            0x6a => self.registers.f = self.registers.d.bit(5, self.registers.f),
            0x6b => self.registers.f = self.registers.e.bit(5, self.registers.f),
            0x6c => self.registers.f = self.registers.h.bit(5, self.registers.f),
            0x6d => self.registers.f = self.registers.l.bit(5, self.registers.f),

            0x77 => self.registers.f = self.registers.a.bit(6, self.registers.f),
            0x70 => self.registers.f = self.registers.b.bit(6, self.registers.f),
            0x71 => self.registers.f = self.registers.c.bit(6, self.registers.f),
            0x72 => self.registers.f = self.registers.d.bit(6, self.registers.f),
            0x73 => self.registers.f = self.registers.e.bit(6, self.registers.f),
            0x74 => self.registers.f = self.registers.h.bit(6, self.registers.f),
            0x75 => self.registers.f = self.registers.l.bit(6, self.registers.f),

            0x7f => self.registers.f = self.registers.a.bit(7, self.registers.f),
            0x78 => self.registers.f = self.registers.b.bit(7, self.registers.f),
            0x79 => self.registers.f = self.registers.c.bit(7, self.registers.f),
            0x7a => self.registers.f = self.registers.d.bit(7, self.registers.f),
            0x7b => self.registers.f = self.registers.e.bit(7, self.registers.f),
            0x7c => self.registers.f = self.registers.h.bit(7, self.registers.f),
            0x7d => self.registers.f = self.registers.l.bit(7, self.registers.f),

            0x46 => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(0, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x4e => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(1, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x56 => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(2, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x5e => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(3, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x66 => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(4, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x6e => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(5, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x76 => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(6, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }
            0x7e => {
                let value = self.memory_read(self.registers.get_hl() as usize);
                self.registers.f = value.bit(7, self.registers.f);
                self.memory_write(self.registers.get_hl() as usize, value);
            }

            _ => {
                debug!("Info for debugging");
                self.memory.dump_tile_data();

                let ten_millis = time::Duration::from_secs(10);
                thread::sleep(ten_millis);
                panic!("Missing cb {:#x}", cb_instruction)
            }
        }
    }

    fn set_gpu_mode(&mut self, mode: gpu::Mode) {
        self.gpu_mode = mode;
        self.memory.io_registers.lcd_status &= !3; // wipe 2 first digits
        self.memory.io_registers.lcd_status |= mode as u8;
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .target(env_logger::Target::Stdout)
        .init();

    let path = "PokemonRed.gb";
    // let path = "Adventure Island II - Aliens in Paradise (USA, Europe).gb";
    // let path = "Legend of Zelda, The - Link's Awakening (USA, Europe) (Rev 2).gb";

    // Testsuites
    // let path = "test/01-special.gb";
    // let path = "test/02-interrupts.gb";
    // let path = "test/03-op sp,hl.gb";
    // let path = "test/04-op r,imm.gb";
    // let path = "test/05-op rp.gb";
    // let path = "test/06-ld r,r.gb";
    // let path = "test/07-jr,jp,call,ret,rst.gb";
    // let path = "test/08-misc instrs.gb";
    // let path = "test/09-op r,r.gb";
    // let path = "test/10-bit ops.gb";
    // let path = "test/11-op a,(hl).gb";

    // untested
    // let path = "test/instr_timing.gb";
    // let path = "test/interrupt_time.gb";
    // let path = ("test/mem_timing_1.gb");
    // let path = ("test/mem_timing_2.gb");
    let path = "test/Acid2 Test for Game Boy.gb";

    let mut cpu = GameBoy {
        cartridge: Cartridge::default(path),
        registers: Registers::default(),
        memory: Memory::default(),
        joypad: Joypad::default(),

        timer: Timer::default(),

        ime: false,
        set_ei: false,

        halt: false,
        cpu_cycles: 0,
        gpu_mode: gpu::Mode::Two, //todo should this set the ff41?
        display: Display::default(),
        // lcd_prev_state: true,
        debug_counter: 0,
    };

    loop {
        // println!("Iteration {}", _i);
        cpu.step();
    }
    cpu.memory.dump_tile_data();
}
