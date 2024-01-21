use std::{cell::RefCell, collections::BinaryHeap, rc::Rc};

use crate::memory::Memory;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum EventType {
    EndFrame,
    HVisibleEnd,
    HBlankEnd,
    VVisibleEnd,
    VBlankEnd
}

#[derive(PartialEq, Eq)]
pub struct Event {
    timestamp: usize,
    pub event_type: EventType,
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.timestamp.partial_cmp(&self.timestamp)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Event {
    pub fn new(timestamp: usize, event_type: EventType) -> Event {
        Event {
            timestamp, event_type
        }
    }

    pub fn add_timestamp(&mut self, timestamp: usize) {
        self.timestamp += timestamp;
    }
}

pub struct Scheduler {
    queue: BinaryHeap<Event>,
    memory: Rc<RefCell<Memory>>
}

impl Scheduler {
    pub fn new(memory: Rc<RefCell<Memory>>) -> Self {
        Scheduler { queue: BinaryHeap::new(), memory }
    }

    pub fn schedule(&mut self, event: Event) {
        
    }

    pub fn schedule_from_now(&mut self, mut event: Event) {
        event.add_timestamp(self.timestamp());
        self.queue.push(event);
    }

    pub fn timestamp(&self) -> usize {
        self.memory.borrow().get_clock_cycles()
    }

    pub fn time_until_next_event(&self) -> usize {
        self.queue.peek().unwrap().timestamp.saturating_sub(self.memory.borrow().get_clock_cycles())
    }

    pub fn pop(&mut self) -> Option<Event> {
        if self.queue.peek().unwrap().timestamp <= self.timestamp() {
            self.queue.pop()
        } else {
            None
        }
    }
}