use anyhow::Result;
use minifb::{Key, Scale, ScaleMode, Window, WindowOptions};
use nes::{controllers::JoyPad, NES};
use std::collections::HashSet;
use std::{
    cell::RefCell,
    env,
    rc::Rc,
    thread,
    time::{Duration, Instant},
};

use crate::nes::controllers::JoyPadButton;

pub mod bus;
pub mod cpu;
pub mod nes;
pub mod ram;

/**
 * 60 frames a sec is so many nanos per frame
 */
const NANOS_PER_FRAME: u64 = 16666666;

const NES_WIDTH: usize = 256;
const NES_HEIGHT: usize = 240;

fn main() -> Result<()> {
    let buffer = Rc::new(RefCell::new(vec![0; NES_WIDTH * NES_HEIGHT]));

    let args = env::args().collect::<Vec<String>>();
    let cartridge_name = if args.len() > 1 {
        &args[1]
    } else {
        "resources/test/nestest.nes"
    };

    let opts = WindowOptions {
        borderless: false,
        scale: Scale::FitScreen,
        title: true,
        resize: true,
        scale_mode: ScaleMode::AspectRatioStretch,
        topmost: false,
        transparency: false,
        none: false,
    };

    let mut window = Window::new("NES RS", NES_WIDTH * 3, NES_HEIGHT * 3, opts)?;
    let render_clone = buffer.clone();

    let renderer = Box::new(move |x, y, r, g, b| {
        let mut buffer = render_clone.as_ref().borrow_mut();
        let color = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
        let (x, y) = (x as usize, y as usize);
        buffer[y * NES_WIDTH + x] = color;
    });

    let mut nes = NES::new(renderer);
    nes.load_cartridge(cartridge_name.to_string())?;
    let joypad1 = Rc::new(RefCell::new(JoyPad::new()));
    nes.plugin_controller1(joypad1.clone());

    nes.reset();

    let frame_time = Duration::from_nanos(NANOS_PER_FRAME);
    let start = Instant::now();
    let mut next_frame = start + frame_time;
    let mut frame = 0.0;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let keys: HashSet<Key> = HashSet::from_iter(window.get_keys().into_iter());

        let input: u8 = check_keycode(&keys, Key::W, JoyPadButton::Up)
            | check_keycode(&keys, Key::A, JoyPadButton::Left)
            | check_keycode(&keys, Key::S, JoyPadButton::Down)
            | check_keycode(&keys, Key::D, JoyPadButton::Right)
            | check_keycode(&keys, Key::J, JoyPadButton::A)
            | check_keycode(&keys, Key::K, JoyPadButton::B)
            | check_keycode(&keys, Key::Enter, JoyPadButton::Start)
            | check_keycode(&keys, Key::Backslash, JoyPadButton::Select);

        joypad1.as_ref().borrow_mut().set_buttons(input);

        while !nes.clock() {}
        let now = Instant::now();

        window.update_with_buffer(&buffer.as_ref().borrow(), NES_WIDTH, NES_HEIGHT)?;

        if now < next_frame {
            thread::sleep(next_frame - now);
        }
        next_frame += frame_time;

        const DISPLAY_FRAME_RATE: bool = true;

        if DISPLAY_FRAME_RATE {
            frame += 1.0;

            println!("Frames/sec: {}", frame / (now - start).as_secs_f32());
        }
    }

    Ok(())
}

fn check_keycode(keys: &HashSet<Key>, key: Key, button: JoyPadButton) -> u8 {
    if keys.contains(&key) {
        0 | button
    } else {
        0
    }
}
