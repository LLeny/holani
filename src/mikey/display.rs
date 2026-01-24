use super::{Deserialize, MikeyRegisters, Serialize};
use crate::alloc;
use crate::consts::{
    LCD_PIXEL_TICK_PERIOD, LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH, SCREEN_BUFFER_LEN,
};
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use log::trace;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
enum LCDState {
    Vblank,
    Active,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LCD {
    state: LCDState,
    #[serde(skip)]
    #[serde(default = "create_argb_buffer")]
    argb_buffer: [u32; SCREEN_BUFFER_LEN],
    buffer_index: usize,
    pixel_pen_buffer: VecDeque<u8>,
    hsync_delay_tick_count: isize,
    hsync_delay_ticks_delay: isize,
    pixel_period_delay: isize,
    row_pixel_count: usize,
}

fn create_argb_buffer() -> [u32; SCREEN_BUFFER_LEN] {
    [0; SCREEN_BUFFER_LEN]
}

impl LCD {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: LCDState::Vblank,
            argb_buffer: [0; SCREEN_BUFFER_LEN],
            buffer_index: 0,
            pixel_pen_buffer: VecDeque::with_capacity(LYNX_SCREEN_WIDTH * 2),
            hsync_delay_tick_count: 0,
            hsync_delay_ticks_delay: 0,
            pixel_period_delay: 0,
            row_pixel_count: 0,
        }
    }

    pub fn reset(&mut self) {
        trace!("reset buf index:{}", self.buffer_index);
        self.buffer_index = 0;
        self.pixel_pen_buffer.clear();
        self.hsync(LYNX_SCREEN_HEIGHT + 2);
    }

    fn push_pixel(&mut self, pix: u32) {
        trace!("push pixel {}", self.buffer_index);
        self.argb_buffer[self.buffer_index] = pix;
        self.buffer_index += 1;
    }

    pub fn push_pen_pixels(&mut self, buf: &[u8]) {
        self.pixel_pen_buffer.extend(buf);
    }

    #[must_use]
    pub fn screen(&self) -> &[u32; SCREEN_BUFFER_LEN] {
        &self.argb_buffer
    }

    pub fn set_pkbup(&mut self, pbkup: u8) {
        self.hsync_delay_tick_count = (isize::from(pbkup) + 1) * 60 - 1920;
        //self.hsync_delay_tick_count = 570;
    }

    pub fn hsync(&mut self, sync_count: usize) {
        self.hsync_delay_ticks_delay = self.hsync_delay_tick_count;
        self.row_pixel_count = 0;
        self.pixel_period_delay = 0;
        self.state = if (0..LYNX_SCREEN_HEIGHT).contains(&sync_count) {
            LCDState::Active
        } else {
            LCDState::Vblank
        }
    }

    pub fn tick(&mut self, regs: &MikeyRegisters) {
        self.pixel_period_delay -= 1;
        self.hsync_delay_ticks_delay -= 1;

        if self.hsync_delay_ticks_delay <= 0
            && self.pixel_period_delay <= 0
            && self.pixel_pen_buffer.len() > 0
            && self.row_pixel_count < LYNX_SCREEN_WIDTH
            && self.state == LCDState::Active
        {
            let pen = self.pixel_pen_buffer.pop_front().unwrap_or(0xFF);
            let pixel = regs.get_argb_pen(pen as usize);
            trace!("LCD Pixel Period: pixel={}, pen=0x{:08X}", pixel, pen);
            self.push_pixel(pixel);
            self.pixel_period_delay = LCD_PIXEL_TICK_PERIOD;
            self.row_pixel_count += 1;
        }
    }
}

impl Default for LCD {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
enum VideoDMAState {
    Paused,
    Active
}

#[derive(Serialize, Deserialize)]
pub struct Display {
    #[serde(skip)]
    #[serde[default="create_video_buffers"]]
    buffers: Vec<LCD>,
    draw_buffer: usize,
    lcd_refreshed: bool,
    dma_state: VideoDMAState,
}

fn create_video_buffers() -> Vec<LCD> {
    vec![LCD::new(), LCD::new()]
}

impl Display {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffers: vec![LCD::new(), LCD::new()],
            draw_buffer: 0,
            lcd_refreshed: false,
            dma_state: VideoDMAState::Paused,
        }
    }

    #[inline]
    fn swap_buffers(&mut self) {
        self.draw_buffer = 1 - self.draw_buffer;
    }

    #[inline]
    pub fn draw_buffer(&mut self) -> &mut LCD {
        &mut self.buffers[self.draw_buffer]
    }

    #[inline]
    pub fn lcd_refreshed(&mut self) -> bool {
        if self.lcd_refreshed {
            self.lcd_refreshed = false;
            true
        } else {
            false
        }
    }

    pub fn set_pbkup(&mut self, pbkup: u8) {
        self.buffers[0].set_pkbup(pbkup);
        self.buffers[1].set_pkbup(pbkup);
    }

    #[inline]
    pub fn push_video_dma_burst(&mut self, pixs: &[u8]) {
        let dma_burst: Vec<u8> = pixs.iter().flat_map(|p| [p >> 4, p & 0x0F]).collect();
        self.draw_buffer().push_pen_pixels(&dma_burst);
    }

    #[inline]
    pub fn hsync(&mut self, count: usize) {
        trace!("HSYNC count={}", count);
        self.dma_state = if (1..=LYNX_SCREEN_HEIGHT).contains(&count) {
            VideoDMAState::Active
        } else {
            VideoDMAState::Paused
        };
        if count == LYNX_SCREEN_HEIGHT + 2 {
            self.swap_buffers();
            self.draw_buffer().reset();
            self.lcd_refreshed = true;
        }
        self.draw_buffer().hsync(count);
    }

    #[inline]
    pub(crate) fn is_dma_active(&self) -> bool {
        self.dma_state == VideoDMAState::Active
    }

    #[inline]
    pub fn tick(&mut self, regs: &MikeyRegisters) {
        self.draw_buffer().tick(regs);
    }

    #[inline]
    #[must_use]
    pub fn argb_screen(&self) -> &[u32; SCREEN_BUFFER_LEN] {
        self.buffers[1 - self.draw_buffer].screen()
    }
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}
