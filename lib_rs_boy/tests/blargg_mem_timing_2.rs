#[cfg(test)]
mod test {
    use lib_rs_boy::gameboy::GameBoy;
    use lib_rs_boy::io::files::load_file;
    use std::path::Path;
    use test_case::test_case;

    const ROMPATH: &str = "tests/game-boy-test-roms-v7.0/blargg/mem_timing-2/rom_singles";

    const STATUS_LOCATION: usize = 0xA000;
    const TEST_RUNNING_VALUE: u8 = 0x80;

    #[test_case("01-read_timing.gb" ; "1 read timing")]
    #[test_case("02-write_timing.gb" ; "2 write timing")]
    #[test_case("03-modify_timing.gb" ; "3 modify timing")]
    fn blargg_mem_timing_2(rom: &str) {
        let rom = load_file(Path::new(ROMPATH).join(rom).as_path()).unwrap();
        let mut gb = GameBoy::new();
        gb.load_rom(rom);

        let mut found = false;
        loop {
            let status = gb.memory_read_no_tick(STATUS_LOCATION);

            if !found && status == TEST_RUNNING_VALUE {
                found = true;
            } else if found && status != TEST_RUNNING_VALUE {
                break;
            }

            gb.step();
        }

        assert_eq!(gb.memory_read_no_tick(0xA001), 0xDE);
        assert_eq!(gb.memory_read_no_tick(0xA002), 0xB0);
        assert_eq!(gb.memory_read_no_tick(0xA003), 0x61);

        assert_eq!(gb.memory_read_no_tick(STATUS_LOCATION), 0);
    }
}
