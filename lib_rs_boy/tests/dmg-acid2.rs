#[cfg(test)]
mod test {
    use image::{DynamicImage, ImageReader};
    use lib_rs_boy::gameboy::GameBoy;
    use lib_rs_boy::io::files::load_file;
    use std::path::Path;

    const TEST_DIR_PATH: &str = "tests/game-boy-test-roms-v7.0/dmg-acid2/";

    #[test]
    fn acid2() {
        let rom = load_file(Path::new(TEST_DIR_PATH).join("dmg-acid2.gb").as_path()).unwrap();

        let img: DynamicImage = ImageReader::open(Path::new(TEST_DIR_PATH).join("dmg-acid2-dmg.png").as_path()).unwrap()
            .decode().unwrap();

        let mut gb = GameBoy::new();
        gb.load_rom(rom);

        let raw_rgb = img.into_rgb8();
        let rgb_vec: Vec<u32> = raw_rgb.chunks(3)
            .map(|chunk| {
                let r = chunk[0] as u32;
                let g = chunk[1] as u32;
                let b = chunk[2] as u32;
                (r << 16) | (g << 8) | b
            })
            .collect();


        loop {
            if gb.memory_read(gb.registers.pc as usize) == 0x40 {
                // LD B,B
                break;
            }
            gb.step();
        }

        assert_eq!(gb.display.engine.screen.len(), rgb_vec.len());
        assert_eq!(gb.display.engine.screen, rgb_vec);
    }
}

