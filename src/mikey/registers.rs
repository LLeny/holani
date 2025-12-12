use super::{
    alloc, uart, Deserialize, MikeyInstruction, Serialize, Uart, ATTEN_A, ATTEN_B, ATTEN_C,
    ATTEN_D, BLUERED0, BLUEREDF, DISPADR, GREEN0, GREENF, MIK_ADDR, MPAN, MSTEREO,
};
use alloc::vec::Vec;
use bitflags::bitflags;
use log::trace;

macro_rules! atten_left {
    ($attn_buff: ident, $channel: expr, $regs: expr) => {
        if ($regs.data(MSTEREO) & (0x10 << $channel)) != 0 {
            if ($regs.data(MPAN) & (0x10 << $channel)) != 0 {
                f32::from($regs.data($attn_buff) >> 4) / 15f32
            } else {
                0.
            }
        } else {
            1f32
        }
    };
}

macro_rules! atten_right {
    ($attn_buff: ident, $channel: expr, $regs: expr) => {
        if ($regs.data(MSTEREO) & (1 << $channel)) != 0 {
            if ($regs.data(MPAN) & (1 << $channel)) != 0 {
                f32::from($regs.data($attn_buff) & 0xF) / 15f32
            } else {
                0.
            }
        } else {
            1f32
        }
    };
}

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct SerCtlW:u8
    {
        const tx_int_en = 0b1000_0000;
        const rx_int_en = 0b0100_0000;
        const zero      = 0b0010_0000;
        const par_en    = 0b0001_0000;
        const reset_err = 0b0000_1000;
        const tx_open   = 0b0000_0100;
        const tx_brk    = 0b0000_0010;
        const par_even  = 0b0000_0001;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct SerCtlR:u8
    {
        const tx_rdy    = 0b1000_0000;
        const rx_rdy    = 0b0100_0000;
        const tx_empty  = 0b0010_0000;
        const par_err   = 0b0001_0000;
        const overrun   = 0b0000_1000;
        const frame_err = 0b0000_0100;
        const rx_brk    = 0b0000_0010;
        const par_bit   = 0b0000_0001;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct DispCtl:u8
    {
        const color      = 0b0000_1000;
        const fourbit    = 0b0000_0100;
        const flip       = 0b0000_0010;
        const dma_enable = 0b0000_0001;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikeyRegisters {
    ticks_delay: u16,
    data_r: u16,
    addr_r: u16,
    ir: MikeyInstruction,
    data: Vec<u8>,
    cart_shift: u8,
    cart_position: u16,
    audin: u16,
    serctl_r: SerCtlR,
    serctl_w: SerCtlW,
    dispctl: DispCtl,
    is_flipped: bool,
    palette_r: [u8; 16],
    palette_g: [u8; 16],
    palette_b: [u8; 16],
    attenuation_left: [f32; 4],
    attenuation_right: [f32; 4],
}

impl MikeyRegisters {
    #[must_use]
    pub fn new() -> Self {
        let mut slf = Self {
            ticks_delay: 0,
            data_r: 0,
            addr_r: 0,
            ir: MikeyInstruction::None,
            data: vec![0; 0x100],
            cart_shift: 0,
            cart_position: 0,
            audin: 0,
            serctl_r: SerCtlR::tx_rdy | SerCtlR::tx_empty,
            serctl_w: SerCtlW::empty(),
            dispctl: DispCtl::fourbit | DispCtl::dma_enable,
            is_flipped: false,
            palette_r: [0; 16],
            palette_g: [0; 16],
            palette_b: [0; 16],
            attenuation_left: [0.; 4],
            attenuation_right: [0.; 4],
        };
        slf.set_data(ATTEN_A, 0xFF);
        slf.set_data(ATTEN_B, 0xFF);
        slf.set_data(ATTEN_C, 0xFF);
        slf.set_data(ATTEN_D, 0xFF);
        slf.update_attenuations();
        slf
    }

    pub fn shift_cart_shift(&mut self, bit: u8) {
        self.cart_position = 0;
        self.cart_shift <<= 1;
        self.cart_shift |= bit;
    }

    #[must_use]
    pub fn cart_shift(&self) -> u8 {
        self.cart_shift
    }

    #[must_use]
    pub fn cart_position(&self) -> u16 {
        self.cart_position
    }

    pub fn inc_cart_position(&mut self) {
        self.cart_position = self.cart_position.saturating_add(1);
    }

    pub fn reset_cart_position(&mut self) {
        self.cart_position = 0;
    }

    pub fn reset_cart_shift(&mut self) {
        self.cart_shift = 0;
    }

    #[must_use]
    pub fn palette_r(&self) -> &[u8; 16] {
        &self.palette_r
    }

    #[must_use]
    pub fn palette_g(&self) -> &[u8; 16] {
        &self.palette_g
    }

    #[must_use]
    pub fn palette_b(&self) -> &[u8; 16] {
        &self.palette_b
    }

    #[must_use]
    pub fn data(&self, addr: u16) -> u8 {
        self.data[(addr - MIK_ADDR) as usize]
    }

    pub fn set_data(&mut self, addr: u16, mut data: u8) {
        match addr {
            GREEN0..=GREENF => {
                data &= 0x0f; // Behave as 4 bits registers.
                self.data[(addr - MIK_ADDR) as usize] = data;
                self.update_pen(addr - GREEN0);
            }
            BLUERED0..=BLUEREDF => {
                self.data[(addr - MIK_ADDR) as usize] = data;
                self.update_pen(addr - BLUERED0);
            }
            _ => self.data[(addr - MIK_ADDR) as usize] = data,
        }
        trace!("> Poke 0x{addr:04x} = 0x{data:02x}");
    }

    fn update_pen(&mut self, pen_index: u16) {
        let bluered = self.data(BLUERED0 + pen_index);
        let green = self.data(GREEN0 + pen_index);
        self.palette_r[pen_index as usize] = (bluered & 0xf) * 16;
        self.palette_g[pen_index as usize] = (green & 0xf) * 16;
        self.palette_b[pen_index as usize] = (bluered >> 4) * 16;
    }

    #[inline]
    #[must_use]
    pub fn get_pen(&self, pen_index: u8) -> [u8; 4] {
        [
            self.palette_r[pen_index as usize],
            self.palette_g[pen_index as usize],
            self.palette_b[pen_index as usize],
            0xFF,
        ]
    }

    #[must_use]
    pub fn ticks_delay(&self) -> u16 {
        self.ticks_delay
    }

    pub fn set_ticks_delay(&mut self, ticks_delay: u16) {
        self.ticks_delay = ticks_delay;
    }

    pub fn dec_ticks_delay(&mut self) {
        self.ticks_delay -= 1;
    }

    #[must_use]
    pub fn data_r(&self) -> u16 {
        self.data_r
    }

    pub fn set_data_r(&mut self, data_r: u16) {
        self.data_r = data_r;
    }

    #[must_use]
    pub fn u16(&self, addr: u16) -> u16 {
        u16::from(self.data(addr)) | (u16::from(self.data(addr + 1)) << 8)
    }

    #[must_use]
    pub fn addr_r(&self) -> u16 {
        self.addr_r
    }

    pub fn set_addr_r(&mut self, addr_r: u16) {
        self.addr_r = addr_r;
    }

    #[must_use]
    pub fn ir(&self) -> MikeyInstruction {
        self.ir
    }

    pub fn set_ir(&mut self, ir: MikeyInstruction) {
        self.ir = ir;
    }

    pub fn reset_ir(&mut self) {
        self.ir = MikeyInstruction::None;
    }

    #[must_use]
    pub fn audin(&self) -> u16 {
        self.audin
    }

    pub fn set_audin(&mut self, audin: u16) {
        self.audin = audin;
    }

    #[must_use]
    pub fn disp_addr(&self) -> u16 {
        self.u16(DISPADR)
    }

    #[must_use]
    pub fn serctl(&self) -> u8 {
        self.serctl_r.bits()
    }

    #[must_use]
    pub fn dispctl(&self) -> u8 {
        self.dispctl.bits()
    }

    pub fn set_dispctl(&mut self, v: u8) {
        self.dispctl = DispCtl::from_bits_truncate(v);
        self.is_flipped = self.dispctl.contains(DispCtl::flip);
    }

    #[must_use]
    pub fn is_flipped(&self) -> bool {
        self.is_flipped
    }

    pub fn set_serctl(&mut self, uart: &mut Uart, v: u8) {
        let brk = self.serctl_w_is_flag_set(SerCtlW::tx_brk);
        self.serctl_w = match SerCtlW::from_bits(v) {
            Some(bits) => bits,
            None => SerCtlW::empty(),
        };

        if brk && !self.serctl_w_is_flag_set(SerCtlW::tx_brk) {
            //Set redeye to high if break has been disabled
            uart.set_redeye_pin(uart::redeye_status::RedeyeStatus::High);
        }

        if self.serctl_w_is_flag_set(SerCtlW::reset_err) {
            self.serctl_r_disable_flag(SerCtlR::par_err);
            self.serctl_r_disable_flag(SerCtlR::frame_err);
            self.serctl_r_disable_flag(SerCtlR::overrun);
            self.serctl_w_disable_flag(SerCtlW::reset_err);
        }
    }

    pub fn serctl_r_enable_flag(&mut self, flag: SerCtlR) {
        self.serctl_r.set(flag, true);
    }

    pub fn serctl_r_disable_flag(&mut self, flag: SerCtlR) {
        self.serctl_r.set(flag, false);
    }

    #[must_use]
    pub fn serctl_r_is_flag_set(&self, flag: SerCtlR) -> bool {
        self.serctl_r.contains(flag)
    }

    pub fn serctl_w_enable_flag(&mut self, flag: SerCtlW) {
        self.serctl_w.set(flag, true);
    }

    pub fn serctl_w_disable_flag(&mut self, flag: SerCtlW) {
        self.serctl_w.set(flag, false);
    }

    #[must_use]
    pub fn serctl_w_is_flag_set(&self, flag: SerCtlW) -> bool {
        self.serctl_w.contains(flag)
    }

    pub fn update_attenuations(&mut self) {
        self.attenuation_left[0] = atten_left!(ATTEN_A, 0, self);
        self.attenuation_left[1] = atten_left!(ATTEN_B, 1, self);
        self.attenuation_left[2] = atten_left!(ATTEN_C, 2, self);
        self.attenuation_left[3] = atten_left!(ATTEN_D, 3, self);

        self.attenuation_right[0] = atten_right!(ATTEN_A, 0, self);
        self.attenuation_right[1] = atten_right!(ATTEN_B, 1, self);
        self.attenuation_right[2] = atten_right!(ATTEN_C, 2, self);
        self.attenuation_right[3] = atten_right!(ATTEN_D, 3, self);
    }

    #[must_use]
    pub fn attenuation_left(&self, i: usize) -> f32 {
        self.attenuation_left[i]
    }

    #[must_use]
    pub fn attenuation_right(&self, i: usize) -> f32 {
        self.attenuation_right[i]
    }
}

impl Default for MikeyRegisters {
    fn default() -> Self {
        Self::new()
    }
}
