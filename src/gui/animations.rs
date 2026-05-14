// GUI/ANIMATIONS
// TODO

use alloc::vec::Vec;
use crate::components::framebuffer::Framebuffer;
use crate::gui::pages::Page;
use crate::components::ft3168::SwipeDirection;
use crate::state;

pub struct SwipeAnimator {
    snap_current: Vec<u16>,
    snap_target: Vec<u16>,
    direction: Option<SwipeDirection>,
    start_x: u16,
    offset: u16,
    target_page: Option<Page>,
    active: bool,
}

impl SwipeAnimator {
    pub fn new() -> Self {
        let pixel_count = (state::LCD_WIDTH as usize) * (state::LCD_HEIGHT as usize);
        Self {
            snap_current: alloc::vec![0u16; pixel_count],
            snap_target: alloc::vec![0u16; pixel_count],
            direction: None,
            start_x: 0,
            offset: 0,
            target_page: None,
            active: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start_swipe(&mut self, fb: &Framebuffer, start_x: u16, direction: SwipeDirection) {
        self.direction = Some(direction);
        self.start_x = start_x;
        self.offset = 0;
        self.active = true;
        self.target_page = None;
        self.snap_current.copy_from_slice(fb.buffer());
    }

    pub fn set_target(&mut self, fb: &Framebuffer, target: Page) {
        self.snap_target.copy_from_slice(fb.buffer());
        self.target_page = Some(target);
    }

    pub fn update(
        &mut self,
        display: &mut crate::components::co5300::Co5300Display,
        touch_x: u16,
    ) {
        if !self.active || self.direction.is_none() {
            return;
        }

        let screen_w = state::LCD_WIDTH as i32;
        let w = screen_w as usize;
        let h = state::LCD_HEIGHT as usize;

        let dir = self.direction.unwrap();

        let delta = touch_x as i32 - self.start_x as i32;
        let raw_offset = match dir {
            SwipeDirection::Left => delta,
            SwipeDirection::Right => -delta,
            _ => 0,
        };
        let offset = raw_offset.clamp(0, screen_w) as u16;
        self.offset = offset;

        if offset == 0 || offset as usize >= w {
            return;
        }

        let offset_usize = offset as usize;

        if dir == SwipeDirection::Left {
            display.set_addr_window(0, 0, (w - offset_usize) as u16, h as u16);
            display.bus_mut().begin_pixels();
            for row in 0..h {
                let start = row * w + offset_usize;
                let end = row * w + w;
                display.bus_mut().stream_pixels(&self.snap_current[start..end]);
            }
            display.bus_mut().end_pixels();

            display.set_addr_window((w - offset_usize) as u16, 0, offset_usize as u16, h as u16);
            display.bus_mut().begin_pixels();
            for row in 0..h {
                let start = row * w;
                let end = row * w + offset_usize;
                display.bus_mut().stream_pixels(&self.snap_target[start..end]);
            }
            display.bus_mut().end_pixels();
        } else {
            display.set_addr_window(0, 0, offset_usize as u16, h as u16);
            display.bus_mut().begin_pixels();
            for row in 0..h {
                let start = row * w + (w - offset_usize);
                let end = row * w + w;
                display.bus_mut().stream_pixels(&self.snap_target[start..end]);
            }
            display.bus_mut().end_pixels();

            display.set_addr_window(offset_usize as u16, 0, (w - offset_usize) as u16, h as u16);
            display.bus_mut().begin_pixels();
            for row in 0..h {
                let start = row * w;
                let end = row * w + (w - offset_usize);
                display.bus_mut().stream_pixels(&self.snap_current[start..end]);
            }
            display.bus_mut().end_pixels();
        }
    }

    pub fn end_drag(
        &mut self,
        display: &mut crate::components::co5300::Co5300Display,
    ) -> Option<Page> {
        if !self.active {
            return None;
        }
        self.active = false;

        let threshold = (state::LCD_WIDTH as u16) / 3;
        if self.offset >= threshold {
            display.set_addr_window(0, 0, state::LCD_WIDTH as u16, state::LCD_HEIGHT as u16);
            display.bus_mut().write_pixels(&self.snap_target);
            self.target_page
        } else {
            display.set_addr_window(0, 0, state::LCD_WIDTH as u16, state::LCD_HEIGHT as u16);
            display.bus_mut().write_pixels(&self.snap_current);
            None
        }
    }
}

#[derive(Debug)]
pub enum AnimCommand {
    Start {
        start_x: u16,
        direction: SwipeDirection,
    },
    Drag {
        x: u16,
    },
    End,
}
