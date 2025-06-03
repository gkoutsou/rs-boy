use rs_boy::gameboy::GameBoy;
use std::path::Path;

#[cfg(test)]
mod test {
    use rs_boy::gameboy::GameBoy;
    use std::path::Path;
    use test_case::test_case;

    const ROMPATH: &str = "tests/mooneye/acceptance";
    #[test_case("rapid_di_ei.gb" ; "rapid_di_ei")]
    #[test_case("oam_dma_start.gb" ; "oam_dma_start")]
    #[test_case("boot_regs-dmgABC.gb" ; "boot_regs_dmg_abc")]
    #[test_case("reti_timing.gb" ; "reti_timing")]
    #[test_case("call_timing.gb" ; "call_timing")]
    #[test_case("reti_intr_timing.gb" ; "reti_intr_timing")]
    #[test_case("boot_regs-mgb.gb" ; "boot_regs_mgb")]
    #[test_case("ei_sequence.gb" ; "ei_sequence")]
    #[test_case("jp_timing.gb" ; "jp_timing")]
    #[test_case("ei_timing.gb" ; "ei_timing")]
    #[test_case("oam_dma_timing.gb" ; "oam_dma_timing")]
    #[test_case("call_cc_timing2.gb" ; "call_cc_timing2")]
    #[test_case("boot_div2-S.gb" ; "boot_div2_s")]
    #[test_case("halt_ime1_timing.gb" ; "halt_ime1_timing")]
    #[test_case("halt_ime1_timing2-GS.gb" ; "halt_ime1_timing2_gs")]
    #[test_case("boot_regs-sgb.gb" ; "boot_regs_sgb")]
    #[test_case("jp_cc_timing.gb" ; "jp_cc_timing")]
    #[test_case("call_timing2.gb" ; "call_timing2")]
    #[test_case("ld_hl_sp_e_timing.gb" ; "ld_hl_sp_e_timing")]
    #[test_case("push_timing.gb" ; "push_timing")]
    #[test_case("boot_hwio-dmg0.gb" ; "boot_hwio_dmg0")]
    #[test_case("rst_timing.gb" ; "rst_timing")]
    #[test_case("boot_hwio-S.gb" ; "boot_hwio_s")]
    #[test_case("boot_div-dmgABCmgb.gb" ; "boot_div_dmg_abc_mgb")]
    #[test_case("div_timing.gb" ; "div_timing")]
    #[test_case("ret_cc_timing.gb" ; "ret_cc_timing")]
    #[test_case("boot_regs-dmg0.gb" ; "boot_regs_dmg0")]
    #[test_case("boot_hwio-dmgABCmgb.gb" ; "boot_hwio_dmg_abc_mgb")]
    #[test_case("pop_timing.gb" ; "pop_timing")]
    #[test_case("ret_timing.gb" ; "ret_timing")]
    #[test_case("oam_dma_restart.gb" ; "oam_dma_restart")]
    #[test_case("add_sp_e_timing.gb" ; "add_sp_e_timing")]
    #[test_case("halt_ime0_nointr_timing.gb" ; "halt_ime0_nointr_timing")]
    #[test_case("call_cc_timing.gb" ; "call_cc_timing")]
    #[test_case("halt_ime0_ei.gb" ; "halt_ime0_ei")]
    #[test_case("intr_timing.gb" ; "intr_timing")]
    #[test_case("if_ie_registers.gb" ; "if_ie_registers")]
    #[test_case("di_timing-GS.gb" ; "di_timing_gs")]
    #[test_case("boot_regs-sgb2.gb" ; "boot_regs_sgb2")]
    #[test_case("boot_div-S.gb" ; "boot_div_s")]
    #[test_case("boot_div-dmg0.gb" ; "boot_div_dmg0")]
    #[test_case("bits/mem_oam.gb" ; "bits_mem_oam")]
    #[test_case("bits/reg_f.gb" ; "bits_ref_f")]
    #[test_case("bits/unused_hwio-GS.gb" ; "bits_unused_hwio_gs")]
    #[test_case("instr/daa.gb" ; "instr_daa")]
    #[test_case("interrupts/ie_push.gb" ; "interrupts_ie_push")]
    #[test_case("oam_dma/basic.gb" ; "oam_dma_basic")]
    #[test_case("oam_dma/reg_read.gb" ; "oam_dma_reg_read")]
    #[test_case("oam_dma/sources-GS.gb" ; "oam_dma_sources_gs")]
    #[test_case("ppu/vblank_stat_intr-GS.gb" ; "ppu_vblank_stat_intr_gs")]
    #[test_case("ppu/intr_2_mode0_timing_sprites.gb" ; "ppu_intr_2_mode0_timing_sprites")]
    #[test_case("ppu/stat_irq_blocking.gb" ; "ppu_stat_irq_blocking")]
    #[test_case("ppu/intr_1_2_timing-GS.gb" ; "ppu_intr_1_2_timing_gs")]
    #[test_case("ppu/intr_2_mode0_timing.gb" ; "ppu_intr_2_mode0_timing")]
    #[test_case("ppu/lcdon_write_timing-GS.gb" ; "ppu_lcdon_write_timing_gs")]
    #[test_case("ppu/hblank_ly_scx_timing-GS.gb" ; "ppu_hblank_ly_scx_timing_gs")]
    #[test_case("ppu/intr_2_0_timing.gb" ; "ppu_intr_2_0_timing")]
    #[test_case("ppu/stat_lyc_onoff.gb" ; "ppu_stat_lyc_onoff")]
    #[test_case("ppu/intr_2_mode3_timing.gb" ; "ppu_intr_2_mode3_timing")]
    #[test_case("ppu/lcdon_timing-GS.gb" ; "ppu_lcdon_timing_gs")]
    #[test_case("ppu/intr_2_oam_ok_timing.gb" ; "ppu_intr_2_oam_ok_timing")]
    #[test_case("serial/boot_sclk_align-dmgABCmgb.gb" ; "serial_boot_sclk_align_dmg_abc_mgb")]
    #[test_case("timer/tima_reload.gb" ; "timer_tima_reload")]
    #[test_case("timer/tma_write_reloading.gb" ; "timer_tma_write_reloading")]
    #[test_case("timer/tim10.gb" ; "timer_tim10")]
    #[test_case("timer/tim00.gb" ; "timer_tim00")]
    #[test_case("timer/tim11.gb" ; "timer_tim11")]
    #[test_case("timer/tim01.gb" ; "timer_tim01")]
    #[test_case("timer/tima_write_reloading.gb" ; "timer_tima_write_reloading")]
    #[test_case("timer/tim11_div_trigger.gb" ; "timer_tim11_div_trigger")]
    #[test_case("timer/div_write.gb" ; "timer_div_write")]
    #[test_case("timer/tim10_div_trigger.gb" ; "timer_tim10_div_trigger")]
    #[test_case("timer/tim00_div_trigger.gb" ; "timer_tim00_div_trigger")]
    #[test_case("timer/rapid_toggle.gb" ; "timer_rapid_toggle")]
    #[test_case("timer/tim01_div_trigger.gb" ; "timer_tim01_div_trigger")]
    fn acceptance(rom: &str) {
        let mut gb = GameBoy::new(Path::new(ROMPATH).join(rom).to_str().unwrap());

        let mut output: Vec<u8> = Vec::new();

        let mut found = false;
        loop {
            if gb.memory_read(gb.registers.pc as usize) == 0x40 {
                // LD B,B
                if found {
                    break;
                } else {
                    found = true
                }
            }

            let (v, ok) = is_serial_write(&gb);
            if ok {
                output.push(v);
            }
            gb.step();
        }

        assert_eq!(gb.registers.b, 3);
        assert_eq!(gb.registers.c, 5);
        assert_eq!(gb.registers.d, 8);
        assert_eq!(gb.registers.e, 13);
        assert_eq!(gb.registers.h, 21);
        assert_eq!(gb.registers.l, 34);

        assert_eq!(output, vec![3, 5, 8, 13, 21, 34])
    }

    // TODO checking serial port.. is it needed?
    fn is_serial_write(gb: &GameBoy) -> (u8, bool) {
        let op = gb.memory_read(gb.registers.pc as usize);
        if op == 0xe0 && gb.memory_read(gb.registers.pc as usize + 1) == 1 {
            return (gb.registers.a, true);
        }

        // if op == 0xea {
        //     let location1 = gb.memory_read(gb.registers.pc as usize + 1) as u16;
        //     let location2 = gb.memory_read(gb.registers.pc as usize + 2) as u16;
        //     println!("Location: {:#x}", location2 << 8 | location1);
        // }
        // if op == 0xe2 {
        //     println!("Location ffxx: {:#x}", gb.registers.c);
        // }
        // if op == 0x02 {
        //     println!("Location bc: {:#x}", gb.registers.get_bc());
        // }
        // if op == 0x12 {
        //     println!("Location de: {:#x}", gb.registers.get_de());
        // }
        (0, false)
    }
}
