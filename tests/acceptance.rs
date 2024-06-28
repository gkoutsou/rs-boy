use rs_boy::gameboy::GameBoy;
use std::path::Path;

const ROMPATH: &str = "test/mooneye/acceptance";

macro_rules! test {
    ($fn_name:ident, $rom:expr) => {
        #[test]
        fn $fn_name() {
            let mut gb = GameBoy::new(Path::new(ROMPATH).join($rom).to_str().unwrap());

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
    };
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

test!(rapid_di_ei, "rapid_di_ei.gb");
test!(oam_dma_start, "oam_dma_start.gb");
test!(boot_regs_dmg_abc, "boot_regs-dmgABC.gb");
test!(reti_timing, "reti_timing.gb");
test!(call_timing, "call_timing.gb");
test!(reti_intr_timing, "reti_intr_timing.gb");
test!(boot_regs_mgb, "boot_regs-mgb.gb");
test!(ei_sequence, "ei_sequence.gb");
test!(jp_timing, "jp_timing.gb");
test!(ei_timing, "ei_timing.gb");
test!(oam_dma_timing, "oam_dma_timing.gb");
test!(call_cc_timing2, "call_cc_timing2.gb");
test!(boot_div2_s, "boot_div2-S.gb");
test!(halt_ime1_timing, "halt_ime1_timing.gb");
test!(halt_ime1_timing2_gs, "halt_ime1_timing2-GS.gb");
test!(boot_regs_sgb, "boot_regs-sgb.gb");
test!(jp_cc_timing, "jp_cc_timing.gb");
test!(call_timing2, "call_timing2.gb");
test!(ld_hl_sp_e_timing, "ld_hl_sp_e_timing.gb");
test!(push_timing, "push_timing.gb");
test!(boot_hwio_dmg0, "boot_hwio-dmg0.gb");
test!(rst_timing, "rst_timing.gb");
test!(boot_hwio_s, "boot_hwio-S.gb");
test!(boot_div_dmg_abc_mgb, "boot_div-dmgABCmgb.gb");
test!(div_timing, "div_timing.gb");
test!(ret_cc_timing, "ret_cc_timing.gb");
test!(boot_regs_dmg0, "boot_regs-dmg0.gb");
test!(boot_hwio_dmg_abc_mgb, "boot_hwio-dmgABCmgb.gb");
test!(pop_timing, "pop_timing.gb");
test!(ret_timing, "ret_timing.gb");
test!(oam_dma_restart, "oam_dma_restart.gb");
test!(add_sp_e_timing, "add_sp_e_timing.gb");
test!(halt_ime0_nointr_timing, "halt_ime0_nointr_timing.gb");
test!(call_cc_timing, "call_cc_timing.gb");
test!(halt_ime0_ei, "halt_ime0_ei.gb");
test!(intr_timing, "intr_timing.gb");
test!(if_ie_registers, "if_ie_registers.gb");
test!(di_timing_gs, "di_timing-GS.gb");
test!(boot_regs_sgb2, "boot_regs-sgb2.gb");
test!(boot_div_s, "boot_div-S.gb");
test!(boot_div_dmg0, "boot_div-dmg0.gb");

test!(bits_mem_oam, "bits/mem_oam.gb");
test!(bits_ref_f, "bits/reg_f.gb");
test!(bits_unused_hwio_gs, "bits/unused_hwio-GS.gb");

test!(instr_daa, "instr/daa.gb");

test!(interrupts_ie_push, "interrupts/ie_push.gb");

test!(oam_dma_basic, "oam_dma/basic.gb");
test!(oam_dma_reg_read, "oam_dma/reg_read.gb");
test!(oam_dma_sources_gs, "oam_dma/sources-GS.gb");

test!(ppu_vblank_stat_intr_gs, "ppu/vblank_stat_intr-GS.gb");
test!(
    ppu_intr_2_mode0_timing_sprites,
    "ppu/intr_2_mode0_timing_sprites.gb"
);
test!(ppu_stat_irq_blocking, "ppu/stat_irq_blocking.gb");
test!(ppu_intr_1_2_timing_gs, "ppu/intr_1_2_timing-GS.gb");
test!(ppu_intr_2_mode0_timing, "ppu/intr_2_mode0_timing.gb");
test!(ppu_lcdon_write_timing_gs, "ppu/lcdon_write_timing-GS.gb");
test!(
    ppu_hblank_ly_scx_timing_gs,
    "ppu/hblank_ly_scx_timing-GS.gb"
);
test!(ppu_intr_2_0_timing, "ppu/intr_2_0_timing.gb");
test!(ppu_stat_lyc_onoff, "ppu/stat_lyc_onoff.gb");
test!(ppu_intr_2_mode3_timing, "ppu/intr_2_mode3_timing.gb");
test!(ppu_lcdon_timing_gs, "ppu/lcdon_timing-GS.gb");
test!(ppu_intr_2_oam_ok_timing, "ppu/intr_2_oam_ok_timing.gb");

test!(
    serial_boot_sclk_align_dmg_abc_mgb,
    "serial/boot_sclk_align-dmgABCmgb.gb"
);

test!(timer_tima_reload, "timer/tima_reload.gb");
test!(timer_tma_write_reloading, "timer/tma_write_reloading.gb");
test!(timer_tim10, "timer/tim10.gb");
test!(timer_tim00, "timer/tim00.gb");
test!(timer_tim11, "timer/tim11.gb");
test!(timer_tim01, "timer/tim01.gb");
test!(timer_tima_write_reloading, "timer/tima_write_reloading.gb");
test!(timer_tim11_div_trigger, "timer/tim11_div_trigger.gb");
test!(timer_div_write, "timer/div_write.gb");
test!(timer_tim10_div_trigger, "timer/tim10_div_trigger.gb");
test!(timer_tim00_div_trigger, "timer/tim00_div_trigger.gb");
test!(timer_rapid_toggle, "timer/rapid_toggle.gb");
test!(timer_tim01_div_trigger, "timer/tim01_div_trigger.gb");
