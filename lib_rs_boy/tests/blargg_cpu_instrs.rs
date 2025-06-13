#[cfg(test)]
mod test {
    use lib_rs_boy::gameboy::GameBoy;
    use std::path::Path;
    use test_case::test_case;

    const ROMPATH: &str = "tests/blargg/cpu_instrs/individual";

    const SERIAL_DATA_LOCATION: usize = 0xff01;
    const SERIAL_TRANSFER_LOCATION: usize = 0xff02;

    #[test_case("01-special.gb" ; "1 special")]
    #[test_case("02-interrupts.gb" ; "2 interrupts")]
    #[test_case("03-op sp,hl.gb" ; "3 op sp,hl")]
    #[test_case("04-op r,imm.gb" ; "4 op r,imm")]
    #[test_case("05-op rp.gb" ; "5 op rp")]
    #[test_case("06-ld r,r.gb" ; "6 ld r,r")]
    #[test_case("07-jr,jp,call,ret,rst.gb" ; "7 jr,jp,call,ret,rst")]
    #[test_case("08-misc instrs.gb" ; "8 misc instrs")]
    #[test_case("09-op r,r.gb" ; "9 op r,r")]
    #[test_case("10-bit ops.gb" ; "10 bit ops")]
    #[test_case("11-op a,(hl).gb" ; "11 op a,(hl)")]
    fn blargg_cpu_instrs(rom: &str) {
        let mut gb = GameBoy::new(Path::new(ROMPATH).join(rom).to_str().unwrap());
        let mut ongoing_transfer = false;
        let mut output = String::new();
        loop {
            let status = gb.memory_read(SERIAL_TRANSFER_LOCATION);
            if (status & (1 << 7)) > 0 && !ongoing_transfer {
                ongoing_transfer = true;
                let ascii = gb.memory_read(SERIAL_DATA_LOCATION);
                output.push(ascii as char);
                // Hacky, but if a serial transfer is requested, read and directly flag it as done
                gb.memory_write(SERIAL_TRANSFER_LOCATION, 1);
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
