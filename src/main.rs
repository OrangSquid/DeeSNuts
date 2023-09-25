use gba::Gba;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::{env, fs};

pub mod alu;
pub mod arm7;
pub mod gba;
pub mod memory;
pub mod video;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let path2 = &args[2];

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let windows = video_subsystem
        .window("GBA", 240, 160)
        .build()
        .unwrap()
        .into_canvas()
        .build()
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut gba = Gba::new(windows);

    match fs::read(path) {
        Ok(x) => gba.load_bios(x),
        _ => panic!(),
    }

    match fs::read(path2) {
        Ok(x) => gba.load_rom(x),
        _ => panic!(),
    }

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
        gba.next();
    }
}
