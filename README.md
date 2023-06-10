Ultimataly targeted to being an NES emulator. Now very much a work in progress. Mapper0/NROM games like Super Mario Bros should be playable but won't have sound. Other mapper types are still TBD.

On my system, the only way to get acceptable performance is cargo run/build --release. Debug mode just won't cut it - PPU cycles take 10x as long under debug as they do under release.

- [X] 6502
    - [X] Official opcodes
    - [X] Unofficial opcodes
- [X] Cartridge Basics
    - [X] Core
    - [X] Mapper0/NROM
- [X] Input
    - [X] General controller support infra
    - [X] Joypad
- [X] PPU
    - [X] Registers
    - [X] Scrolling
    - [X] Background rendering
    - [X] Sprite rendering
- [ ] APU
    - [X] Sprite DMA
    - [ ] Sound DMA (in progress)
    - [ ] Sound
- [ ] Detailed Timing
    - [ ] Cycle correct 6502
    - [ ] Fix NMI timing
    - [ ] DMA should only pause CPU on 'read' cycle
- [ ] Other Mappers
    - [ ] Mapper1/MMC
    - [ ] Other MMC deriviatives
    - [ ] More TBD