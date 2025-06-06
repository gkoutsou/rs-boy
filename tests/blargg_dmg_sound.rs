#[cfg(test)]
mod test {
    use rs_boy::gameboy::GameBoy;
    use std::path::Path;
    use test_case::test_case;

    const ROM_PATH: &str = "tests/blargg/dmg_sound/rom_singles";

    const STATUS_LOCATION: usize = 0xA000;
    const TEST_RUNNING_VALUE: u8 = 0x80;

    #[test_case("01-registers.gb" ; "1 registers")]
    #[test_case("02-len ctr.gb" ; "2 len_ctr")]
    #[test_case("03-trigger.gb" ; "3 trigger")]
    #[test_case("04-sweep.gb" ; "4 sweep")]
    #[test_case("05-sweep details.gb" ; "5 sweep_details")]
    #[test_case("06-overflow on trigger.gb" ; "6 overflow_on_trigger")]
    #[test_case("07-len sweep period sync.gb" ; "7 len_sweep_period_sync")]
    #[test_case("08-len ctr during power.gb" ; "8 leb_ctr_during_power")]
    #[test_case("09-wave read while on.gb" ; "9 wave_read_while_on")]
    #[test_case("10-wave trigger while on.gb" ; "10 wave_trigger_while_on")]
    #[test_case("11-regs after power.gb" ; "11 regs_after_power")]
    #[test_case("12-wave write while on.gb" ; "12 wave_write_while_on")]
    fn acceptance(rom: &str) {
        let mut gb = GameBoy::new(Path::new(ROM_PATH).join(rom).to_str().unwrap());

        let mut found = false;
        loop {
            let status = gb.memory_read(STATUS_LOCATION);

            if !found && status == TEST_RUNNING_VALUE {
                found = true;
            } else if found && status != TEST_RUNNING_VALUE {
                break;
            }

            gb.step();
        }

        assert_eq!(gb.memory_read(0xA001), 0xDE);
        assert_eq!(gb.memory_read(0xA002), 0xB0);
        assert_eq!(gb.memory_read(0xA003), 0x61);

        // Not correct
        // TODO do I need to read the text after each test?
        assert_eq!(gb.memory_read(STATUS_LOCATION), 0);
    }
}
