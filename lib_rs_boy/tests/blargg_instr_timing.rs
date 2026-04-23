#[cfg(test)]
mod test {
    use lib_rs_boy::gameboy::GameBoy;
    use lib_rs_boy::io::files::load_file;
    use std::path::Path;
    use test_case::test_case;

    const ROMPATH: &str = "tests/game-boy-test-roms-v7.0/blargg/instr_timing";

    const SERIAL_DATA_LOCATION: usize = 0xff01;
    const SERIAL_TRANSFER_LOCATION: usize = 0xff02;

    #[test_case("instr_timing.gb" ; "instr_timing.gb")]
    fn blargg_instr_timing(rom: &str) {
        let rom = load_file(Path::new(ROMPATH).join(rom).as_path()).unwrap();
        let mut gb = GameBoy::new();
        gb.load_rom(rom);
        let mut ongoing_transfer = false;
        let mut output = String::new();
        loop {
            let status = gb.memory_read_no_tick(SERIAL_TRANSFER_LOCATION);
            if (status & (1 << 7)) > 0 && !ongoing_transfer {
                ongoing_transfer = true;
                let ascii = gb.memory_read_no_tick(SERIAL_DATA_LOCATION);
                output.push(ascii as char);
                // Hacky, but if a serial transfer is requested, read and directly flag it as done
                gb.memory_write_no_tick(SERIAL_TRANSFER_LOCATION, 1);
            } else if status & 1 << 7 == 0 && ongoing_transfer {
                ongoing_transfer = false;
            }

            if output.contains("Passed") {
                break;
            } else if output.contains("Failed") {
                break;
            }

            gb.step();
        }
        assert_eq!(output.contains("Passed"), true);
    }
}
