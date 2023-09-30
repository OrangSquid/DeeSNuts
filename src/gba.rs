use std::cell::RefCell;
use std::rc::Rc;

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::arm7::Arm7;
use crate::memory::Memory;
use crate::video::Video;

pub struct Gba {
    memory: Rc<RefCell<Memory>>,
    cpu: Arm7,
    video: Video,
    clock: u64
}

impl Gba {
    pub fn new(window: Canvas<Window>) -> Gba {
        let memory = Rc::new(RefCell::new(Memory::new()));
        Gba {
            memory: Rc::clone(&memory),
            cpu: Arm7::new(Rc::clone(&memory)),
            video: Video::new(window, Rc::clone(&memory)),
            clock: 0
        }
    }

    pub fn load_bios(&mut self, bios: Vec<u8>) {
        self.memory.borrow_mut().load_bios(bios);
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory.borrow_mut().load_rom(rom);
    }

    fn tick(&mut self) {
        self.clock = self.clock.wrapping_add(1);
    }

    pub fn next(&mut self) {
        self.cpu.next();
        self.video.tick();
        self.tick();
    }
}
