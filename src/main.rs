use std::{fs::File, io::{Read, self}, str};

#[derive(Copy, Clone)]
pub enum CpuFlag
{
    /// carry
    C = 0b00010000,
    /// half-carry
    H = 0b00100000,
    /// substraction
    N = 0b01000000,
    /// zero - indicates that result was zero
    Z = 0b10000000,
}

struct Registers {
    a: u8, f: u8,
    b: u8, c: u8,
    d: u8, e: u8,
    h: u8, l: u8,
    pc: u16,
    sp: u16,
}

impl Registers {
    fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = (v & 0x00FF) as u8;
    }

    fn get_hl(&self) -> u16 {
        println!("H={:#x}, L={:#x}", self.h, self.l);
        println!("HL={:#x}", (self.h as u16) << 8 | self.l as u16);
        (self.h as u16) << 8 | self.l as u16
    }

    fn step_pc(&mut self) -> usize{
        let current = self.pc;
        self.pc += 1;
        current as usize
    }

    fn set_pc(&mut self, loc: u16){
        self.pc = loc;
    }

    fn has_flag(f:u8, flag: CpuFlag) -> bool {
        // println!("{:#b} - {:#b}", f, flag as u8);
        (f & (flag as u8)) > 0
    }

    fn set_flag(f:u8, flag: CpuFlag, value: bool) -> u8 {
        if value {
            f | flag as u8
        } else {
            f & !(flag as u8)
        }
    }

    fn xor(a:u8, b:u8) -> (u8, u8){
        let result = a ^ b;
        let mut f = Registers::set_flag(0x0, CpuFlag::Z, result == 0);
        f = Registers::set_flag(f, CpuFlag::N, false);
        f = Registers::set_flag(f, CpuFlag::H, false);
        f = Registers::set_flag(f, CpuFlag::C, false);
        (result, f)
    }
}

struct Cpu <'a> {
    registers: &'a mut Registers,
    rom: Vec<u8>,
    ram: &'a mut Vec<u8>,
}

fn load_rom() -> io::Result<Vec<u8>> {

    let mut f = File::open("PokemonRed.gb")?;
    let mut buffer = Vec::new();

    // read the whole file
    f.read_to_end(&mut buffer)?;

    Ok(buffer)
}

impl <'a> Cpu <'a>{
    fn step(&mut self) {
        let location = self.registers.step_pc();
        println!("Running location {}", location);

        
        if location > 0x3FFF {
            println!("moving outside of bank 1??")
        }
        if location > 0x7FFF {
            panic!("moving outside of bank 2??")
        }

        self.find_operator(location);
    }

    fn get_memory_location(&self, location: usize) -> u8 {
        if location<=0x7FFF {
            self.rom[location as usize]
        } else {
            panic!("Location not in ROM: {:#x}", location)
        }
    }

    fn write_memory_location(&mut self, location: usize, value: u8) {
        println!("Writing to Memory Location: {:#x}", location );
        if location<=0x7FFF {
            panic!("how can I write to ROM?! {:#x}", location)
        } else if location < 0xe0000 && location >= 0xc000 {
            println!("Writting to internal RAM");
            self.ram[location - 0xc000] = value;
        }else {
            panic!("Need to handle memory write to: {:#x}", location)
        }
    }

    fn get_u16(&mut self) -> u16 {
        let location = self.registers.step_pc();
        println!("Reading location {}", location);
        let v1 = self.rom[location] as u16;
        let location = self.registers.step_pc();
        println!("Reading location 2 {}", location);
        let v2 = self.rom[location] as u16;
        v2 << 8 | v1
    }

    fn get_u8(&mut self) -> u8 {
        let location = self.registers.step_pc();
        self.rom[location]
    }
    
    fn find_operator(&mut self, location: usize) {
        let op = self.rom[location];
        match op {
            0x0 => println!("NOP"),

            0xc3 => {
                let v = self.get_u16();
                self.registers.set_pc(v);
                println!("JP nn --> {:#x}", v);
            },

            // JR n
            0x18 => {
                let steps = self.get_u8() as i16;
                let new_location = self.registers.pc  as i32 + steps as i32;
                self.registers.set_pc(new_location as u16);
                println!("JR n (jump {} -> {:#x})", steps, new_location);
            }

            // JR cc,n
            0x28 => {
                println!("JR cc,n");
                let steps = self.get_u8() as i16;
                println!("{:#b}", self.registers.f);
                if Registers::has_flag(self.registers.f, CpuFlag::Z) {
                    let new_location = (self.registers.pc as i16 + steps) as u16;
                    println!("Current location: {}, next: {}", self.registers.pc, new_location);
                    self.registers.set_pc(new_location);
                    panic!("untested jump");
                }

            }

            0x61 => {
                println!("LD H,C");
                self.registers.h = self.registers.c;
            }

            // LD x, A
            0x47 => {
                println!("LD B,A");
                self.registers.b = self.registers.a;
            }
            0x4f => {
                println!("LD C,A");
                self.registers.c = self.registers.a;
            }
            0x57 => {
                println!("LD D,A");
                self.registers.d = self.registers.a;
            }
            0x5f => {
                println!("LD E,A");
                self.registers.e = self.registers.a;
            }
            0x67 => {
                println!("LD H,A");
                self.registers.h = self.registers.a;
            }
            0x6f => {
                println!("LD L,A");
                self.registers.l = self.registers.a;
            }

            0xea => {
                println!("LD (nn),A");
                let target = self.get_u16();
                self.write_memory_location(target as usize, self.registers.a);
            }

            // LD r1,r2
            0x7e => {
                println!("LD A, (HL)");
                self.registers.a = self.get_memory_location(self.registers.get_hl() as usize);
            }

            // LDI (HL), A
            0x22 => {
                println!("LDI (HL), A");
                self.write_memory_location(self.registers.get_hl() as usize, self.registers.a);
                self.registers.set_hl(self.registers.get_hl()+1)
            }

            // 
            0xfa => {
                println!("LD A, nn");
                let source = self.get_u16();
                self.registers.a = self.get_memory_location(source as usize);
            }

            // SUB n
            0x90 => {
                println!("SUB B");
                let mut f = Registers::set_flag(self.registers.f, CpuFlag::C, self.registers.a < self.registers.b);
                f = Registers::set_flag(f, CpuFlag::H, (self.registers.b & 0x0f) > (self.registers.a & 0x0f));
                self.registers.a = self.registers.a.wrapping_sub(self.registers.b);
                f = Registers::set_flag(f, CpuFlag::Z, self.registers.a == 0);
                f = Registers::set_flag(f, CpuFlag::N, true);
                self.registers.f = f;
            }

            0xd6 => {
                println!("SUB #");
                let b = self.get_u8();
                let mut f = Registers::set_flag(self.registers.f, CpuFlag::C, self.registers.a < b);
                f = Registers::set_flag(f, CpuFlag::H, (b & 0x0f) > (self.registers.a & 0x0f));
                self.registers.a = self.registers.a.wrapping_sub(b);
                f = Registers::set_flag(f, CpuFlag::Z, self.registers.a == 0);
                f = Registers::set_flag(f, CpuFlag::N, true);
                self.registers.f = f;
            }

            // DEC
            0x25 => {
                println!("DEC H");
                let mut f = self.registers.f;
                f = Registers::set_flag(f, CpuFlag::H, (self.registers.a & 0x0f) == 0 );
                self.registers.a = self.registers.a.wrapping_sub(1);
                f = Registers::set_flag(f, CpuFlag::Z, self.registers.a == 0);
                f = Registers::set_flag(f, CpuFlag::N, true);
                self.registers.f = f;
            }

            // XOR n
            0xaf => {
                println!("XOR A");
                (self.registers.a, self.registers.f) = Registers::xor(self.registers.a, self.registers.a);
            }

            0xa8 => {
                println!("XOR B");
                (self.registers.a, self.registers.f) = Registers::xor(self.registers.a, self.registers.b);
            }

            0xa9 => {
                println!("XOR C");
                (self.registers.a, self.registers.f) = Registers::xor(self.registers.a, self.registers.c);
            }

            0xaa => {
                println!("XOR D");
                (self.registers.a, self.registers.f) = Registers::xor(self.registers.a, self.registers.d);
            }

            0xab => {
                println!("XOR E");
                (self.registers.a, self.registers.f) = Registers::xor(self.registers.a, self.registers.e);
            }

            0xac => {
                println!("XOR H");
                (self.registers.a, self.registers.f) = Registers::xor(self.registers.a, self.registers.h);
            }

            0xad => {
                println!("XOR L");
                (self.registers.a, self.registers.f) = Registers::xor(self.registers.a, self.registers.l);
            }

            // CP n
            0xfe => {
                let n = self.get_u8();
                println!("CP # -> {}", n);
                let mut f = Registers::set_flag(self.registers.f, CpuFlag::C, self.registers.a < n);
                f = Registers::set_flag(f, CpuFlag::H, (n & 0x0f) > (self.registers.a & 0x0f));
                f = Registers::set_flag(f, CpuFlag::Z, self.registers.a == 0);
                f = Registers::set_flag(f, CpuFlag::N, true);
                self.registers.f = f;
            }

            

            // MISC

            0xcb => {
                self.do_cb();
            }

            _ => panic!("missing operator {:#x}", op),
        };


    }


    fn do_cb(&mut self) {
        let op = self.get_u8();
        match op {
            0x37 => {
                println!("SWAP nimble A");
                self.registers.a = (self.registers.a >> 4) | (self.registers.a<< 4);
                let mut f = Registers::set_flag(self.registers.f, CpuFlag::Z, self.registers.a==0);
                f = Registers::set_flag(f, CpuFlag::N, false);
                f = Registers::set_flag(f, CpuFlag::H, false);
                f = Registers::set_flag(f, CpuFlag::C, false);
                self.registers.f = f;
            }

            // SRA n
            0x28 => {
                println!("SRA B");
                let c = self.registers.b | 0x01;
                let msb = self.registers.b | (1<<7);
                let shifted = self.registers.b >> 1;

                println!("FROM: {:#x}", self.registers.b);

                let mut f = Registers::set_flag(self.registers.f, CpuFlag::C, c==1);
                f = Registers::set_flag(f, CpuFlag::H, false);
                f = Registers::set_flag(f, CpuFlag::Z, self.registers.b == 0);
                f = Registers::set_flag(f, CpuFlag::N, false);
                self.registers.f = f;
                self.registers.b = shifted | msb;
                println!("TO: {:#x}", self.registers.b);
                panic!("not implememented SRA properly")

            }

            _ => panic!("Missing cb {:#x}", op),
        }
    }
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
    if buffer.len() < 0x150 { panic!("Rom size to small"); }
    
    let title = str::from_utf8( &buffer[0x134..0x142]).unwrap();

    println!("Title = {}", title);

    println!("Type = {}", buffer[0x143]);
    println!("GB/SGB Indicator = {}", buffer[0x146]);
    println!("Cartridge type = {}", buffer[0x147]);
    let rom_size = buffer[0x148];
    println!("ROM size = {}", rom_size);
    println!("RAM size = {}", buffer[0x149]);

    let expected_rom_size = 32 * (2u32.pow(rom_size as u32) )* 1024u32;

    if buffer.len() as u32 != expected_rom_size {
        panic!("Wrong length found. Expected {} - Found {}", expected_rom_size ,buffer.len());
    } else {
        println!("ROM size Bytes = {}", expected_rom_size);
    }

    let mut cpu = Cpu{
        registers: &mut Registers {  // Classic
            pc: 0x100,
            sp: 0xFFFE,
            a: 0x01, // $01-GB/SGB, $FF-GBP, $11-GBC
            l: 0x4d, 
            f: 0xB0, 
            b: 0x00, 
            c: 0x13, 
            d: 0x00, 
            e: 0xd8,
            h: 0x01 },
        rom: buffer,
        ram: &mut vec![0; 8192],
    };

    for _i in 0..20{
        cpu.step();
    }
    
}
