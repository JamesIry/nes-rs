Ultimataly targeted to being an NES emulator. Now very much a work in progress.

I've chosen Rust SDL2 as a base library which means an external dependency. [Check here for installation](https://github.com/Rust-SDL2/rust-sdl2#sdl20-development-libraries)

- [X] 6502
    - [X] Official opcodes
    - [X] Unofficial opcodes
- [X] Cartridge Basics
    - [X] Core
    - [X] Mapper0/NROM
- [ ] PPU
    - [X] Registers
    - [X] Scrolling
    - [ ] Background rendering (in progress)
    - [ ] Sprite rendering
- [ ] ALU
    - [X] Sprite DMA
    - [ ] Sound DMA
    - [ ] Sound
- [ ] Input
    - [ ] Joypad
    - [ ] Other input devices TBD