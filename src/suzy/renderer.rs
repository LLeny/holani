use crate::{mikey, suzy};
use log::trace;
use mikey::video::{LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH};
use sprite_data::SpriteData;
use suzy::{
    sprite_data, Deserialize, Ram, Serialize, SprSysR, SprSysW, SuzyRegisters, SuzyTask, TaskStep,
    COLLADRL, COLLOFFL, HOFFL, HPOSSTRTH, HPOSSTRTL, HSIZOFFL, LINE_END, RAM_PAGE_READ_TICKS,
    SCBNEXTH, SCBNEXTL, SPRCOLL, SPRCOLL_DONT_COLLIDE, SPRCOLL_NUMBER, SPRCTL0, SPRCTL0_HFLIP,
    SPRCTL0_SPR_TYPE, SPRCTL0_VFLIP, SPRCTL1, SPRCTL1_RELOAD_HVS, SPRCTL1_RELOAD_HVST,
    SPRCTL1_REUSE_PALETTE, SPRCTL1_SKIP_SPRITE, SPRDLINEH, SPRDLINEL, SPRDOFFL, SPRGO,
    SPRGO_EVERON, SPRHSIZH, SPRHSIZL, SPRVSIZH, SPRVSIZL, STRETCHH, STRETCHL,
    SUZY_SPRITE_SCB_ADDITIONAL_COST, SUZY_SPRITE_VERT_ADDITIONAL_COST, TILTACUMH, TILTACUML, TILTH,
    TILTL, VIDADRL, VOFFL, VPOSSTRTH, VPOSSTRTL, VSIZACUMH, VSIZACUML, VSIZOFFL,
};

macro_rules! peek_dma {
    ($regs: ident, $ram: ident, $addr: expr) => {{
        $regs.set_task_ticks_delay(RAM_PAGE_READ_TICKS as u16);
        $ram.get($addr)
    }};
}

macro_rules! peek_scb_header {
    ($slf: ident, $regs: ident, $ram: ident) => {{
        let data = peek_dma!($regs, $ram, $regs.tmp_addr());
        $regs.set_tmp_addr($regs.tmp_addr().wrapping_add(1));
        trace!(
            "  SCB Header Step {}: Read 0x{:02X} from 0x{:04X}",
            $slf.scb_step,
            data,
            $regs.tmp_addr()
        );
        $slf.scb_step += 1;
        data
    }};
}

macro_rules! peek_and_store_scb_data {
    ($slf: ident, $regs: ident, $ram: ident) => {
        let data = peek_dma!($regs, $ram, $regs.tmp_addr());
        $slf.sprite_data.push_data(data);
        $regs.set_tmp_addr($regs.tmp_addr().wrapping_add(1));
    };
}

macro_rules! peek_and_store_scb_header {
    ($slf: ident, $regs: ident, $ram: ident, $dest: expr) => {{
        let d = peek_scb_header!($slf, $regs, $ram);
        $regs.set_data($dest, d);
    }};
}

#[derive(Serialize, Deserialize)]
pub struct Renderer {
    scb_step: u8,
    scb_pen_idx: usize,
    start_quadrant: u8,
    quadrant: u8,
    ever_on_screen: bool,
    orig_vsign: i16,
    orig_hsign: i16,
    hquadoff: i16,
    vquadoff: i16,
    hsize_accumulator: u16,
    screen_h_start: i16,
    screen_v_start: i16,
    voff: i16,
    hoff: i16,
    vsign: i16,
    hsign: i16,
    pixel_height: u8,
    orig_pixel_height: u8,
    sprite_data: SpriteData,
    pixel: u8,
    pixel_width: u8,
    collision: u8,
    pens: [u8; 16],
}

impl Renderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scb_step: 0,
            scb_pen_idx: 0,
            start_quadrant: 0,
            quadrant: 0,
            ever_on_screen: false,
            orig_vsign: 0,
            orig_hsign: 0,
            vsign: 0,
            hsign: 0,
            hquadoff: 0,
            vquadoff: 0,
            hsize_accumulator: 0,
            screen_h_start: 0,
            screen_v_start: 0,
            voff: 0,
            hoff: 0,
            pixel_height: 0,
            orig_pixel_height: 0,
            sprite_data: SpriteData::new(),
            pixel: 0,
            pixel_width: 0,
            collision: 0,
            pens: [0; 16],
        }
    }

    fn initialize_for_painting(&mut self, regs: &mut SuzyRegisters) {
        regs.sprsys_r_enable_flag(SprSysR::sprite_working);
        regs.sprsys_r_enable_flag(SprSysR::math_working);

        let firstscb = regs.sbc_next();
        regs.set_scb_addr(firstscb);

        trace!(
            "Starting sprite rendering. Renders 0x{:04X} to 0x{:04X}",
            firstscb,
            regs.u16(VIDADRL)
        );

        if 0 == (firstscb & 0xFF00) {
            regs.set_task_step(TaskStep::MaxSteps);
            return;
        }

        self.scb_step = 0;

        let scbaddr = regs.scb_addr();
        if 0 == (scbaddr & 0xFF00) {
            trace!("Stop current sprite.");
            self.stop_sprite_engine(regs);
        } else {
            regs.set_tmp_addr(scbaddr);
            regs.set_task_ticks_delay(SUZY_SPRITE_SCB_ADDITIONAL_COST);
            regs.inc_task_step();
        }
    }

    fn stop_sprite_engine(&mut self, regs: &mut SuzyRegisters) {
        self.scb_step = 0;
        regs.sprsys_r_disable_flag(SprSysR::sprite_working);
        regs.sprsys_r_disable_flag(SprSysR::math_working);
        regs.reset_task();
    }

    fn load_scb(&mut self, ram: &mut Ram, regs: &mut SuzyRegisters) {
        let mut end_scb: bool = false;

        match self.scb_step {
            0 => peek_and_store_scb_header!(self, regs, ram, SPRCTL0),
            1 => peek_and_store_scb_header!(self, regs, ram, SPRCTL1),
            2 => peek_and_store_scb_header!(self, regs, ram, SPRCOLL),
            3 => peek_and_store_scb_header!(self, regs, ram, SCBNEXTL),
            4 => {
                peek_and_store_scb_header!(self, regs, ram, SCBNEXTH);
                if regs.sprctl1() & SPRCTL1_SKIP_SPRITE != 0 {
                    trace!("Sprite skipped.");
                    self.scb_step = 0;
                    regs.set_task_step(TaskStep::InitializePainting); // next scb if any
                }
            }
            5 => peek_and_store_scb_header!(self, regs, ram, SPRDLINEL),
            6 => peek_and_store_scb_header!(self, regs, ram, SPRDLINEH),
            7 => peek_and_store_scb_header!(self, regs, ram, HPOSSTRTL),
            8 => peek_and_store_scb_header!(self, regs, ram, HPOSSTRTH),
            9 => peek_and_store_scb_header!(self, regs, ram, VPOSSTRTL),
            10 => {
                peek_and_store_scb_header!(self, regs, ram, VPOSSTRTH);
                if regs.sprctl1() & SPRCTL1_RELOAD_HVST == 0 {
                    if regs.sprctl1() & SPRCTL1_REUSE_PALETTE == SPRCTL1_REUSE_PALETTE {
                        end_scb = true;
                    } else {
                        self.scb_step = 19;
                    }
                }
            }
            11 => peek_and_store_scb_header!(self, regs, ram, SPRHSIZL),
            12 => peek_and_store_scb_header!(self, regs, ram, SPRHSIZH),
            13 => peek_and_store_scb_header!(self, regs, ram, SPRVSIZL),
            14 => {
                peek_and_store_scb_header!(self, regs, ram, SPRVSIZH);
                if regs.sprctl1() & SPRCTL1_RELOAD_HVS != SPRCTL1_RELOAD_HVS {
                    if regs.sprctl1() & SPRCTL1_REUSE_PALETTE == SPRCTL1_REUSE_PALETTE {
                        end_scb = true;
                    } else {
                        self.scb_step = 19;
                    }
                }
            }
            15 => peek_and_store_scb_header!(self, regs, ram, STRETCHL),
            16 => {
                peek_and_store_scb_header!(self, regs, ram, STRETCHH);
                if regs.sprctl1() & SPRCTL1_RELOAD_HVST != SPRCTL1_RELOAD_HVST {
                    if regs.sprctl1() & SPRCTL1_REUSE_PALETTE == SPRCTL1_REUSE_PALETTE {
                        end_scb = true;
                    } else {
                        self.scb_step = 19;
                    }
                }
            }
            17 => peek_and_store_scb_header!(self, regs, ram, TILTL),
            18 => {
                peek_and_store_scb_header!(self, regs, ram, TILTH);
                if regs.sprctl1() & SPRCTL1_REUSE_PALETTE == SPRCTL1_REUSE_PALETTE {
                    end_scb = true;
                }
            }
            19..27 => {
                let data = peek_scb_header!(self, regs, ram);
                self.pens[self.scb_pen_idx] = data >> 4;
                self.pens[self.scb_pen_idx + 1] = data & 0xf;
                self.scb_pen_idx += 2;
                if self.scb_pen_idx >= 16 {
                    end_scb = true;
                }
            }
            _ => unreachable!("Should not be happening"),
        }

        if end_scb {
            regs.inc_task_step();
            self.scb_step = 0;
            self.scb_pen_idx = 0;
            trace!("End Load SCB.");
        }
    }

    fn initialize_quadrants_rendering(&mut self, regs: &mut SuzyRegisters) {
        trace!("> initialize_quadrants_rendering. Sprite: CTL0:{:08b} CTL1:{:08b} COLL:{:08b} NEXT:{:04X} LINE:{:04X} HPOS:{:04X} VPOS:{:04X} HSIZE:{:04X} VSIZE:{:04X} STRETCH:{:04X} TITLT:{:04X} PENS:{:?}", 
            regs.sprctl0(), regs.sprctl1(), regs.sprcoll(),
            regs.sbc_next(), regs.sprdline(),
            regs.u16(HPOSSTRTL), regs.u16(VPOSSTRTL),
            regs.u16(SPRHSIZL), regs.u16(SPRVSIZL),
            regs.u16(STRETCHL), regs.u16(TILTL),
            self.pens
        );
        self.ever_on_screen = false;
        self.collision = 0;
        self.start_quadrant = regs.start_quadrant();
        self.quadrant = self.start_quadrant;
        self.hoff = regs.i16(HOFFL);
        self.voff = regs.i16(VOFFL);
        self.screen_h_start = self.hoff;
        self.screen_v_start = self.voff;
        self.orig_hsign = if self.start_quadrant == 0 || self.start_quadrant == 1 {
            1
        } else {
            -1
        };
        self.orig_vsign = if self.start_quadrant == 0 || self.start_quadrant == 3 {
            1
        } else {
            -1
        };

        regs.inc_task_step();
    }

    fn initialize_quadrant_render(&mut self, regs: &mut SuzyRegisters) {
        self.hsign = if self.quadrant == 0 || self.quadrant == 1 {
            1
        } else {
            -1
        };
        self.vsign = if self.quadrant == 0 || self.quadrant == 3 {
            1
        } else {
            -1
        };

        if regs.sprctl0() & SPRCTL0_VFLIP == SPRCTL0_VFLIP {
            self.vsign = -self.vsign;
        }

        if regs.sprctl0() & SPRCTL0_HFLIP == SPRCTL0_HFLIP {
            self.hsign = -self.hsign;
        }

        self.voff = regs.i16(VPOSSTRTL) - self.screen_v_start;

        regs.set_u16(TILTACUML, 0);

        if self.vsign == 1 {
            regs.set_u16(VSIZACUML, regs.u16(VSIZOFFL));
        } else {
            regs.set_u16(VSIZACUML, 0);
        }

        if self.quadrant == self.start_quadrant {
            self.vquadoff = self.vsign;
        }
        if self.vsign != self.vquadoff {
            self.voff += self.vsign;
        }

        self.pixel_height = 0;
        self.sprite_data.reset(regs);
        regs.inc_task_step();
    }

    fn end_quadrant_render(&mut self, regs: &mut SuzyRegisters) {
        trace!("< end_quadrant_render {}.", self.quadrant);
        if regs.u16(SPRDOFFL) == 0 {
            regs.inc_task_step();
        } else {
            self.quadrant += 1;
            self.quadrant &= 0x03;

            if self.quadrant == self.start_quadrant {
                regs.inc_task_step();
            } else {
                regs.set_task_step(TaskStep::InitializeQuadrant);
            }
        }
    }

    pub fn sprite_end(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram) -> u16 {
        let mut mem_count = 0;
        if regs.sprcoll() & SPRCOLL_DONT_COLLIDE == 0
            && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
        {
            match regs.sprctl0() & SPRCTL0_SPR_TYPE {
                2 | 3 | 4 | 6 | 7 => {
                    let coladr = regs.scb_addr().wrapping_add(regs.u16(COLLOFFL));
                    ram.set(coladr, self.collision);
                    mem_count += 1;
                    trace!("set collision 0x{:04X}=0x{:02X}", coladr, self.collision);
                }
                _ => (),
            }
        }

        if regs.data(SPRGO) & SPRGO_EVERON != 0 {
            let coladr = regs.scb_addr().wrapping_add(regs.u16(COLLOFFL));
            let mut coldat = ram.get(coladr);
            if self.ever_on_screen {
                coldat &= 0x7f;
            } else {
                coldat |= 0x80;
            }
            ram.set(coladr, coldat);
        }

        if regs.sprsys_w_is_flag_set(SprSysW::sprite_to_stop) {
            regs.inc_task_step();
        } else {
            regs.set_task_step(TaskStep::InitializePainting); // next scb if any
        }
        mem_count
    }

    fn render_lines_start(&mut self, ram: &mut Ram, regs: &mut SuzyRegisters) {
        trace!("> render_lines_start.");
        match self.sprite_data.initialize(regs, 0) {
            Result::Err(_e) => {
                peek_and_store_scb_data!(self, regs, ram);
                return;
            }
            Result::Ok(v) => {
                regs.set_u16(SPRDOFFL, v);
            }
        }

        regs.set_u16(
            VSIZACUML,
            regs.u16(VSIZACUML).wrapping_add(regs.u16(SPRVSIZL)),
        );

        self.orig_pixel_height = regs.data(VSIZACUMH);
        trace!("- render_lines_start. height:{}", self.orig_pixel_height);
        self.pixel_height = 0;

        regs.set_data(VSIZACUMH, 0);

        if 1 == regs.u16(SPRDOFFL) {
            regs.set_u16(SPRDLINEL, regs.sprdline().wrapping_add(1));
            regs.set_task_step(TaskStep::NextQuadrant);
            return;
        }

        if 0 == regs.u16(SPRDOFFL) {
            regs.set_task_step(TaskStep::SpriteEnd);
            return;
        }

        self.sprite_data.reset(regs);
        regs.inc_task_step();
    }

    fn render_lines_end(&mut self, regs: &mut SuzyRegisters) {
        trace!("< render_lines_end.");
        regs.set_u16(SPRDLINEL, regs.sprdline().wrapping_add(regs.u16(SPRDOFFL)));

        /* "
        The vertical size of a sprite can be modified every time a scan line is processed.
        This allows for 'stretching' a sprite vertically. The vertical stretch factor is the same as the horizontal stretch factor.
        " */
        if regs.sprsys_r_is_flag_set(SprSysR::v_stretching) {
            let size = regs.i16(SPRVSIZL);
            let stretch = if regs.sprctl1() & SPRCTL1_RELOAD_HVS == SPRCTL1_RELOAD_HVS {
                regs.i16(STRETCHL)
            } else {
                1
            };
            regs.set_i16(SPRVSIZL, size + stretch * i16::from(self.pixel_height));
        }

        self.sprite_data.reset(regs);
        regs.set_task_step(TaskStep::RenderLinesStart);
    }

    fn render_pixel_height_start(&mut self, ram: &mut Ram, regs: &mut SuzyRegisters) {
        trace!("> render_pixel_height_start.");

        if (self.vsign > 0 && self.voff >= LYNX_SCREEN_HEIGHT as i16)
            || (self.vsign < 0 && self.voff < 0)
            || self.orig_pixel_height == 0
        {
            regs.set_task_step(TaskStep::RenderLinesEnd);
            return;
        }

        if let Result::Err(_e) = self.sprite_data.initialize(regs, self.voff) {
            peek_and_store_scb_data!(self, regs, ram);
            return;
        }

        let mut hposstart = regs.i16(HPOSSTRTL);
        hposstart += regs.i16(TILTACUML) >> 8;
        regs.set_u16(HPOSSTRTL, hposstart as u16);

        regs.set_data(TILTACUMH, 0);

        self.hoff = regs.i16(HPOSSTRTL) - self.screen_h_start;

        self.hsize_accumulator = 0;
        if self.hsign > 0 {
            self.hsize_accumulator = regs.u16(HSIZOFFL);
        }

        if self.quadrant == self.start_quadrant {
            self.hquadoff = self.hsign;
        }
        if self.hsign != self.hquadoff {
            self.hoff += self.hsign;
        }

        regs.inc_task_step();
    }

    fn render_pixel_height_end(&mut self, ram: &mut Ram, regs: &mut SuzyRegisters) {
        trace!(
            "< render_pixel_height_end. pixel_height:{}",
            self.pixel_height
        );
        self.voff += self.vsign;

        let sprctl1 = regs.sprctl1();

        if sprctl1 & SPRCTL1_RELOAD_HVS == SPRCTL1_RELOAD_HVS {
            regs.set_u16(
                SPRHSIZL,
                regs.u16(SPRHSIZL).wrapping_add(regs.u16(STRETCHL)),
            );
        }

        if sprctl1 & SPRCTL1_RELOAD_HVST == SPRCTL1_RELOAD_HVST {
            regs.set_u16(TILTACUML, regs.u16(TILTACUML).wrapping_add(regs.u16(TILTL)));
        }

        self.pixel_height += 1;

        if self.pixel_height == self.orig_pixel_height {
            regs.inc_task_step();
        } else {
            self.sprite_data.reset(regs);
            peek_and_store_scb_data!(self, regs, ram);
            regs.add_task_ticks_delay(SUZY_SPRITE_VERT_ADDITIONAL_COST);
            regs.set_task_step(TaskStep::RenderPixelHeightStart);
        }
    }

    fn render_pixels_in_line(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram) {
        trace!("- render_pixels_in_line.");

        if self.voff < 0 || self.voff >= LYNX_SCREEN_HEIGHT as i16 {
            regs.inc_task_step();
            return;
        }

        let mut mem_access_count: u16 = 0;

        match self.sprite_data.line_get_pixel(regs, &self.pens) {
            Result::Err(_e) => {
                peek_and_store_scb_data!(self, regs, ram);
                return;
            }
            Result::Ok(v) => self.pixel = v,
        }

        if self.pixel == LINE_END {
            regs.inc_task_step();
            return;
        }

        self.hsize_accumulator = self.hsize_accumulator.wrapping_add(regs.u16(SPRHSIZL));
        self.pixel_width = (self.hsize_accumulator >> 8) as u8;
        self.hsize_accumulator &= 0xff;

        for _ in 0..self.pixel_width {
            if self.hoff >= 0 && self.hoff < LYNX_SCREEN_WIDTH as i16 {
                self.ever_on_screen = true;
                mem_access_count += self.process_pixel(regs, ram);
                trace!("- RenderPixel. width:{}", self.pixel_width);
            }
            self.hoff = self.hoff.wrapping_add(self.hsign);
        }

        regs.set_task_ticks_delay(mem_access_count);
    }

    fn write_pixel(&mut self, regs: &SuzyRegisters, ram: &mut Ram, pixel: u8) -> u16 {
        let scr_addr: u16 = regs.u16(VIDADRL) + (self.hoff as u16 / 2);

        let mut dest: u8 = ram.get(scr_addr);

        if self.hoff & 0x01 == 0 {
            dest &= 0x0f;
            dest |= pixel << 4;
        } else {
            dest &= 0xf0;
            dest |= pixel;
        }
        ram.set(scr_addr, dest);
        trace!(
            "write_pixel({}, {}) 0x{:04x} = 0x{:02x}",
            self.hoff,
            pixel,
            scr_addr,
            dest
        );

        2
    }

    fn read_pixel(&mut self, regs: &SuzyRegisters, ram: &mut Ram) -> (u8, u16) {
        let scr_addr: u16 = regs.u16(VIDADRL) + (self.hoff as u16 / 2);

        let mut data: u8 = ram.get(scr_addr);

        if self.hoff & 0x01 == 0 {
            data >>= 4;
        } else {
            data &= 0x0f;
        }

        (data, 1)
    }

    fn write_collision(&mut self, regs: &SuzyRegisters, ram: &mut Ram, pixel: u8) -> u16 {
        let col_addr = regs.u16(COLLADRL) + (self.hoff as u16 / 2);

        let mut dest: u8 = ram.get(col_addr);

        if self.hoff & 0x01 == 0 {
            dest &= 0x0f;
            dest |= pixel << 4;
        } else {
            dest &= 0xf0;
            dest |= pixel;
        }
        ram.set(col_addr, dest);
        trace!("Write collision pixel 0x{col_addr:04x} = 0x{dest:02x}");
        2
    }

    fn read_collision(&mut self, regs: &SuzyRegisters, ram: &mut Ram) -> (u8, u16) {
        let col_addr: u16 = regs.u16(COLLADRL) + (self.hoff as u16 / 2);

        let mut data: u8 = ram.get(col_addr);

        if self.hoff & 0x01 == 0 {
            data >>= 4;
        } else {
            data &= 0x0f;
        }

        (data, 1)
    }

    fn process_sprite_type(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprite_type: u8,
        sprcoll: u8,
    ) -> u16 {
        let mut mem_accesses = 0;
        match sprite_type {
            // 0 - BACKGROUND SHADOW
            0 => self.process_background_shadow(regs, ram, sprcoll, &mut mem_accesses),
            // 1 - BACKGROUND NOCOLLIDE
            1 => mem_accesses += self.write_pixel(regs, ram, self.pixel),
            // 2 - BOUNDARY_SHADOW
            2 => self.process_boundary_shadow(regs, ram, sprcoll, &mut mem_accesses),
            // 3 - BOUNDARY
            3 => self.process_boundary_sprite(regs, ram, sprcoll, &mut mem_accesses),
            // 4 - NORMAL
            4 => self.process_normal_sprite(regs, ram, sprcoll, &mut mem_accesses),
            // 5 - NOCOLLIDE
            5 => self.process_nocollide(regs, ram, &mut mem_accesses),
            // 6 - XOR SHADOW
            6 => self.process_xor_shadow_sprite(regs, ram, sprcoll, &mut mem_accesses),
            // 7 - SHADOW
            7 => self.process_shadow_sprite(regs, ram, sprcoll, &mut mem_accesses),
            _ => (),
        }
        mem_accesses
    }

    fn process_nocollide(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        mem_accesses: &mut u16,
    ) {
        if self.pixel != 0x00 {
            *mem_accesses += self.write_pixel(regs, ram, self.pixel);
        }
    }

    fn process_boundary_shadow(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        mem_accesses: &mut u16,
    ) {
        if self.pixel != 0x00 && self.pixel != 0x0e && self.pixel != 0x0f {
            *mem_accesses += self.write_pixel(regs, ram, self.pixel);
        }
        if self.pixel != 0x00
            && self.pixel != 0x0e
            && sprcoll & SPRCOLL_DONT_COLLIDE == 0
            && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
        {
            let (c, m) = self.read_collision(regs, ram);
            *mem_accesses += m;
            if c > self.collision {
                self.collision = c;
            }
            *mem_accesses += self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
        }
    }

    fn process_background_shadow(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        mem_accesses: &mut u16,
    ) {
        *mem_accesses += self.write_pixel(regs, ram, self.pixel);
        if sprcoll & SPRCOLL_DONT_COLLIDE == 0
            && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
            && self.pixel != 0x0e
        {
            *mem_accesses += self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
        }
    }

    fn process_boundary_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        mem_accesses: &mut u16,
    ) {
        if self.pixel != 0x00 && self.pixel != 0x0f {
            *mem_accesses += self.write_pixel(regs, ram, self.pixel);
        }
        if self.pixel != 0x00
            && sprcoll & SPRCOLL_DONT_COLLIDE == 0
            && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
        {
            let (c, m) = self.read_collision(regs, ram);
            *mem_accesses += m;
            if c > self.collision {
                self.collision = c;
            }
            *mem_accesses += self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
        }
    }

    fn process_normal_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        mem_accesses: &mut u16,
    ) {
        if self.pixel != 0x00 {
            *mem_accesses += self.write_pixel(regs, ram, self.pixel);
            if sprcoll & SPRCOLL_DONT_COLLIDE == 0
                && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
            {
                let (c, m) = self.read_collision(regs, ram);
                *mem_accesses += m;
                if c > self.collision {
                    self.collision = c;
                }
                *mem_accesses += self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
            }
        }
    }

    fn process_xor_shadow_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        mem_accesses: &mut u16,
    ) {
        if self.pixel != 0x00 {
            let (p, m) = self.read_pixel(regs, ram);
            *mem_accesses += m;
            *mem_accesses += self.write_pixel(regs, ram, p ^ self.pixel);
        }
        if self.pixel != 0x00
            && self.pixel != 0x0e
            && sprcoll & SPRCOLL_DONT_COLLIDE == 0
            && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
        {
            let (c, m) = self.read_collision(regs, ram);
            *mem_accesses += m;
            if c > self.collision {
                self.collision = c;
            }
            *mem_accesses += self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
        }
    }

    fn process_shadow_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        mem_accesses: &mut u16,
    ) {
        if self.pixel != 0x00 {
            *mem_accesses += self.write_pixel(regs, ram, self.pixel);
        }
        if self.pixel != 0x00
            && self.pixel != 0x0e
            && sprcoll & SPRCOLL_DONT_COLLIDE == 0
            && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
        {
            let (c, m) = self.read_collision(regs, ram);
            *mem_accesses += m;
            if c > self.collision {
                self.collision = c;
            }
            *mem_accesses += self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
        }
    }

    pub fn process_pixel(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram) -> u16 {
        trace!(
            "process_pixel() 0x{:04x} 0x{:02x} type:{}",
            self.hoff,
            self.pixel,
            regs.sprctl0() & SPRCTL0_SPR_TYPE
        );

        let sprcoll = regs.sprcoll();
        let sprite_type = regs.sprctl0() & SPRCTL0_SPR_TYPE;
        self.process_sprite_type(regs, ram, sprite_type, sprcoll)
    }

    pub fn render_sprites(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram) -> bool {
        match regs.task_step() {
            TaskStep::None => (),
            TaskStep::InitializePainting => self.initialize_for_painting(regs),
            TaskStep::LoadSCB => self.load_scb(ram, regs),
            TaskStep::InitializeQuadrants => self.initialize_quadrants_rendering(regs),
            TaskStep::InitializeQuadrant => self.initialize_quadrant_render(regs),
            TaskStep::RenderLinesStart => self.render_lines_start(ram, regs),
            TaskStep::RenderPixelHeightStart => self.render_pixel_height_start(ram, regs),
            TaskStep::RenderPixelsInLine => self.render_pixels_in_line(regs, ram),
            TaskStep::RenderPixelheightEnd => self.render_pixel_height_end(ram, regs),
            TaskStep::RenderLinesEnd => self.render_lines_end(regs),
            TaskStep::NextQuadrant => self.end_quadrant_render(regs),
            TaskStep::SpriteEnd => regs.set_task(SuzyTask::EndSprite),
            TaskStep::MaxSteps => self.stop_sprite_engine(regs),
        }
        regs.task() == SuzyTask::None
    }

    pub fn push_sprite_data(&mut self, data: u8) {
        self.sprite_data.push_data(data);
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new()
    }
}
