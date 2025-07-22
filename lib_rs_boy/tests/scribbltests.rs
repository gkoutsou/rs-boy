#[cfg(test)]
mod test {
    use image::{DynamicImage, ImageReader};
    use lib_rs_boy::gameboy::GameBoy;
    use lib_rs_boy::io::files::load_file;
    use std::path::Path;
    use test_case::test_case;

    const TEST_DIR_PATH: &str = "tests/game-boy-test-roms-v7.0/scribbltests";
    #[test_case("lycscx/lycscx.gb", "lycscx/lycscx-cgb-dmg.png" ; "lycscx")]
    #[test_case("lycscy/lycscy.gb", "lycscy/lycscy-cgb-dmg.png" ; "lycscy")]
    #[test_case("palettely/palettely.gb", "palettely/palettely-dmg.png" ; "palettely")]
    #[test_case("scxly/scxly.gb", "scxly/scxly-dmg.png" ; "scxly")]
    #[test_case("statcount/statcount-auto.gb", "statcount/statcount_auto-cgb-dmg.png" ; "statcount-auto")]
    // fairylake - manual test
    // statcount - manual test
    // winpos    - manual test
    fn scribbletests(rom: &str, result_path: &str) {

        let img: DynamicImage = ImageReader::open(Path::new(TEST_DIR_PATH).join(result_path).as_path()).unwrap()
            .decode().unwrap();

        let raw_rgb = img.into_rgb8();
        let rgb_vec: Vec<u32> = raw_rgb.chunks(3)
            .map(|chunk| {
                let r = chunk[0] as u32;
                let g = chunk[1] as u32;
                let b = chunk[2] as u32;
                (r << 16) | (g << 8) | b
            })
            .collect();


        let rom = load_file(Path::new(TEST_DIR_PATH).join(rom).as_path()).unwrap();
        let mut gb = GameBoy::new();
        gb.load_rom(rom);

        let mut frame_counter = 0;
        loop {
            let render = gb.step();
            if render {
                frame_counter += 1;
            }

            // Arbitrarily stopping after 3 frames
            if frame_counter >= 3 {
                break
            }
        }
        // println!("Pixels: {:?}", &rgb_vec[..20]); // Show the first 10 as example
        // println!("Screen: {:?}", &gb.display.engine.screen[..20]); // Show the first 10 as example

        assert_eq!(gb.display.engine.screen.len(), rgb_vec.len());
        assert_eq!(gb.display.engine.screen, rgb_vec)
    }

}
