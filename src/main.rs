use anyhow::Result;
use nes::NES;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::{event::Event, rect::Rect};
use std::{
    cell::RefCell,
    env,
    rc::Rc,
    thread,
    time::{Duration, Instant},
};

pub mod bus;
pub mod cpu;
pub mod nes;
pub mod ram;

/**
 * Master clock is 236.25/11, or 945/44 Mhz
 * PPU is 1/4 of master clock, so 945/176 Mhz
 * If we target 60 frames/second then thats
 * 945000000 / 10560, or
 * 984375 / 11, cycles per frame
 * That's 89488 7/11 cycles per frame
 * let's call it 89489
 */
const CYCLES_PER_FRAME: u32 = 89489;
/**
 * 30 frames a sec is so many nanos per frame
 */
const NANOS_PER_FRAME: u64 = 33333333;

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let cartridge_name = if args.len() > 1 {
        &args[1]
    } else {
        "resources/test/nestest.nes"
    };

    let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;
    let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;

    let window = video_subsystem
        .window("NES RS", 512, 480)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())
        .map_err(anyhow::Error::msg)?;

    let mut canvas = window
        .into_canvas()
        .build()
        .map_err(|e| e.to_string())
        .map_err(anyhow::Error::msg)?;

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let canvas = Rc::new(RefCell::new(canvas));

    let mut event_pump = sdl_context.event_pump().map_err(anyhow::Error::msg)?;

    let render_clone = canvas.clone();
    let renderer = Box::new(move |x, y, r, g, b| {
        let mut c = render_clone.as_ref().borrow_mut();
        c.set_draw_color(Color::RGB(r, g, b));
        for i in x * 2..=x * 2 + 1 {
            for j in y * 2..=y * 2 + 1 {
                c.draw_point((i as i32, j as i32)).unwrap()
            }
        }
    });

    let mut nes = NES::new(renderer);
    nes.load_cartridge(cartridge_name.to_string())?;

    nes.reset();

    let frame_time = Duration::from_nanos(NANOS_PER_FRAME);
    let start = Instant::now();
    let mut next_frame = start + frame_time;
    let mut frame = 0.0;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => (),
            }
        }

        for _ in 0..CYCLES_PER_FRAME {
            nes.clock();
        }
        let now = Instant::now();

        canvas.as_ref().borrow_mut().present();

        frame += 1.0;

        if now < next_frame {
            thread::sleep(next_frame - now);
        }
        println!("Frames/sec: {}", frame / (now - start).as_secs_f32());
        next_frame += frame_time;
    }

    Ok(())
}
