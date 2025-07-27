#[cfg(test)]
mod test {
    use image::{DynamicImage, ImageBuffer, ImageReader, Rgb};
    use lib_rs_boy::gameboy::GameBoy;
    use lib_rs_boy::io::files::load_file;
    use std::path::Path;
    use test_case::test_case;

    const TEST_DIR_PATH: &str = "tests/game-boy-test-roms-v7.0/mealybug-tearoom-tests/ppu";
    #[test_case("m2_win_en_toggle" ; "m2_win_en_toggle")]
    #[test_case("m3_bgp_change_sprites" ; "m3_bgp_change_sprites")]
    #[test_case("m3_bgp_change" ; "m3_bgp_change")]
    #[test_case("m3_lcdc_bg_en_change" ; "m3_lcdc_bg_en_change")]
    #[test_case("m3_lcdc_bg_en_change2" ; "m3_lcdc_bg_en_change2")]
    #[test_case("m3_lcdc_bg_map_change" ; "m3_lcdc_bg_map_change")]
    #[test_case("m3_lcdc_bg_map_change2" ; "m3_lcdc_bg_map_change2")]
    #[test_case("m3_lcdc_obj_en_change_variant" ; "m3_lcdc_obj_en_change_variant")]
    #[test_case("m3_lcdc_obj_en_change" ; "m3_lcdc_obj_en_change")]
    #[test_case("m3_lcdc_obj_size_change_scx" ; "m3_lcdc_obj_size_change_scx")]
    #[test_case("m3_lcdc_obj_size_change" ; "m3_lcdc_obj_size_change")]
    #[test_case("m3_lcdc_tile_sel_change" ; "m3_lcdc_tile_sel_change")]
    #[test_case("m3_lcdc_tile_sel_change2" ; "m3_lcdc_tile_sel_change2")]
    #[test_case("m3_lcdc_tile_sel_win_change" ; "m3_lcdc_tile_sel_win_change")]
    #[test_case("m3_lcdc_tile_sel_win_change2" ; "m3_lcdc_tile_sel_win_change2")]
    #[test_case("m3_lcdc_win_en_change_multiple_wx" ; "m3_lcdc_win_en_change_multiple_wx")]
    #[test_case("m3_lcdc_win_en_change_multiple" ; "m3_lcdc_win_en_change_multiple")]
    #[test_case("m3_lcdc_win_map_change" ; "m3_lcdc_win_map_change")]
    #[test_case("m3_lcdc_win_map_change2" ; "m3_lcdc_win_map_change2")]
    #[test_case("m3_obp0_change" ; "m3_obp0_change")]
    #[test_case("m3_scx_high_5_bits_change2" ; "m3_scx_high_5_bits_change2")]
    #[test_case("m3_scx_high_5_bits" ; "m3_scx_high_5_bits")]
    #[test_case("m3_scx_low_3_bits" ; "m3_scx_low_3_bits")]
    #[test_case("m3_scy_change" ; "m3_scy_change")]
    #[test_case("m3_scy_change2" ; "m3_scy_change2")]
    #[test_case("m3_window_timing_wx_0" ; "m3_window_timing_wx_0")]
    #[test_case("m3_window_timing" ; "m3_window_timing")]
    #[test_case("m3_wx_4_change_sprites" ; "m3_wx_4_change_sprites")]
    #[test_case("m3_wx_4_change" ; "m3_wx_4_change")]
    #[test_case("m3_wx_5_change" ; "m3_wx_5_change")]
    #[test_case("m3_wx_6_change" ; "m3_wx_6_change")]
    #[test_case("win_without_bg" ; "win_without_bg")]
    fn ppu(rom_name: &str) {
        let rom = load_file(Path::new(TEST_DIR_PATH).join(rom_name.to_owned() + ".gb").as_path()).unwrap();

        let png_path = rom_name.to_owned() + "_dmg_blob.png";
        let img: DynamicImage = ImageReader::open(Path::new(TEST_DIR_PATH).join(png_path).as_path()).unwrap()
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

        // let output_png_path = rom_name.to_owned() + "_output.png";
        // save_rgb_as_png(gb.display.engine.screen.as_slice(), 160, 144, &*output_png_path).unwrap();

        assert_eq!(gb.display.engine.screen.len(), rgb_vec.len());

        let equals = gb.display.engine.screen.eq(&rgb_vec);
        assert!(equals, "output was not equal");
        // assert_eq!(gb.display.engine.screen, rgb_vec);
    }

    fn save_rgb_as_png(rgb_vec: &[u32], width: u32, height: u32, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Step 1 & 2: Unpack u32 values and flatten to Vec<u8>
        let mut raw_rgb: Vec<u8> = Vec::with_capacity(rgb_vec.len() * 3);
        for &packed in rgb_vec {
            let r = ((packed >> 16) & 0xFF) as u8;
            let g = ((packed >> 8) & 0xFF) as u8;
            let b = (packed & 0xFF) as u8;
            raw_rgb.extend_from_slice(&[r, g, b]);
        }

        // Step 3: Create the image buffer
        let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, raw_rgb)
            .expect("Dimensions don't match the length of the RGB vector");

        // Step 4: Save the buffer as PNG
        img.save(Path::new(filename))?;
        Ok(())
    }

}

