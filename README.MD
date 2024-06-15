# Yet another GameBoy emulator

This is a pet project in an attempt to learn Rust by doing a real life project.

While the emulator is usable in some level, it is not fully implementing all the hardware's features so most games will not work as expected.

 ## Implemented features
 - [x] All CPU instructions
 - [x] Basic support for NoMBC/MBC1/MBC3 Cartridge types
 - [x] Timer/VBlank/STAT Interrupts
 - [x] Keyboard controls

## Missing features
- [ ] Audio
- [ ] GamePad support
- [ ] RTC Register for MBC3 cartridges.

## Test Suites
### Blargg's test ROMs
- [x] CPU Instructions
- [ ] Instruction Timing
- [ ] Interrupt Timing
- [ ] Memory Timing

### Acid2:
- Failed `Object Priority Same X Coordinate`


## References:
- [Pandocs](https://gbdev.io/pandocs/)
- [Game Boy: Complete Technical Reference](https://gekkio.fi/files/gb-docs/gbctr.pdf)
- [GameBoy Opcode Summary](http://gameboy.mongenel.com/dmg/opcodes.html)
- [GameBoy CPU Manual](http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf)
- [Blargg's test ROMs](https://gbdev.gg8.se/files/roms/blargg-gb-tests/)
- [Acid2 Test](https://github.com/mattcurrie/dmg-acid2)