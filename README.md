An in progress emulator for the venerable Nintendo Entertainment System. Within the limitations listed in the todo list below, it has a pretty complete core and can run a good variety of games. One key thing missing is any kind of UI. Specify the cartridge to load on the command line. The following control scheme is (currently) hard coded

| Control | Key   |
| ------- | ----- |
| Up      | W     |
| Left    | A     |
| Down    | S     |
| Right   | D     |
| A       | K     |
| B       | J     |
| Start   | Enter |
| Select  | \     |


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
- [ ] Top 10 Mappers [^1]
    - [X] Mapper 1
    - [X] Mapper 4
    - [X] Mapper 2 
    - [X] Mapper 0 
    - [X] Mapper 3 
    - [X] Mapper 7 
    - [X] Mapper 206
    - [X] Mapper 11 
    - [ ] Mapper 5 
    - [ ] Mapper 19
- [ ] Related Mappers
    - [X] Mapper 76
    - [X] Mapper 88
    - [X] Mapper 94
    - [X] Mapper 95
    - [X] Mapper 105
    - [X] Mapper 118
    - [X] Mapper 119
    - [X] Mapper 154
    - [X] Mapper 155
    - [X] Mapper 180
    - [X] Mapper 185
    - [ ] More TBD
- [ ] Graphical UI
    - [ ] Cart database
    - [ ] Pause/resume
    - [ ] Input config

[^1]: Mappers are software "shims" used to create compatibility with different cartridges.The original NES was quite limited in the capacity and capability built into the machine. However, as time went by and chips became cheaper, cartridges added their own capabilities and capacities by adding more memory storage, interrupt counters, and even sound channels. The bits of software needed to emulate different cartridge capabilities are called "mappers" because much of what they do is "map" a limited range memory addresses to large memory stores via address banking. Those mappers can't be captured in cartidge dumps, so emulator authors have to build them. The bad news is that there are hundreds of mappers. The good news is that the top 10 will cover ~90% of all cartridges.