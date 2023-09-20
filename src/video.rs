use sdl2::{video::Window, render::Canvas};

pub struct Video {
    window: Canvas<Window>
}

impl Video {
    pub fn new(window: Canvas<Window>) -> Self {
        Self {
            window
        }
    }
}