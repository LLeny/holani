use log::trace;
use mikey::video::LYNX_SCREEN_WIDTH;

use super::*;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LineType {
    Error,
    AbsLiteral,
    Literal,
    Packed,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SpriteData {
    shift_reg: u16,
    bits_left: u16,
    shift_reg_count: u16,
    repeat_count: u16,
    line_pixel: u32,
    line_type: LineType,
    addr: u16,
}

impl SpriteData {
    pub fn new () -> Self {
        Self {
            shift_reg: 0,
            bits_left: 0xffff,
            shift_reg_count: 0,
            repeat_count: 0,
            line_pixel: 0,
            line_type: LineType::Error,
            addr: 0,
        }
    }

    pub fn reset(&mut self, regs: &mut SuzyRegisters)
    {
        self.shift_reg = 0;
        self.shift_reg_count = 0;
        self.repeat_count = 0;
        self.line_pixel = 0;
        self.line_type = LineType::Error;  
        self.bits_left = 0xffff;   
        self.addr = regs.u16(SPRDLINEL);
        trace!("reset");
    }

    pub fn initialize(&mut self, regs: &mut SuzyRegisters, voff: i16) -> Result<u16, &'static str> {

        let offset = match self.get_bits(8) {
            None => return Err("Not enough data available"),
            Some(v) => v as u16,
        };

        self.bits_left = offset.overflowing_sub(1).0.overflowing_mul(8).0;

        if regs.sprctl1() & SPRCTL1_LITERAL != 0 {
            self.line_type = LineType::AbsLiteral;
            self.repeat_count = self.bits_left / (regs.bpp() + 1) as u16;
        }

        let sprvpos2 = voff * LYNX_SCREEN_WIDTH as i16 / 2;

        regs.set_i16(VIDADRL, regs.i16(VIDBASL) + sprvpos2);

        regs.set_i16(COLLADRL, regs.i16(COLLBASL) + sprvpos2);

        trace!("initialize({}) offset:{} bits_left:{}", voff, offset, self.bits_left);
    
        Ok(offset)
    }

    pub fn push_data(&mut self, data: u8) {
        self.shift_reg <<= SUZY_DATA_BUFFER_LEN * 8;
        self.shift_reg |= (data as u16) << ((SUZY_DATA_BUFFER_LEN - 1) * 8);    
        self.shift_reg_count += SUZY_DATA_BUFFER_LEN * 8;
        trace!("Push shift_reg 0x{:08x} shift_reg_count:{}", self.shift_reg, self.shift_reg_count);
    }

    pub fn get_bits(&mut self, bits: u16) -> Option<u32> {
        let mut ret = 0;

        if self.bits_left <= bits {
            trace!("get_bits({}) no bits left {}.", bits, self.bits_left);
            return Some(ret);
        }

        if self.shift_reg_count < bits {
            trace!("get_bits({}) shift_reg_count too low {}.", bits, self.shift_reg_count);
            return None;
        }

        ret = (self.shift_reg >> (self.shift_reg_count - bits)) as u32;
        ret &= (1 << bits) - 1;
    
        self.shift_reg_count -= bits;
        self.bits_left -= bits;

        trace!("get_bits({}), shift_reg_count {}, bits_left {} -> {}", bits, self.shift_reg_count, self.bits_left, ret);

        Some(ret)
    }

    pub fn line_get_pixel(&mut self, regs: &mut SuzyRegisters, pens: &[u8; 16]) -> Result<u32, &'static str> {
        trace!("- line_get_pixel");
        if self.shift_reg_count < 9 {
            trace!("line_get_pixel buffer too low");
            return Err("Data buffer too low");
        }

        let bpp : u16 = regs.bpp() as u16 + 1;

        if 0 == self.repeat_count {
            if self.line_type != LineType::AbsLiteral {
                let literal = self.get_bits(1).unwrap();
                if literal == 1 {
                    self.line_type = LineType::Literal;
                } else {
                    self.line_type = LineType::Packed;
                }
            }
    
            match self.line_type {
                LineType::AbsLiteral => {
                    self.line_pixel = LINE_END;
                    return Result::Ok(self.line_pixel); 
                }
                LineType::Literal => {
                    self.repeat_count = self.get_bits(4).unwrap() as u16;
                    self.repeat_count += 1;
                }
                LineType::Packed => {
                    self.repeat_count = self.get_bits(4).unwrap() as u16;
                    if self.repeat_count == 0 {
                        self.line_pixel = LINE_END;
                    } else {
                        let bits = self.get_bits(bpp).unwrap() as u8;
                        self.line_pixel = pens[bits as usize] as u32;
                    }
                    self.repeat_count += 1;
                }
                _ => return Ok(0),
            }
        }
    
        if self.line_pixel != LINE_END {
            self.repeat_count -= 1;
            match self.line_type {
                LineType::AbsLiteral => {
                    self.line_pixel = self.get_bits(bpp).unwrap();
                    if self.repeat_count == 0 && self.line_pixel == 0 {
                        self.line_pixel = LINE_END;
                    } else {
                        self.line_pixel = pens[self.line_pixel as usize] as u32;
                    }
                }
                LineType::Literal => {
                    let bits = self.get_bits(bpp).unwrap() as u8;
                    self.line_pixel = pens[bits as usize] as u32;
                }
                LineType::Packed => (),
                _ => return Ok(0),
            }
        }
    
        Ok(self.line_pixel)
    }
    
    pub fn addr(&self) -> u16 {
        self.addr
    }
    
    pub fn set_addr(&mut self, addr: u16) {
        self.addr = addr;
    }
    
    pub fn shift_reg_count(&self) -> u16 {
        self.shift_reg_count
    }
}

impl Default for SpriteData {
    fn default() -> Self {
        Self::new()
    }
}
