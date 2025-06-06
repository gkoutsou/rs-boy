# Yet another GameBoy emulator

This is a pet project in an attempt to learn Rust by doing a real life project.

While the emulator is usable in some extent, it is not fully implementing all the hardware's features so most games will
not work as expected.

## Implemented features

- [x] All CPU instructions
- [x] Basic support for NoMBC/MBC1/MBC3 Cartridge types
- [x] Timer/VBlank/STAT Interrupts
- [x] Keyboard controls
- [x] Audio Channel 1/2

## Missing features

- [ ] Audio Channels 3/4
- [ ] GamePad support
- [ ] RTC Register for MBC3 cartridges.

## Test Suites

### Blargg's test ROMs

| Name             | Passed | Failing |
|------------------|--------|---------|
| cpu_instructions | 11     | 0       |
| dgm_sound        | 2      | 10      |
| instr_timing     | 0      | 1       |
| interrupt_time   | 0      | 1       |
| memory_timing    | 0      | 3       |
| memory_timing_2  | 0      | 3       |

### Mooneye Acceptance Tests

| Passed | Failing | Skipped |
|--------|---------|---------|
| 29     | 37      | 9       |

### Acid2:

- Failed `Object Priority Same X Coordinate`

## References:

- [Pandocs](https://gbdev.io/pandocs/)
- [Game Boy: Complete Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf)
- [GameBoy Opcode Summary](http://gameboy.mongenel.com/dmg/opcodes.html)
- [GameBoy CPU Manual](http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf)
- [Game Boy Sound Hardware](https://gbdev.gg8.se/wiki/articles/Gameboy_sound_hardware)
- [Blargg's test ROMs](https://gbdev.gg8.se/files/roms/blargg-gb-tests/)
- [Mooneye Test ROMs](https://github.com/Gekkio/mooneye-test-suite/)
- [Acid2 Test](https://github.com/mattcurrie/dmg-acid2)