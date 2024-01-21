use std::cell::RefCell;
use std::rc::Rc;

use crate::arm7::cpu::Cpu;
use crate::constants::{VISIBLE_H, VISIBLE_V, V_BLANK};
use crate::memory::Memory;
use crate::scheduler::{Event, Scheduler, EventType};
use crate::video::Video;

pub struct Gba {
    memory: Rc<RefCell<Memory>>,
    cpu: Cpu,
    video: Video,
    scheduler: Scheduler,
    frames: usize,
    overshot: usize
}

impl Gba {
    pub fn new() -> Gba {
        let memory = Rc::new(RefCell::new(Memory::new()));
        let mut scheduler = Scheduler::new(Rc::clone(&memory));
        scheduler.schedule_from_now(Event::new(VISIBLE_H, EventType::HVisibleEnd));
        Gba {
            memory: Rc::clone(&memory),
            cpu: Cpu::new(Rc::clone(&memory)),
            video: Video::new(Rc::clone(&memory)),
            scheduler,
            frames: 0,
            overshot: 0
        }
    }

    pub fn load_bios(&mut self, bios: Vec<u8>) {
        self.memory.borrow_mut().load_bios(bios);
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory.borrow_mut().load_rom(rom);
    }

    fn run(&mut self, cycles: usize) {
        let start_time = self.scheduler.timestamp();
        let next_frame = self.frames + 1;
        let frame_end = Event::new(cycles - self.overshot, crate::scheduler::EventType::EndFrame);
        self.scheduler.schedule_from_now(frame_end);
        while self.frames != next_frame {
            while self.scheduler.time_until_next_event() > 0 {
                self.next()
            }

            self.handle_events();
        }
        let end_time = self.scheduler.timestamp();
        self.overshot = start_time.saturating_sub(end_time).saturating_sub(cycles);
    }

    pub fn frame(&mut self) {
        self.run(VISIBLE_V + V_BLANK);
    }

    pub fn next(&mut self) {
        self.cpu.next();
    }

    pub fn get_frame_buffer(&mut self) -> &mut [u8] {
        &mut self.video.frame_buffer
    }

    fn handle_events(&mut self) {
        while let Some(event) = self.scheduler.pop() {
            let new_event = match event.event_type {
                EventType::EndFrame => {
                    self.frames += 1;
                    None
                },
                EventType::HVisibleEnd => Some(self.video.h_visible_end_handler()),
                EventType::HBlankEnd => Some(self.video.h_blank_end_handler()),
                EventType::VVisibleEnd => Some(self.video.v_visible_end_handler()),
                EventType::VBlankEnd => Some(self.video.v_blank_end_handler()),
            };
            if new_event.is_some() {
                self.scheduler.schedule_from_now(new_event.unwrap())
            }
        }
    }
}
