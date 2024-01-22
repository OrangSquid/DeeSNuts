use std::{cell::RefCell, rc::Rc};

use crate::constants::*;
use crate::memory::Memory;
use crate::scheduler::{Event, EventType};

const DISPCNT: u32 = 0x4000000;
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
        let mut vcount = self.memory.borrow_mut().get_halfword(VCOUNT, false);
        if vcount < 160 {
            self.render_line();
        }
        let mut memory = self.memory.borrow_mut();
        let dispstat = memory.get_halfword(DISPSTAT, false) & !0x2;
        vcount = (vcount + 1) % 228;
        memory.store_halfword(DISPSTAT, dispstat, false);
        memory.store_halfword(VCOUNT, vcount, false);
        drop(memory);

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
        self.frame_buffer[line..(line + SCREEN_WIDTH * 2)].copy_from_slice(&memory[(0x0600_0000 | line)..(0x600_0000 | (line + SCREEN_WIDTH * 2))]);
    }

    fn video_mode_4(&mut self) {
        let mut memory = self.memory.borrow_mut();
        let line = memory.get_halfword(VCOUNT, false) as usize * SCREEN_WIDTH;
        let frame_buffer_line = line * 2;
        let bg_memory_start = 0x600_0000 | ((memory.get_halfword(DISPCNT, false) as usize & 0x10) >> 4) * 0xA000 + line;
        let pallete = &memory[0x500_0000..0x500_01FF];
        for (vertical_line, entry) in memory[bg_memory_start..(bg_memory_start + SCREEN_WIDTH)].iter().enumerate() {
            let color = pallete[(*entry as usize) * 2] as u16 | ((pallete[(*entry as usize) * 2 + 1] as u16) << 8);
            self.frame_buffer[frame_buffer_line + vertical_line * 2] = color as u8;
            self.frame_buffer[frame_buffer_line + vertical_line * 2 + 1] = color as u8;
        }
    }
}
