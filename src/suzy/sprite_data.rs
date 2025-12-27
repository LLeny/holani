use log::trace;
use mikey::video::LYNX_SCREEN_WIDTH;

use super::{
    mikey, Deserialize, Serialize, SuzyRegisters, COLLADRL, COLLBASL, LINE_END, SPRCTL1_LITERAL,
    SUZY_DATA_BUFFER_LEN, VIDADRL, VIDBASL,
};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LineType {
    Error,
    AbsLiteral,
    Literal,
    Packed,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SpriteData {
    shift_reg: u64,
    bits_left: u16,
    shift_reg_count: u16,
    repeat_count: u16,
    line_pixel: u8,
    line_type: LineType,
}

impl SpriteData {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shift_reg: 0,
            bits_left: 0xffff,
            shift_reg_count: 0,
            repeat_count: 0,
            line_pixel: 0,
            line_type: LineType::Error,
        }
    }

    pub fn reset(&mut self, regs: &mut SuzyRegisters) {
        self.shift_reg = 0;
        self.shift_reg_count = 0;
        self.repeat_count = 0;
        self.line_pixel = 0;
        self.line_type = LineType::Error;
        self.bits_left = 0xffff;
        regs.set_tmp_addr(regs.sprdline());

        trace!("reset");
    }

    /// Initializes the sprite data with the given registers and vertical offset.
    ///
    /// # Errors
    ///
    /// Returns an error if there is not enough data available to read the offset.
    pub fn initialize(&mut self, regs: &mut SuzyRegisters, voff: i16) -> Result<u16, &'static str> {
        let offset = match self.get_bits(8) {
            None => return Err("Not enough data available"),
            Some(v) => u16::from(v),
        };

        self.bits_left = offset.saturating_sub(1).saturating_mul(8);

        if regs.sprctl1() & SPRCTL1_LITERAL != 0 {
            self.line_type = LineType::AbsLiteral;
            self.repeat_count = self.bits_left / u16::from(regs.bpp() + 1);
        }

        let sprvpos2 = voff * LYNX_SCREEN_WIDTH as i16 / 2;

        regs.set_i16(VIDADRL, regs.i16(VIDBASL) + sprvpos2);

        regs.set_i16(COLLADRL, regs.i16(COLLBASL) + sprvpos2);

        trace!(
            "initialize({}) offset:{} bits_left:{}",
            voff,
            offset,
            self.bits_left
        );

        Ok(offset)
    }

    pub fn push_data(&mut self, data: u8) {
        self.shift_reg <<= SUZY_DATA_BUFFER_LEN * 8;
        self.shift_reg |= u64::from(data) << ((SUZY_DATA_BUFFER_LEN - 1) * 8);
        self.shift_reg_count += SUZY_DATA_BUFFER_LEN * 8;
        trace!(
            "Push shift_reg 0x{:08x} shift_reg_count:{}",
            self.shift_reg,
            self.shift_reg_count
        );
    }

    fn peek_bits(&self, start: u16, bits: u16) -> Option<(u8, u8)> {
        let end = start + bits;

        if self.bits_left <= end {
            trace!(
                "peek_bits({},{}) no bits left {}.",
                start,
                bits,
                self.bits_left
            );
            return Some((0, 0));
        }

        if self.shift_reg_count < end {
            trace!(
                "peek_bits({},{}) shift_reg_count too low {}.",
                start,
                bits,
                self.shift_reg_count
            );
            return None;
        }

        let shift = self.shift_reg_count - end;
        let mask = (1 << bits) - 1;
        let ret = ((self.shift_reg >> shift) & mask) as u32;

        trace!("peek_bits({start},{bits}) -> {ret}");

        Some((ret as u8, bits as u8))
    }

    pub fn get_bits(&mut self, bits: u16) -> Option<u8> {
        if let Some(ret) = self.peek_bits(0, bits) {
            self.shift_reg_count -= u16::from(ret.1);
            self.bits_left -= u16::from(ret.1);
            self.shift_reg &= (1 << self.shift_reg_count) - 1;
            trace!("get_bits({}) -> {}, bits_left:{}, shift_reg_count:{}", bits, ret.0, self.bits_left, self.shift_reg_count);
            Some(ret.0)
        } else {
            None
        }
    }

    /// Gets the next pixel from the sprite line.
    ///
    /// # Errors
    ///
    /// Returns an error if there is not enough data in the buffer to process the next pixel.
    pub fn line_get_pixel(
        &mut self,
        regs: &mut SuzyRegisters,
        pens: &[u8; 16],
    ) -> Result<u8, &'static str> {
        trace!("- line_get_pixel");
        let bpp: u16 = u16::from(regs.bpp()) + 1;
        let mut bit_count: u16 = 0;
        let mut line_pixel = self.line_pixel;
        let mut line_type = self.line_type;
        let mut repeat_count = self.repeat_count;

        let peek_or_err = |start: u16, bits: u16| -> Result<u8, &'static str> {
            self.peek_bits(start, bits)
                .ok_or("Not enough data to peek bits")
                .map(|(v, _)| v)
        };

        if repeat_count == 0 {
            if line_type != LineType::AbsLiteral {
                let literal = peek_or_err(0, 1)?;
                bit_count += 1;
                line_type = if literal == 1 {
                    LineType::Literal
                } else {
                    LineType::Packed
                };
            }

            match line_type {
                LineType::Literal => {
                    let count = peek_or_err(bit_count, 4)?;
                    bit_count += 4;
                    repeat_count = u16::from(count) + 1;
                }
                LineType::Packed => {
                    let count = peek_or_err(bit_count, 4)?;
                    bit_count += 4;
                    repeat_count = u16::from(count);
                    if repeat_count == 0 {
                        line_pixel = LINE_END;
                    } else {
                        let bits = peek_or_err(bit_count, bpp)?;
                        bit_count += bpp;
                        line_pixel = pens[bits as usize];
                    }
                    repeat_count += 1;
                }
                LineType::AbsLiteral | LineType::Error => line_pixel = LINE_END,
            }
        }

        if line_pixel != LINE_END {
            repeat_count = repeat_count.saturating_sub(1);
            match line_type {
                LineType::AbsLiteral => {
                    let pixel = peek_or_err(bit_count, bpp)?;
                    bit_count += bpp;
                    line_pixel = pixel;
                    if repeat_count == 0 && line_pixel == 0 {
                        line_pixel = LINE_END;
                    } else {
                        line_pixel = pens[line_pixel as usize];
                    }
                }
                LineType::Literal => {
                    let bits = peek_or_err(bit_count, bpp)?;
                    bit_count += bpp;
                    line_pixel = pens[bits as usize];
                }
                LineType::Packed => (),
                LineType::Error => line_pixel = LINE_END,
            }
        }

        self.line_pixel = line_pixel;
        self.line_type = line_type;
        self.repeat_count = repeat_count;
        let _ = self.get_bits(bit_count);
        Ok(line_pixel)
    }

    #[must_use]
    pub fn shift_reg_count(&self) -> u16 {
        self.shift_reg_count
    }
}

impl Default for SpriteData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peek_bits() {
        let mut sprite_data = SpriteData::new();
        sprite_data.push_data(0b1010_1100u8);
        sprite_data.push_data(0b0101_0110u8);

        assert_eq!(sprite_data.peek_bits(0, 4), Some((0b1010, 4)));
        assert_eq!(sprite_data.peek_bits(4, 4), Some((0b1100, 4)));
        assert_eq!(sprite_data.peek_bits(8, 4), Some((0b0101, 4)));
        assert_eq!(sprite_data.peek_bits(12, 4), Some((0b0110, 4)));
        assert_eq!(sprite_data.peek_bits(1, 4), Some((0b0101, 4)));
        assert_eq!(sprite_data.peek_bits(5, 2), Some((0b10, 2)));
        assert_eq!(sprite_data.peek_bits(5, 4), Some((0b1000, 4)));
    }
}
