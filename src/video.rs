use std::{cell::RefCell, rc::Rc};

use crate::constants::*;
use crate::memory::Memory;
use crate::scheduler::{Event, EventType};

const DISPSTAT: u32 = 0x4000004;
const VCOUNT: u32 = 0x4000006;

pub struct Video {
    memory: Rc<RefCell<Memory>>,
    pub frame_buffer: [u8; SCREEN_WIDTH * SCREEN_HEIGHT * 2]
}

impl Video {
    pub fn new(memory: Rc<RefCell<Memory>>) -> Self {
        Self {
            memory,
            frame_buffer: [0; SCREEN_WIDTH * SCREEN_HEIGHT * 2]
        }
    }

    pub fn h_visible_end_handler(&mut self) -> Event {
        let mut memory = self.memory.borrow_mut();
        let dispstat = memory.get_halfword(DISPSTAT, false) | 0x2;
        memory.store_halfword(DISPSTAT, dispstat, false);

        Event::new(H_BLANK, EventType::HBlankEnd)
    }

    pub fn h_blank_end_handler(&mut self) -> Event {
        let mut memory = self.memory.borrow_mut();
        let dispstat = memory.get_halfword(DISPSTAT, false) & !0x2;
        let vcount = (memory.get_halfword(VCOUNT, false) + 1) % 228;
        memory.store_halfword(DISPSTAT, dispstat, false);
        memory.store_halfword(VCOUNT, vcount, false);
        drop(memory);
        if vcount < 160 {
            self.render_line();
        }

        Event::new(VISIBLE_H, EventType::HVisibleEnd)
    }

    pub fn v_visible_end_handler(&mut self) -> Event {
        let mut memory = self.memory.borrow_mut();
        let dispstat = memory.get_halfword(DISPSTAT, false) | 0x1;
        memory.store_halfword(DISPSTAT, dispstat, false);

        Event::new(V_BLANK, EventType::VBlankEnd)
    }

    pub fn v_blank_end_handler(&mut self) -> Event {
        let mut memory = self.memory.borrow_mut();
        let dispstat = memory.get_halfword(DISPSTAT, false) & !0x1;
        memory.store_halfword(DISPSTAT, dispstat, false);

        Event::new(VISIBLE_V, EventType::VVisibleEnd)
    }

    pub fn render_line(&mut self) {
        let video_mode = self.memory.borrow_mut().get_halfword(0x0400_0000, false) & 0x7;
        match video_mode {
            0x3 => self.video_mode_3(),
            0x4 => self.video_mode_4(),
            _ => ()
        }
    }

    fn video_mode_3(&mut self) {
        let mut memory = self.memory.borrow_mut();
        let line = memory.get_halfword(VCOUNT, false) as usize * SCREEN_WIDTH * 2;
        if line > 76800 {
            println!("aaaa");
        }
        self.frame_buffer[line..(line + SCREEN_WIDTH * 2)].copy_from_slice(&memory[(0x0600_0000 | line)..(0x600_0000 | (line + SCREEN_WIDTH * 2))]);
    }

    fn video_mode_4(&mut self) {
        
    }
}
