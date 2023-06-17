use anyhow::Result;
use blip_buf::BlipBuf;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, Sample, SizedSample, StreamConfig,
};
use crossbeam_channel::bounded;
use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use nes::{controllers::JoyPad, NES};
use std::collections::HashSet;
use std::{cell::RefCell, env, rc::Rc, time::Instant};

use crate::nes::controllers::JoyPadButton;

pub mod bus;
pub mod cpu;
pub mod nes;
pub mod ram;

const NES_WIDTH: usize = 256;
const NES_HEIGHT: usize = 240;
const SCREEN_SCALE: usize = 3;
const SCREEN_HEIGHT: usize = 240 * SCREEN_SCALE;
// CRT TV aspect ratio of 4/3
const SCREEN_WIDTH: usize = SCREEN_HEIGHT * 4 / 3;
const PPU_CLOCK_SPEED: usize = 5369318;
// this buffer can be large. it's the working space
// for blip_buff to create downsamples during a frame
const BLIP_BUFF_SIZE: usize = 30000;
const VOLUME: f32 = 0.5;

fn main() -> Result<()> {
    let audio_host = cpal::default_host();
    let audio_device = audio_host
        .default_output_device()
        .expect("Could not find audio output device");
    let audio_config = audio_device.default_output_config()?;
    let audio_format = audio_config.sample_format();

    println!("{audio_format}");
    let run = match audio_format {
        cpal::SampleFormat::I8 => run::<i8>,
        cpal::SampleFormat::I16 => run::<i16>,
        cpal::SampleFormat::I32 => run::<i32>,
        cpal::SampleFormat::I64 => run::<i64>,
        cpal::SampleFormat::U8 => run::<u8>,
        cpal::SampleFormat::U16 => run::<u16>,
        cpal::SampleFormat::U32 => run::<u32>,
        cpal::SampleFormat::U64 => run::<u64>,
        cpal::SampleFormat::F32 => run::<f32>,
        cpal::SampleFormat::F64 => run::<f64>,
        _ => panic!("Unsupported sample format '{audio_format}'"),
    };

    let stream_config: StreamConfig = audio_config.into();
    run(&audio_device, &stream_config)
}

fn run<T>(audio_device: &Device, stream_config: &StreamConfig) -> Result<()>
where
    T: FromSample<i16> + SizedSample,
{
    let mut muted = false;
    // be sure  there's enough space in the shared queue for 2 frame's worth of samples
    let buff_size = (stream_config.sample_rate.0 / 60 * 2 + 1) as usize;
    let (sender, receiver) = bounded::<i16>(buff_size);
    let mut next_value = move || receiver.recv().unwrap_or(0);

    let err_callback = |err| eprintln!("an error occurred on stream: {}", err);
    let channels = stream_config.channels as usize;
    let data_callback = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
        write_data(data, channels, &mut next_value)
    };

    let stream =
        audio_device.build_output_stream(stream_config, data_callback, err_callback, None)?;
    stream.play()?;

    let mut blip = BlipBuf::new(buff_size as u32);
    let mut blip_buffer = [0; BLIP_BUFF_SIZE];
    blip.set_rates(PPU_CLOCK_SPEED as f64, stream_config.sample_rate.0 as f64);

    let mut screen_buffer = vec![0; NES_WIDTH * NES_HEIGHT];

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
        scale_mode: ScaleMode::Stretch,
        topmost: false,
        transparency: false,
        none: false,
    };

    let mut window = Window::new("NES RS", SCREEN_WIDTH, SCREEN_HEIGHT, opts)?;

    let mut nes = NES::new();
    nes.load_cartridge(cartridge_name.to_string())?;
    let joypad1 = Rc::new(RefCell::new(JoyPad::new()));
    nes.plugin_controller1(joypad1.clone());

    nes.reset();

    let start = Instant::now();
    let mut frame = 0.0;
    let mut last_sample = 0;
    let mut clocks = 0;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (frame_complete, pixel_info, sample_opt) = nes.clock();

        if let Some(p) = pixel_info {
            let color = (p.r as u32) << 16 | (p.g as u32) << 8 | (p.b as u32);
            let (x, y) = (p.x as usize, p.y as usize);
            screen_buffer[y * NES_WIDTH + x] = color;
        }

        if let Some(sample_float) = sample_opt {
            let sample = if muted {
                0
            } else {
                // we get samples from 0.0 to 1.0, convert to ranging from -VOLUME to VOLUME with a clamp
                // to make doubly sure
                let sample_normed = ((sample_float - 0.5) * VOLUME).clamp(-VOLUME, VOLUME);
                // now convert that to -i16::MAX to +i16::MAX
                (sample_normed * (i16::MAX as f32)) as i32
            };

            let delta = sample - last_sample;
            last_sample = sample;
            blip.add_delta(clocks, delta);
        }
        clocks += 1;
        if frame_complete {
            window
                .update_with_buffer(&screen_buffer, NES_WIDTH, NES_HEIGHT)
                .unwrap();

            const DISPLAY_FRAME_RATE: bool = false;

            if DISPLAY_FRAME_RATE {
                frame += 1.0;
                let now = Instant::now();
                println!("Frames/sec: {}", frame / (now - start).as_secs_f32());
            }

            blip.end_frame(clocks);
            clocks = 0;
            while blip.samples_avail() != 0 {
                let samples_avail = blip.samples_avail().min(blip_buffer.len() as u32);
                blip.read_samples(&mut blip_buffer, false);
                for i in 0..samples_avail {
                    sender.send(blip_buffer[i as usize])?;
                }
            }

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

            if window.is_key_pressed(Key::M, KeyRepeat::No) {
                muted = !muted;
            }

            if window.is_key_pressed(Key::R, KeyRepeat::No) {
                nes.reset();
            }
        }
    }

    // ensures that the audio thread is killed
    drop(sender);
    Ok(())
}

fn check_keycode(keys: &HashSet<Key>, key: Key, button: JoyPadButton) -> u8 {
    if keys.contains(&key) {
        0 | button
    } else {
        0
    }
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> i16)
where
    T: Sample + FromSample<i16>,
{
    for frame in output.chunks_mut(channels) {
        let value: T = T::from_sample(next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
