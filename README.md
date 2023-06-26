An in progress emulator for the venerable Nintendo Entertainment System. Within the limitations listed in the todo list below, it has a pretty complete core and can run a good variety of games. One key thing missing is any kind of UI. Specify the cartridge to load on the command line. The following control scheme is (currently) hard coded

| Control | Key    |
| ------- | ------ |
| Up      | W      |
| Left    | A      |
| Down    | S      |
| Right   | D      |
| A       | K      |
| B       | J      |
| Start   | Enter  |
| Select  | \      |


On my system, the only way to get acceptable performance is cargo run/build --release. Debug mode just won't cut it - PPU cycles take 10x as long under debug as they do under release.

## TODO List

- [X] 6502
    - [X] Official opcodes
    - [X] Unofficial opcodes
- [ ]  Cartridge
    - [X] Cartridge Core
    - [X] INes 1.0
    - [ ] INes 2.0
    - [X] Persistent SRAM
- [X] Input
    - [X] General controller support infra
    - [X] Joypad 1
    - [ ] Joypad 2
    - [ ] Other controllers TBD
- [X] PPU
    - [X] Registers
    - [X] Scrolling
    - [X] Background rendering
    - [X] Sprite DMA
    - [X] Sprite rendering
- [X] APU
    - [X] Pulse channels
    - [X] Mixer
    - [X] Play sounds
    - [X] Triangle channel
    - [X] Noise channel  
    - [X] DMC channel
- [ ] Detailed Timing
    - [ ] Cycle correct 6502
    - [ ] Accurate NMI timing
    - [ ] DMA should only pause CPU on 'read' cycle
    - [ ] DMC DMA and OAM DMA interact in weird ways
- [ ] Top 10 Mappers
    - [X] Mapper 1
    - [ ] Mapper 4 
    - [X] Mapper 2 
    - [X] Mapper 0 
    - [X] Mapper 3 
    - [X] Mapper 7 
    - [ ] Mapper 206
    - [X] Mapper 11 
    - [ ] Mapper 5 
    - [ ] Mapper 19
- [ ] Related Mappers
    - [X] Mapper 94
    - [X] Mapper 105
    - [X] Mapper 155
    - [X] Mapper 180
    - [X] Mapper 185
    - [ ] More TBD
- [ ] Graphical UI
    - [ ] Cart database
    - [ ] Pause/resume
    - [ ] Input config