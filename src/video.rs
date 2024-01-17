use std::{cell::RefCell, rc::Rc};

use sdl2::{
    render::{Canvas, TextureCreator},
    surface::Surface,
    video::{Window, WindowContext},
};

use crate::memory::Memory;

const VISIBLE_V: u64 = 197120;
const V_BLANK: u64 = 83776;
const VISIBLE_H: u64 = 960;
const H_BLANK: u64 = 272;
const ACTUAL_VISIBLE_H: u64 = 1006;
const DRAW_LINE: u64 = VISIBLE_H + H_BLANK;

pub struct Video {
    window: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    memory: Rc<RefCell<Memory>>,
    v_clock: u64,
    h_clock: u64
}

impl Video {
    pub fn new(window: Canvas<Window>, memory: Rc<RefCell<Memory>>) -> Self {
        let texture_creator = window.texture_creator();
        Self {
            window,
            texture_creator,
            memory,
            v_clock: 0,
            h_clock: 0
        }
    }

    pub fn tick(&mut self) {
        let mut memory = self.memory.borrow_mut();
        let dispstat = memory.get_halfword(0x4000004);
        self.h_clock += 1;
        if self.h_clock == ACTUAL_VISIBLE_H {
            memory.store_halfword(0x4000004, dispstat | 0x2);
        } else if self.h_clock > DRAW_LINE {
            memory.store_halfword(0x4000004, dispstat & !0x2);
            self.h_clock = 0;
        }
        self.v_clock += 1;
        if self.v_clock == DRAW_LINE * 160 {
            memory.store_halfword(0x4000004, dispstat | 0x1);
        } else if self.v_clock > DRAW_LINE * 226 {
            memory.store_halfword(0x4000004, dispstat & !0x1);
            drop(memory);
            self.v_clock = 0;
            self.display();
        }
    }

    pub fn display(&mut self) {
        let video_mode = self.memory.borrow_mut().get_halfword(0x0400_0000) & 0x7;
        match video_mode {
            0x3 => self.video_mode_3(),
            _ => ()
        }
    }

    fn video_mode_3(&mut self) {
        let memory_mut = &mut self.memory.borrow_mut()[0x0600_0000..0x0601_2C00];
        let screen = Surface::from_data(
            memory_mut,
            240,
            160,
            480,
            sdl2::pixels::PixelFormatEnum::BGR555,
        ).unwrap();
        self.window.copy(&self.texture_creator.create_texture_from_surface(screen).unwrap(), None, None).unwrap();
        self.window.present();
     }
}
