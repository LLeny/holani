use alloc::vec::Vec;
use log::trace;

use super::{Deserialize, MikeyRegisters, Serialize};
use crate::alloc;

pub const LYNX_SCREEN_WIDTH: u32 = 160;
pub const LYNX_SCREEN_HEIGHT: u32 = 102;
pub const SCREEN_BUFFER_LEN: usize = (LYNX_SCREEN_WIDTH * LYNX_SCREEN_HEIGHT) as usize;
pub const RGB_SCREEN_BUFFER_LEN: usize = SCREEN_BUFFER_LEN * 3;
pub const RGBA_SCREEN_BUFFER_LEN: usize = SCREEN_BUFFER_LEN * 4;
const VBLANK_VSYNC_COUNT: u8 = 102;

#[derive(Serialize, Deserialize, Clone)]
struct VideoBuffer {
    #[serde(skip)]
    #[serde(default = "create_rgb_buffer")]
    rgb_buffer: Vec<u8>,
    buffer_index: usize,
}

fn create_rgb_buffer() -> Vec<u8> {
    vec![0; RGB_SCREEN_BUFFER_LEN]
}

#[derive(Serialize, Deserialize)]
pub struct Video {
    #[serde(skip)]
    #[serde[default="create_video_buffers"]]
    buffers: Vec<VideoBuffer>,
    draw_buffer: usize,
    pix_buffer: u64,
    pix_buffer_available: u8,
    redraw_requested: bool,
    display_row_buffer: Vec<u8>,
    vsync_count: u8,
}

fn create_video_buffers() -> Vec<VideoBuffer> {
    vec![VideoBuffer::new(), VideoBuffer::new()]
}

macro_rules! pixel {
    ($p: expr) => {
        u64::from($p.rotate_right(4))
    };
}

impl VideoBuffer {
    pub fn new() -> Self {
        Self {
            rgb_buffer: vec![0; RGB_SCREEN_BUFFER_LEN],
            buffer_index: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        trace!("reset buf index:{}", self.buffer_index);
        self.buffer_index = 0;
    }

    #[inline]
    pub fn push(&mut self, pix: [u8; 3]) {
        trace!("push pixel {}", self.buffer_index);
        self.rgb_buffer[self.buffer_index..self.buffer_index + 3].copy_from_slice(&pix);
        self.buffer_index += 3;
    }

    #[inline]
    pub fn screen(&self) -> &Vec<u8> {
        &self.rgb_buffer
    }
}

impl Video {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffers: vec![VideoBuffer::new(), VideoBuffer::new()],
            draw_buffer: 0,
            pix_buffer: 0,
            pix_buffer_available: 0,
            redraw_requested: false,
            display_row_buffer: vec![],
            vsync_count: 0,
        }
    }

    #[inline]
    fn swap_buffers(&mut self) {
        self.draw_buffer = 1 - self.draw_buffer;
    }

    #[inline]
    fn draw_buffer(&mut self) -> &mut VideoBuffer {
        &mut self.buffers[self.draw_buffer]
    }

    #[inline]
    pub fn redraw_requested(&mut self) -> bool {
        if self.redraw_requested {
            self.redraw_requested = false;
            true
        } else {
            false
        }
    }

    pub fn push_pix_buffer(&mut self, pixs: &[u8]) {
        self.pix_buffer = pixel!(pixs[0])
            | (pixel!(pixs[1]) << 8)
            | (pixel!(pixs[2]) << 16)
            | (pixel!(pixs[3]) << 24)
            | (pixel!(pixs[4]) << 32)
            | (pixel!(pixs[5]) << 40)
            | (pixel!(pixs[6]) << 48)
            | (pixel!(pixs[7]) << 56);
        self.pix_buffer_available = 16;
        trace!("push_pix_buffer 0x{:04X}", self.pix_buffer);
    }

    #[inline]
    pub fn pop_pixel(&mut self) -> u8 {
        let pix = (self.pix_buffer & 0b1111) as u8;
        self.pix_buffer >>= 4;
        self.pix_buffer_available -= 1;
        pix
    }

    #[inline]
    pub fn hsync(&mut self, count: u8, regs: &MikeyRegisters) {
        self.vsync_count = count;
        self.send_row_buffer(regs);
        if count == VBLANK_VSYNC_COUNT + 2 {
            self.swap_buffers();
            self.draw_buffer().reset();
            self.pix_buffer_available = 0;
            self.redraw_requested = true;
        }
    }

    fn send_row_buffer(&mut self, regs: &MikeyRegisters) {
        let draw_buffer = &mut self.buffers[self.draw_buffer];
        for pixel in &self.display_row_buffer {
            let rgb = regs.get_pen(*pixel);
            draw_buffer.push(*rgb);
        }
        self.display_row_buffer.clear();
    }

    #[inline]
    fn is_available(&mut self) -> bool {
        self.vsync_count < VBLANK_VSYNC_COUNT
            && self.display_row_buffer.len() < LYNX_SCREEN_WIDTH as usize
    }

    #[inline]
    pub fn tick(&mut self) {
        if self.pix_buffer_available > 0 && self.is_available() {
            let pixel = self.pop_pixel();
            self.display_row_buffer.push(pixel);
        }
    }

    pub fn required_bytes(&mut self) -> Option<u16> {
        if !self.is_available() {
            return None;
        }
        match self.pix_buffer_available {
            0 => Some(
                self.draw_buffer().buffer_index as u16 / 6
                    + self.display_row_buffer.len() as u16 / 2,
            ),
            _ => None,
        }
    }

    #[inline]
    #[must_use]
    pub fn rgb_screen(&self) -> &Vec<u8> {
        self.buffers[1 - self.draw_buffer].screen()
    }
}

impl Default for Video {
    fn default() -> Self {
        Self::new()
    }
}
