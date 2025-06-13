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
        self.flags & 1 << 5 > 0
    }

    pub fn is_y_flipped(&self) -> bool {
        self.flags & 1 << 6 > 0
    }

    pub fn has_priority(&self) -> bool {
        self.flags & 1 << 7 == 0
    }

    pub fn new(y: u8, x: u8, tile_index: u8, flags: u8) -> Tile {
        Tile {
            y,
            x,
            tile_index,
            flags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Tile;

    #[test]
    fn object_in_scanline() {
        assert!(!Tile::new(0, 7, 1, 1).object_in_scanline(0, false));
        assert!(!Tile::new(2, 7, 1, 1).object_in_scanline(0, false));

        let t = Tile::new(16, 7, 1, 1);
        for i in 0..8 {
            assert!(t.object_in_scanline(i, false), "iteration {}", i);
        }
        assert!(!t.object_in_scanline(9, false));

        let t = Tile::new(144, 7, 1, 1);
        for i in 0..8 {
            assert!(
                t.object_in_scanline(144 - 16 + i, false),
                "iteration {}",
                144 + i
            );
        }
        assert!(!t.object_in_scanline(144 - 16 + 8, false));
    }

    #[test]
    fn object_in_scanline_double() {
        assert!(!Tile::new(0, 7, 1, 1).object_in_scanline(0, true));

        assert!(Tile::new(2, 7, 1, 1).object_in_scanline(0, true));
        assert!(Tile::new(2, 7, 1, 1).object_in_scanline(1, true));
        assert!(!Tile::new(2, 7, 1, 1).object_in_scanline(2, true));

        let t = Tile::new(16, 7, 1, 1);
        for i in 0..16 {
            assert!(t.object_in_scanline(i, true), "iteration {}", i);
        }
        assert!(!t.object_in_scanline(17, true));

        let t = Tile::new(144, 7, 1, 1);
        for i in 0..16 {
            assert!(
                t.object_in_scanline(144 - 16 + i, true),
                "iteration {}",
                144 + i
            );
        }
        assert!(!t.object_in_scanline(144 - 16 + 16, false));
    }
}
