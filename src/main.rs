use std::{fs::File, io::{Read, self}, str, default};

struct Registers {
    pc: u16
}

impl Registers {
    fn step_pc(&mut self) -> usize{
        let current = self.pc;
        self.pc += 1;
        current as usize
    }

    fn set_pc(&mut self, loc: u16){
        self.pc = loc;
    }
}

struct Cpu <'a> {
    registers: &'a mut Registers,
    rom: Vec<u8>,
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

        
        if location > 0x3FFF {
            panic!("moving outside of bank 1??")
        }

        self.find_operator(location);
    }
    
    fn find_operator(&mut self, location: usize) {
        let op = self.rom[location];
        match op {
            0x0 => println!("NOP"),

            0xc3 => {
                let v1 = self.rom[location] as u16;
                let v2 = self.rom[location+1] as u16;
                self.registers.set_pc(v1  | v2 << 8);
                println!("JP nn --> {:#x}-{:#x} -> {:#x}", v1, v2, v1  | v2 << 8);
            },

            _ => panic!("missing operator {:#x}", op),
        };
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
        registers: &mut Registers { pc: 0x100 },
        rom: buffer,
    };

    for _i in 0..10{
        cpu.step();
    }
    
}
