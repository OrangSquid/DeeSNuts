use std::{cell::RefCell, rc::Rc};

use sdl2::{
    render::{Canvas, TextureCreator},
    surface::Surface,
    video::{Window, WindowContext},
};

use crate::memory::Memory;

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
        self.video_mode_3();
        /* let video_mode = self.memory.borrow().get_halfword(0x0400_0000) & 0x7;
        match video_mode {
            0x3 => self.video_mode_3(),
            _ => ()
        } */
    }

    fn video_mode_3(&mut self) {
        let memory_mut = &mut self.memory.borrow_mut()[0x0600_0000..0x0601_2C00];
        let screen = Surface::from_data(
            memory_mut,
            240,
            160,
            480,
            sdl2::pixels::PixelFormatEnum::RGBA5551,
        ).unwrap();
        self.window.copy(&self.texture_creator.create_texture_from_surface(screen).unwrap(), None, None).unwrap();
        self.window.present();
     }
}
