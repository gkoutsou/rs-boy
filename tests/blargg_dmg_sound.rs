use log::info;
use rs_boy::gameboy::GameBoy;
use std::path::Path;

const ROMPATH: &str = "test/blargg/dmg_sound/rom_singles";

const STATUS_LOCATION: usize = 0xA000;
const TEST_RUNNING_VALUE: u8 = 0x80;

macro_rules! test {
    ($fn_name:ident, $rom:expr) => {
        #[test]
        fn $fn_name() {
            let mut gb = GameBoy::new(Path::new(ROMPATH).join($rom).to_str().unwrap());

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
    };
}

test!(registers, "01-registers.gb");
test!(len_ctr, "02-len ctr.gb");
test!(trigger, "03-trigger.gb");
test!(sweep, "04-sweep.gb");
test!(sweep_details, "05-sweep details.gb");
test!(overflow_on_trigger, "06-overflow on trigger.gb");
test!(len_sweep_period_sync, "07-len sweep period sync.gb");
test!(leb_ctr_during_power, "08-len ctr during power.gb");
test!(wave_read_while_on, "09-wave read while on.gb");
test!(wave_trigger_while_on, "10-wave trigger while on.gb");
test!(regs_after_power, "11-regs after power.gb");
test!(wave_write_while_on, "12-wave write while on.gb");
