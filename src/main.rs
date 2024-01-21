use constants::{SCREEN_HEIGHT, SCREEN_WIDTH};
use gba::Gba;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};
use sdl2::{event::Event, render::Canvas};
use sdl2::keyboard::Keycode;
use std::time::Instant;
use std::{env, fs};

mod arm7;
mod gba;
mod memory;
mod video;
mod utils;
mod scheduler;
mod constants;

fn render(gba: &mut Gba, window: &mut Canvas<Window>, texture_creator: &TextureCreator<WindowContext>) {
    let memory_mut = gba.get_frame_buffer();
    let screen = Surface::from_data(
        memory_mut,
        SCREEN_WIDTH as u32,
        SCREEN_HEIGHT as u32,
        480,
        sdl2::pixels::PixelFormatEnum::BGR555,
    ).unwrap();
    window.copy(&texture_creator.create_texture_from_surface(screen).unwrap(), None, None).unwrap();
    window.present();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let path2 = &args[2];

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut window = video_subsystem
        .window("GBA", 240, 160)
        .build()
        .unwrap()
        .into_canvas()
        .build()
        .unwrap();

    let texture_creator = window.texture_creator();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut gba = Gba::new();

    match fs::read(path) {
        Ok(x) => gba.load_bios(x),
        _ => panic!(),
    }

    match fs::read(path2) {
        Ok(x) => gba.load_rom(x),
        _ => panic!(),
    }

    'running: loop {
        let start_time = Instant::now();
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
        gba.frame();
        render(&mut gba, &mut window, &texture_creator);
        println!("{:#?}", start_time.elapsed());
    }
}
