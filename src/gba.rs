use std::cell::RefCell;
use std::rc::Rc;

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::arm7::Arm7;
use crate::memory::Memory;
use crate::video::Video;

const REFRESH_RATE: f64 = 1.0 / 60.0;

pub struct Gba {
    memory: Rc<RefCell<Memory>>,
    cpu: Arm7,
    video: Video
}

impl Gba {
    pub fn new(window: Canvas<Window>) -> Gba {
        let memory = Rc::new(RefCell::new(Memory::new()));
        Gba {
            memory: Rc::clone(&memory),
            cpu: Arm7::new(Rc::clone(&memory)),
            video: Video::new(window, Rc::clone(&memory)),
        }
    }

    pub fn load_bios(&mut self, bios: Vec<u8>) {
        self.memory.borrow_mut().load_bios(bios);
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory.borrow_mut().load_rom(rom);
    }

    pub fn next(&mut self) {
        self.cpu.next();
    }

    pub fn display(&mut self) {
        self.video.display();
    }

    pub fn lamo(&mut self) {
        self.cpu.cpsr_register;
    }
}
