use alloc::vec::Vec;
use log::trace;

use super::{Deserialize, MikeyRegisters, Serialize};
use crate::alloc;

pub const LYNX_SCREEN_WIDTH: u32 = 160;
pub const LYNX_SCREEN_HEIGHT: u32 = 102;
pub const SCREEN_BUFFER_LEN: usize = (LYNX_SCREEN_WIDTH * LYNX_SCREEN_HEIGHT) as usize;
pub const RGB_SCREEN_BUFFER_LEN: usize = SCREEN_BUFFER_LEN * 3;
pub const RGBA_SCREEN_BUFFER_LEN: usize = SCREEN_BUFFER_LEN * 4;
pub const RGBA_PIXEL_LEN: usize = 4;
const VBLANK_VSYNC_COUNT: u8 = 102;

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoBuffer {
    #[serde(skip)]
    #[serde(default = "create_rgba_buffer")]
    rgba_buffer: Vec<u8>,
    buffer_index: usize,
}

fn create_rgba_buffer() -> Vec<u8> {
    vec![0; RGBA_SCREEN_BUFFER_LEN]
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
    #[serde(skip)]
    #[serde[default="create_row_buffer"]]
    display_row_buffer: [u8; LYNX_SCREEN_WIDTH as usize],
    pub display_row_index: usize,
    vsync_count: u8,
}

fn create_video_buffers() -> Vec<VideoBuffer> {
    vec![VideoBuffer::new(), VideoBuffer::new()]
}

fn create_row_buffer() -> [u8; LYNX_SCREEN_WIDTH as usize] {
    [0; LYNX_SCREEN_WIDTH as usize]
}

macro_rules! pixel {
    ($p: expr) => {
        u64::from($p.rotate_right(4))
    };
}

impl VideoBuffer {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            rgba_buffer: vec![0; RGBA_SCREEN_BUFFER_LEN],
            buffer_index: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        trace!("reset buf index:{}", self.buffer_index);
        self.buffer_index = 0;
    }

    #[inline]
    pub fn push(&mut self, pix: [u8; RGBA_PIXEL_LEN]) {
        trace!("push pixel {}", self.buffer_index);
        self.rgba_buffer[self.buffer_index..self.buffer_index + RGBA_PIXEL_LEN]
            .copy_from_slice(&pix);
        self.buffer_index += RGBA_PIXEL_LEN;
    }

    #[inline]
    #[must_use]
    pub fn screen(&self) -> &Vec<u8> {
        &self.rgba_buffer
    }
}

impl Default for VideoBuffer {
    fn default() -> Self {
        Self::new()
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
            display_row_buffer: [0; LYNX_SCREEN_WIDTH as usize],
            display_row_index: 0,
            vsync_count: 0,
        }
    }

    #[inline]
    fn swap_buffers(&mut self) {
        self.draw_buffer = 1 - self.draw_buffer;
    }

    #[inline]
    pub fn draw_buffer(&mut self) -> &mut VideoBuffer {
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

    #[inline]
    #[allow(unreachable_code)]
    pub fn push_pix_buffer(&mut self, pixs: &[u8]) {
        self.pix_buffer_available = 16;

        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        unsafe {
            use core::arch::x86_64::*;

            let v = _mm_loadl_epi64(pixs.as_ptr() as *const __m128i);
            let low_shifted = _mm_and_si128(_mm_slli_epi16(v, 4), _mm_set1_epi8(0xF0u8 as i8));
            let high_shifted = _mm_and_si128(_mm_srli_epi16(v, 4), _mm_set1_epi8(0x0F as i8));
            let swapped = _mm_or_si128(low_shifted, high_shifted);
            let mut out = [0u8; 8];
            _mm_storel_epi64(out.as_mut_ptr() as *mut __m128i, swapped);
            self.pix_buffer = u64::from_le_bytes(out);

            return;
        }

        self.pix_buffer = pixel!(pixs[0])
            | (pixel!(pixs[1]) << 8)
            | (pixel!(pixs[2]) << 16)
            | (pixel!(pixs[3]) << 24)
            | (pixel!(pixs[4]) << 32)
            | (pixel!(pixs[5]) << 40)
            | (pixel!(pixs[6]) << 48)
            | (pixel!(pixs[7]) << 56);
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

    #[inline(never)]
    #[allow(unreachable_code)]
    pub fn send_row_buffer(&mut self, regs: &MikeyRegisters) {
        if self.display_row_index == 0 {
            return;
        }

        let draw_buffer = &mut self.buffers[self.draw_buffer];

        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        unsafe {
            use core::arch::x86_64::*;

            let ff = _mm_set1_epi8(-1);
            let lut_r = _mm_loadu_si128(regs.palette_r().as_ptr() as *const _);
            let lut_g = _mm_loadu_si128(regs.palette_g().as_ptr() as *const _);
            let lut_b = _mm_loadu_si128(regs.palette_b().as_ptr() as *const _);

            for i in (0..self.display_row_index).step_by(16) {
                let rgb_ptr = draw_buffer
                    .rgba_buffer
                    .as_mut_ptr()
                    .add(draw_buffer.buffer_index);

                let pixels = _mm_loadu_si128(self.display_row_buffer.as_ptr().add(i) as *const _);

                let r_values = _mm_shuffle_epi8(lut_r, pixels);
                let g_values = _mm_shuffle_epi8(lut_g, pixels);
                let b_values = _mm_shuffle_epi8(lut_b, pixels);

                let rb_lo = _mm_unpacklo_epi8(r_values, b_values);
                let rb_hi = _mm_unpackhi_epi8(r_values, b_values);
                let ga_lo = _mm_unpacklo_epi8(g_values, ff);
                let ga_hi = _mm_unpackhi_epi8(g_values, ff);

                let rgba0 = _mm_unpacklo_epi8(rb_lo, ga_lo);
                let rgba1 = _mm_unpackhi_epi8(rb_lo, ga_lo);
                let rgba2 = _mm_unpacklo_epi8(rb_hi, ga_hi);
                let rgba3 = _mm_unpackhi_epi8(rb_hi, ga_hi);

                _mm_storeu_si128(rgb_ptr as *mut __m128i, rgba0);
                _mm_storeu_si128(rgb_ptr.add(16) as *mut __m128i, rgba1);
                _mm_storeu_si128(rgb_ptr.add(32) as *mut __m128i, rgba2);
                _mm_storeu_si128(rgb_ptr.add(48) as *mut __m128i, rgba3);

                draw_buffer.buffer_index += 16 * RGBA_PIXEL_LEN;
            }

            self.display_row_index = 0;
            return;
        }

        self.display_row_buffer[0..self.display_row_index]
            .iter()
            .map(|pix| regs.get_pen(*pix))
            .for_each(|rgba| draw_buffer.push(rgba));
        self.display_row_index = 0;
    }

    #[inline]
    fn is_available(&mut self) -> bool {
        self.vsync_count < VBLANK_VSYNC_COUNT && self.display_row_index < LYNX_SCREEN_WIDTH as usize
    }

    #[inline]
    pub fn tick(&mut self) {
        if self.pix_buffer_available > 0 && self.is_available() {
            let pixel = self.pop_pixel();
            self.display_row_buffer[self.display_row_index] = pixel;
            self.display_row_index += 1;
        }
    }

    pub fn required_bytes(&mut self) -> Option<usize> {
        if !self.is_available() {
            return None;
        }
        match self.pix_buffer_available {
            0 => Some(
                self.draw_buffer().buffer_index / (RGBA_PIXEL_LEN * 2)
                    + self.display_row_index / 2,
            ),
            _ => None,
        }
    }

    #[inline]
    #[must_use]
    pub fn rgba_screen(&self) -> &Vec<u8> {
        self.buffers[1 - self.draw_buffer].screen()
    }
}

impl Default for Video {
    fn default() -> Self {
        Self::new()
    }
}
