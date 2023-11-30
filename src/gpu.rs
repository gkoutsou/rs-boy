// use crate::Memory;

// pub struct GPU<'a> {
//     // tile_data: &'a mut Vec<u8>,
//     // tile_maps: &'a mut Vec<u8>,

//     // oam: &'a mut Vec<u8>,
//     pub memory: &'a mut Memory,
// }

// impl<'a> GPU<'a> {
//     fn step(&mut self) {
//         self.memory.io_registers.scanline += 1;
//         if self.memory.io_registers.scanline == 144 {
//             self.memory.io_registers.enable_video_interrupt();
//         }
//         if self.memory.io_registers.scanline > 153 {
//             self.memory.io_registers.scanline = 0;
//         }
//     }
// }

#[derive(Copy, Clone)]
pub enum Mode {
    Zero,
    One,
    Two,
    Three,
}

#[derive(Debug)]
pub struct Tile {
    y: u8,
    x: u8,
    tile_index: u8,
    flags: u8,
}

impl Tile {
    pub fn object_in_scanline(&self, scanline: u8, double_size: bool) -> bool {
        let size = if double_size { 16 } else { 8 };
        let y = self.y as i16;
        let scan = scanline as i16;
        // todo totally wrong
        if scan < y - 16 + size && scan >= y - 16 {
            return true;
        }
        false
    }

    pub fn new(y: u8, x: u8, tile_index: u8, flags: u8) -> Tile {
        Tile {
            y: y,
            x: x,
            tile_index: tile_index,
            flags: flags,
        }
    }
}
