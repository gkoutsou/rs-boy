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
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

pub enum LcdStatusFlag {
    LcdEnabled = 1 << 7,
    WindowTileMapArea = 1 << 6,
    WindowEnabled = 1 << 5,
    TileDataArea = 1 << 4,
    BGTileMapArea = 1 << 3,
    ObjectSize = 1 << 2,
    ObjectEnabled = 1 << 1,
    BgWindowEnabled = 1 << 0,
}

#[derive(Debug)]
pub struct Tile {
    pub y: u8,
    pub x: u8,
    pub tile_index: u8,
    /// 7 - Priority: 0 = No, 1 = BG and Window colors 1–3 are drawn over this OBJ
    /// 6 - Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
    /// 5 - X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
    /// 4- DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
    pub flags: u8,
}

impl Tile {
    pub fn object_in_scanline(&self, scanline: u8, double_size: bool) -> bool {
        let size = if !double_size { 8 } else { 16 };

        let y = self.y as i16;
        let scan = scanline as i16;
        // todo this probably should return false if double_size but scanline is outside
        if scan < y - 16 + size && scan >= y - 16 {
            return true;
        }
        false
    }

    pub fn is_x_flipped(&self) -> bool {
        return self.flags & 1 << 5 > 0;
    }

    pub fn is_y_flipped(&self) -> bool {
        return self.flags & 1 << 6 > 0;
    }

    pub fn has_priority(&self) -> bool {
        return self.flags & 1 << 7 == 0;
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

#[cfg(test)]
mod tests {
    use crate::gpu::Tile;

    #[test]
    fn object_in_scanline() {
        assert_eq!(Tile::new(0, 7, 1, 1).object_in_scanline(0, false), false);
        assert_eq!(Tile::new(2, 7, 1, 1).object_in_scanline(0, false), false);

        let t = Tile::new(16, 7, 1, 1);
        for i in 0..8 {
            assert_eq!(t.object_in_scanline(i, false), true, "iteration {}", i);
        }
        assert_eq!(t.object_in_scanline(9, false), false);

        let t = Tile::new(144, 7, 1, 1);
        for i in 0..8 {
            assert_eq!(
                t.object_in_scanline(144 - 16 + i, false),
                true,
                "iteration {}",
                144 + i
            );
        }
        assert_eq!(t.object_in_scanline(144 - 16 + 8, false), false);
    }

    #[test]
    fn object_in_scanline_double() {
        assert_eq!(Tile::new(0, 7, 1, 1).object_in_scanline(0, true), false);

        assert_eq!(Tile::new(2, 7, 1, 1).object_in_scanline(0, true), true);
        assert_eq!(Tile::new(2, 7, 1, 1).object_in_scanline(1, true), true);
        assert_eq!(Tile::new(2, 7, 1, 1).object_in_scanline(2, true), false);

        let t = Tile::new(16, 7, 1, 1);
        for i in 0..16 {
            assert_eq!(t.object_in_scanline(i, true), true, "iteration {}", i);
        }
        assert_eq!(t.object_in_scanline(17, true), false);

        let t = Tile::new(144, 7, 1, 1);
        for i in 0..16 {
            assert_eq!(
                t.object_in_scanline(144 - 16 + i, true),
                true,
                "iteration {}",
                144 + i
            );
        }
        assert_eq!(t.object_in_scanline(144 - 16 + 16, false), false);
    }
}
