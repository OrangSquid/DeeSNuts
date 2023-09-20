use std::{cell::RefCell, rc::Rc};

use sdl2::{
    render::{Canvas, TextureCreator},
    surface::Surface,
    video::{Window, WindowContext},
};

use crate::memory::{self, Memory};

pub struct Video {
    window: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    memory: Rc<RefCell<Memory>>,
}

impl Video {
    pub fn new(window: Canvas<Window>, memory: Rc<RefCell<Memory>>) -> Self {
        let texture_creator = window.texture_creator();
        Self {
            window,
            texture_creator,
            memory,
        }
    }

    pub fn display(&mut self) {
        let video_mode = self.memory.borrow()[0x0400_0000] & 0x7;
        match video_mode {
            0x3 => self.video_mode_3(),
            _ => ()
        }
    }

    fn video_mode_3(&mut self) {
        let mut memory_mut = self.memory.borrow_mut();
        let screen = Surface::from_data(
            &mut memory_mut[0x0600_0000..0x0601_2C00],
            240,
            160,
            480,
            sdl2::pixels::PixelFormatEnum::RGBA5551,
        ).unwrap();
        self.texture_creator.create_texture_from_surface(screen).unwrap();
        self.window.present();
    }
}
