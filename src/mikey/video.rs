use alloc::vec::Vec;
use log::trace;

use crate::*;
use super::*;

pub const LYNX_SCREEN_WIDTH: u32 = 160;
pub const LYNX_SCREEN_HEIGHT: u32 = 102;
pub const SCREEN_BUFFER_LEN: usize = (LYNX_SCREEN_WIDTH * LYNX_SCREEN_HEIGHT) as usize;
pub const RGB_SCREEN_BUFFER_LEN: usize = SCREEN_BUFFER_LEN * 3;
pub const RGBA_SCREEN_BUFFER_LEN: usize = SCREEN_BUFFER_LEN * 4;
const VBLANK_HSYNC_COUNT: u16 = 3;

#[derive(Serialize, Deserialize, Clone)]
struct VideoBuffer {
    #[serde(skip)]
    #[serde(default="create_rgb_buffer")]
    rgb_buffer: Vec<u8>,
    buffer_index: usize,
    hsync_count: u16,
    line_pixels_to_write: u8,
    hblank_video_delay: u16,
    hblank_video_delay_bkp: u16,
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
}

fn create_video_buffers() -> Vec<VideoBuffer> {
    vec![VideoBuffer::new(), VideoBuffer::new()]
}

macro_rules! pixel {
    ($p: expr) => {
        ($p.rotate_right(4) as u64)
    };
}

impl VideoBuffer {
    pub fn new() -> Self {
        Self {
            rgb_buffer: vec![0; RGB_SCREEN_BUFFER_LEN],
            buffer_index: 0,
            hsync_count: 0,
            line_pixels_to_write: LYNX_SCREEN_WIDTH as u8,
            hblank_video_delay: 0,
            hblank_video_delay_bkp: 0,
        }
    }

    pub fn set_pbkup(&mut self, pbkup: u8) {
        /* "
        Additionally, the magic 'P' counter has to be set to match the LCD scan rate. The formula is:
        INT((((line time - .5us) / 15) * 4) -1)
        " */
        self.hblank_video_delay_bkp = 5 /* ?! */ * ((pbkup as f32 + 1.0) / 4.0 * 15.0 + 0.50) as u16;      
        trace!("pbkup: {}, hblank_video_delay_bkp: {}", pbkup, self.hblank_video_delay_bkp);
    }   

    pub fn reset(&mut self) {
        trace!("reset");
        self.buffer_index = 0;
        self.hsync_count = 0;
        self.hblank_video_delay = self.hblank_video_delay_bkp;
    }

    pub fn push(&mut self, pix: &[u8; 3]) {
        trace!("push pixel {}", self.buffer_index);
        self.line_pixels_to_write -= 1;
        self.rgb_buffer[self.buffer_index..self.buffer_index+3].copy_from_slice(pix);
        self.buffer_index += 3;
    }

    pub fn h_sync(&mut self) {
        self.hsync_count += 1;
        self.hblank_video_delay = self.hblank_video_delay_bkp;
        self.line_pixels_to_write = LYNX_SCREEN_WIDTH as u8;
        trace!("hsync count:{}", self.hsync_count);
    }

    pub fn tick(&mut self) {
        if self.hblank_video_delay > 0 {
            self.hblank_video_delay -= 1;
        }
    }

    pub fn is_in_vblank(&self) -> bool {
        self.hsync_count < VBLANK_HSYNC_COUNT
    }

    pub fn is_writing_enabled(&self) -> bool {
        self.hblank_video_delay == 0 && 
        self.line_pixels_to_write > 0 && 
        self.buffer_index < RGB_SCREEN_BUFFER_LEN
    }

    pub fn screen(&self) -> &Vec<u8>{
        &self.rgb_buffer
    }
}

impl Video {
    pub fn new() -> Self {
        Self {
            buffers: vec![VideoBuffer::new(), VideoBuffer::new()],
            draw_buffer: 0,
            pix_buffer: 0,
            pix_buffer_available: 0,
            redraw_requested: false,
        }
    }

    fn swap_buffers(&mut self) {
        self.draw_buffer = 1 - self.draw_buffer;
    }

    fn draw_buffer(&mut self) -> &mut VideoBuffer {
        &mut self.buffers[self.draw_buffer]
    }

    pub fn redraw_requested(&mut self) -> bool {
        if self.redraw_requested {
            self.redraw_requested = false;
            true
        } else {
            false
        }
    }

    pub fn push_pix_buffer(&mut self, pixs: &[u8]) {
        self.pix_buffer = 
            pixel!(pixs[0]) | (pixel!(pixs[1]) << 8)  | (pixel!(pixs[2]) << 16) | (pixel!(pixs[3]) << 24) |
            (pixel!(pixs[4]) << 32) | (pixel!(pixs[5]) << 40) | (pixel!(pixs[6]) << 48) | (pixel!(pixs[7]) << 56);
        self.pix_buffer_available = 16;
        trace!("push_pix_buffer 0x{:04X}", self.pix_buffer);
    }

    pub fn pop_pixel(&mut self) -> u8 {
        let pix = (self.pix_buffer & 0b1111) as u8;
        self.pix_buffer >>= 4;
        self.pix_buffer_available -= 1;
        pix
    }

    pub fn set_pbkup(&mut self, value: u8) {
        self.buffers[0].set_pbkup(value);
        self.buffers[1].set_pbkup(value);
    }

    pub fn vsync(&mut self) {
        self.swap_buffers();
        self.draw_buffer().reset();
        self.pix_buffer_available = 0;
        self.redraw_requested = true;
    }  

    pub fn hsync(&mut self) {
        self.draw_buffer().h_sync();
    }

    fn is_available(&mut self) -> bool {
        !self.draw_buffer().is_in_vblank() && self.draw_buffer().is_writing_enabled()
    }

    pub fn tick(&mut self, regs: &MikeyRegisters) {
        self.draw_buffer().tick();

        if self.pix_buffer_available > 0 && self.is_available() {
            let pixel = self.pop_pixel();
            let rgb = regs.get_pen(pixel);
            self.buffers[self.draw_buffer].push(rgb);
        }
    }

    pub fn required_bytes(&mut self) -> Option<u16> {
        if !self.is_available() {
            return None;
        }
        match self.pix_buffer_available {
            0 => Some(self.draw_buffer().buffer_index as u16/6),
            _ => None
        }
    }

    pub fn rgb_screen(&self) -> &Vec<u8> {
        self.buffers[1-self.draw_buffer].screen()
    }
}

impl Default for Video {
    fn default() -> Self {
        Self::new()
    }
}