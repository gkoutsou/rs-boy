# RS Boy

This is a pet project in an attempt to learn Rust by doing a real life project.

While the emulator is usable in some extent, it is not fully implementing all the hardware's features so most games will
not work as expected.

It can function either as a standalone binary, or it can be loaded as a retroarch/libretro core.

## Implemented features

- [x] All CPU instructions
- [x] Basic support for NoMBC/MBC1/MBC3/MBC5 Cartridge types
- [x] Timer/VBlank/STAT Interrupts
- [x] Keyboard controls
- [x] GamePad support (only via libretro)
- [x] Rumble support
- [x] Audio Channel 1,2,3,4

## Missing features

- [ ] RTC Register for MBC3 cartridges.

## Test Suites

### [Blargg's test ROMs](https://gbdev.gg8.se/files/roms/blargg-gb-tests/)

| Name             | Passed | Failing |
|------------------|--------|---------|
| cpu_instructions | 11     | 0       |
| dgm_sound        | 6      | 6       |
| instr_timing     | 1      | 0       |
| interrupt_time   | 0      | 1       |
| memory_timing    | 0      | 3       |
| memory_timing_2  | 0      | 3       |

### [Mooneye Acceptance Test ROMs](https://github.com/Gekkio/mooneye-test-suite/)

| Passed | Failing | Skipped |
|--------|---------|---------|
| 33     | 33      | 9       |

### [Mealybug Tearoom PPU Test ROMs](https://github.com/mattcurrie/mealybug-tearoom-tests/)

| Passed | Failing | Skipped |
|--------|---------|---------|
| 0      | 32      | 0       |

### [Acid2 Test:](https://github.com/mattcurrie/dmg-acid2)

- Passed

### [Scribbltests:](https://github.com/Hacktix/scribbltests/tree/master)

| Name      | Passed |
|-----------|--------|
| fairylake | ❌      |
| lycscx    | ✅      |
| lycscy    | ✅      |
| palettely | ✅      |
| scxly     | ✅      |
| statcount | ❌      |
| winpos    | ✅      |

## References:

- [Pandocs](https://gbdev.io/pandocs/)
- [Game Boy: Complete Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf)
- [Game Boy Opcode Summary](http://gameboy.mongenel.com/dmg/opcodes.html)
- [Game Boy CPU Manual](http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf)
- [Game Boy Sound Hardware](https://gbdev.gg8.se/wiki/articles/Gameboy_sound_hardware)
- [Nightshade's GB Sound Emulation](https://nightshade256.github.io/2021/03/27/gb-sound-emulation.html)
