use std::fmt;

use log::trace;
use mikey::video::{LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH};
use suzy::*;
use sprite_data::SpriteData;
use crate::*;

#[derive(Serialize, Deserialize)]
pub struct Renderer {
    data: [u8; 32],
    in_idx: u16,
    store_idx: u16,
    start_quadrant: u8,
    quadrant: u8,
    ever_on_screen: bool,
    orig_vsign: i16,
    orig_hsign: i16,
    hquadoff: i16,
    vquadoff: i16,
    screen_h_start: i16,
    screen_v_start: i16,
    voff: i16,
    hoff: i16,
    vsign: i16,
    hsign: i16,
    pixel_height: u8,
    orig_pixel_height: u8,
    sprite_data: SpriteData,
    pixel: u32,
    pixel_width: u8,
    onscreen: bool,
    collision: u8,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            data: [0; 32],       
            in_idx: 0,
            store_idx: 0,
            start_quadrant: 0,
            quadrant: 0,
            ever_on_screen: false,
            orig_vsign: 0,
            orig_hsign: 0,
            vsign: 0,
            hsign: 0,
            hquadoff: 0,
            vquadoff: 0,
            screen_h_start: 0,
            screen_v_start: 0,
            voff: 0,
            hoff: 0,
            pixel_height: 0,
            orig_pixel_height: 0,
            sprite_data: SpriteData::new(),
            pixel: 0,
            pixel_width: 0,
            onscreen: false,
            collision: 0,
        }
    }

    pub fn data(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn set_data(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

    pub fn u16(&self, addr: u16) -> u16 {
        self.data(addr) as u16 | ((self.data(addr+1) as u16) << 8)
    }

    pub fn i16(&self, addr: u16) -> i16 {
        (self.data(addr) as u16 | ((self.data(addr+1) as u16) << 8)) as i16
    }

    pub fn scb_next(&self) -> u16 {
        self.u16(R_SCBNEXTL)
    }

    pub fn scb_sprdata(&self) -> u16 {
        self.u16(R_SPRDATAL)
    }

    pub fn scb_sprctl0(&self) -> u8 {
        self.data[R_SPRCTL0 as usize]
    }

    pub fn scb_sprctl1(&self) -> u8 {
        self.data[R_SPRCTL1 as usize]
    }

    pub fn scb_sprcoll(&self) -> u8 {
        self.data[R_SPRCOLL as usize]
    }

    pub fn scb_hpos(&self) -> i16 {
        self.i16(R_HPOSL)
    }

    pub fn scb_vpos(&self) -> i16 {
        self.i16(R_VPOSL)
    }

    pub fn scb_hsize(&self) -> i16 {
        self.i16(R_HSIZEL)
    }

    pub fn scb_vsize(&self) -> i16 {
        self.i16(R_VSIZEL)
    }

    pub fn scb_stretch(&self) -> i16 {
        self.i16(R_STRETCHL)
    }

    pub fn scb_tilt(&self) -> i16 {
        self.i16(R_TILTL)
    }

    pub fn push_scb_data(&mut self, v: u8) {
        trace!("Push SCB data idx:{} data:0x{:02x}", self.store_idx, v);
        self.data[self.store_idx as usize] = v;
        self.in_idx += 1;
        self.store_idx += 1;
    }

    fn initialize_for_painting(&mut self, regs: &mut SuzyRegisters) {
        trace!("Starting sprite rendering. Renders to 0x{:04X}", regs.u16(VIDADRL));
        regs.sprsys_r_enable_flag(SprSysR::sprite_working);
        regs.sprsys_r_enable_flag(SprSysR::math_working);

        let firstscb = regs.sbc_next();
        regs.set_scb_addr(firstscb);
        
        if 0 == firstscb {
            regs.set_task_step(TaskStep::MaxSteps);
            return;
        }
        
        self.store_idx = 0;
        self.in_idx = 0;
        regs.inc_task_step();
    }

    fn stop_sprite_engine(&mut self, regs: &mut SuzyRegisters) {
        self.in_idx = 0;
        self.store_idx = 0;
        regs.sprsys_r_disable_flag(SprSysR::sprite_working);
        regs.sprsys_r_disable_flag(SprSysR::math_working);
        regs.reset_task();
    }

    fn load_scb(&mut self, regs: &mut SuzyRegisters) {
        trace!("Load SCB. in: {} store: {}", self.in_idx, self.store_idx);
        match self.in_idx {
            R_SPRCTL0 => { 
                let scbaddr = regs.scb_addr();
                self.sprite_data.set_addr(scbaddr);
                if 0 == scbaddr {
                    trace!("Stop current sprite.");
                    self.stop_sprite_engine(regs);
                    return;
                }
                regs.scb_peek_ram();
            },
            R_SPRCTL1..=R_SCBNEXTH   => regs.scb_peek_ram(),
            R_SPRDATAL       => { 
                regs.set_data(SPRCTL0, self.data[R_SPRCTL0 as usize]);
                regs.set_data(SPRCTL1, self.data[R_SPRCTL1 as usize]);
                regs.set_data(SPRCOLL, self.data[R_SPRCOLL as usize]);
                regs.set_data(SCBNEXTL, self.data[R_SCBNEXTL as usize]);
                regs.set_data(SCBNEXTH, self.data[R_SCBNEXTH as usize]);

                if regs.sprctl1() & SPRCTL1_SKIP_SPRITE != 0 {
                    trace!("Sprite skipped.");
                    self.in_idx = 0;
                    self.store_idx = 0;
                    regs.set_task_step(TaskStep::InitializePainting); // next scb if any
                }
                else {
                    regs.scb_peek_ram();
                }
            },
            R_SPRDATAH..=R_VPOSH   => regs.scb_peek_ram(),
            R_HSIZEL => {
                regs.set_data(SPRDLINEL, self.data[R_SPRDATAL as usize]);
                regs.set_data(SPRDLINEH, self.data[R_SPRDATAH as usize]);
                regs.set_data(HPOSSTRTL, self.data[R_HPOSL as usize]);
                regs.set_data(HPOSSTRTH, self.data[R_HPOSH as usize]);
                regs.set_data(VPOSSTRTL, self.data[R_VPOSL as usize]);
                regs.set_data(VPOSSTRTH, self.data[R_VPOSH as usize]);

                let sprctl1 = regs.sprctl1();
                if sprctl1 & SPRCTL1_RELOAD_HVST == SPRCTL1_RELOAD_HVST {
                    regs.scb_peek_ram(); 
                }
                else if sprctl1 & SPRCTL1_RELOAD_HVS == SPRCTL1_RELOAD_HVS {                 
                    self.in_idx += 2;
                    regs.scb_peek_ram();
                    
                }
                else if sprctl1 & SPRCTL1_RELOAD_HV == SPRCTL1_RELOAD_HV {
                    self.in_idx += 4;
                    regs.scb_peek_ram();
                }
                else {
                    self.in_idx += 8;
                }
            }
            R_HSIZEH..=R_TILTH => regs.scb_peek_ram(),
            R_PALETTE_00 => {
                let sprctl1 = regs.sprctl1();
                if sprctl1 & SPRCTL1_RELOAD_HVST == SPRCTL1_RELOAD_HVST {
                    regs.set_data(SPRHSIZL, self.data[R_HSIZEL as usize]);
                    regs.set_data(SPRHSIZH, self.data[R_HSIZEH as usize]);
                    regs.set_data(SPRVSIZL, self.data[R_VSIZEL as usize]);
                    regs.set_data(SPRVSIZH, self.data[R_VSIZEH as usize]);
                    regs.set_data(STRETCHL, self.data[R_STRETCHL as usize]);
                    regs.set_data(STRETCHH, self.data[R_STRETCHH as usize]);
                    regs.set_data(TILTL,    self.data[R_TILTL as usize]);
                    regs.set_data(TILTH,    self.data[R_TILTH as usize]);                    
                } else if sprctl1 & SPRCTL1_RELOAD_HVS == SPRCTL1_RELOAD_HVS {
                    regs.set_data(SPRHSIZL, self.data[R_HSIZEL as usize]);
                    regs.set_data(SPRHSIZH, self.data[R_HSIZEH as usize]);
                    regs.set_data(SPRVSIZL, self.data[R_VSIZEL as usize]);
                    regs.set_data(SPRVSIZH, self.data[R_VSIZEH as usize]);
                    regs.set_data(STRETCHL, self.data[R_STRETCHL as usize]);
                    regs.set_data(STRETCHH, self.data[R_STRETCHH as usize]);
                    
                } else if sprctl1 & SPRCTL1_RELOAD_HV == SPRCTL1_RELOAD_HV {
                    regs.set_data(SPRHSIZL, self.data[R_HSIZEL as usize]);
                    regs.set_data(SPRHSIZH, self.data[R_HSIZEH as usize]);
                    regs.set_data(SPRVSIZL, self.data[R_VSIZEL as usize]);
                    regs.set_data(SPRVSIZH, self.data[R_VSIZEH as usize]);
                }

                if sprctl1 & SPRCTL1_REUSE_PALETTE == SPRCTL1_REUSE_PALETTE {
                    trace!("End current sprite.");
                    self.in_idx = 0;
                    self.store_idx = 0;
                    regs.inc_task_step(); 
                } else {
                    self.store_idx = R_PALETTE_00;
                    regs.scb_peek_ram();
                }       
            }
            R_PALETTE_01..=R_PALETTE_07 => regs.scb_peek_ram(),
            _ => { 
                regs.inc_task_step(); 
                self.in_idx = 0;
                self.store_idx = 0;
                trace!("Sprite SCB:\n{:?}", self); 
            }
        }
    }

    fn initialize_quadrants_rendering(&mut self, regs: &mut SuzyRegisters) {
        trace!("> initialize_quadrants_rendering.");
        self.ever_on_screen = false;
        self.collision = 0;
        self.sprite_data.set_addr(regs.sprdline());
        self.start_quadrant = regs.start_quadrant();
        self.quadrant = self.start_quadrant;
        regs.set_u16(TMPADRL, 0);
        self.hoff = regs.i16(HOFFL);
        self.voff = regs.i16(VOFFL);        
        self.screen_h_start = self.hoff;
        self.screen_v_start = self.voff;
        self.orig_hsign = if self.start_quadrant == 0 || self.start_quadrant == 1 {1} else {-1};
        self.orig_vsign = if self.start_quadrant == 0 || self.start_quadrant == 3 {1} else {-1};
        
        trace!("> initialize_quadrants_rendering. {:?}", self);
 
        regs.inc_task_step();
    }

    fn initialize_quadrant_render(&mut self, regs: &mut SuzyRegisters) {
        self.hsign = if self.quadrant == 0 || self.quadrant == 1 {1} else {-1};
        self.vsign = if self.quadrant == 0 || self.quadrant == 3 {1} else {-1};

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
        }
        else {
            self.quadrant += 1;
            self.quadrant &= 0x03;

            if self.quadrant == self.start_quadrant {
                regs.inc_task_step();
            } else {
                regs.set_task_step(TaskStep::InitializeQuadrant);
            }
        }
    }

    pub fn sprite_end(&mut self, regs: &mut SuzyRegisters, dma_ram: &mut Ram) -> u16 {
        let mut mem_count = 0;
        if regs.sprcoll() & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide) {
            match regs.sprctl0() & SPRCTL0_SPR_TYPE {
                2 | 3 | 4 | 6 | 7 => {
                    let coladr = regs.scb_addr().overflowing_add(regs.u16(COLLOFFL)).0;
                    dma_ram.set(coladr, self.collision);
                    mem_count += 1;
                    trace!("set collision 0x{:04X}=0x{:02X}", coladr, self.collision);
                }
                _ => (),
            }
        }

        if regs.data(SPRGO) & SPRGO_EVERON != 0 {
            let coladr = regs.scb_addr().overflowing_add(regs.u16(COLLOFFL)).0;
            let mut coldat = dma_ram.get(coladr);
            if !self.ever_on_screen {
                coldat |= 0x80;
            } else {
                coldat &= 0x7f;
            }
            dma_ram.set(coladr, coldat);
        }

        if regs.sprsys_w_is_flag_set(SprSysW::sprite_to_stop) {
            regs.inc_task_step();
        } else {
            regs.set_task_step(TaskStep::InitializePainting); // next scb if any
        }
        mem_count
    }

    fn render_lines_start(&mut self, regs: &mut SuzyRegisters) {
        trace!("> render_lines_start.");
        match self.sprite_data.initialize(regs, 0) {
            Result::Err(_e) => { 
                regs.scb_peek_sprite_data();
                return;
            }
            Result::Ok(v) => {
                regs.set_u16(SPRDOFFL, v);
            }
        }

        regs.set_u16(VSIZACUML, regs.u16(VSIZACUML).overflowing_add(regs.u16(SPRVSIZL)).0);

        self.orig_pixel_height = regs.data(VSIZACUMH);
        self.pixel_height = 0;

        regs.set_data(VSIZACUMH, 0);

        if 1 == regs.u16(SPRDOFFL) {
            regs.set_u16(SPRDLINEL, regs.u16(SPRDLINEL).overflowing_add(regs.u16(SPRDOFFL)).0);
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
        regs.set_u16(SPRDLINEL, regs.u16(SPRDLINEL).overflowing_add(regs.u16(SPRDOFFL)).0);

        /* "
        The vertical size of a sprite can be modified every time a scan line is processed. 
        This allows for 'stretching' a sprite vertically. The vertical stretch factor is the same as the horizontal stretch factor. 
        " */
        if regs.sprsys_r_is_flag_set(SprSysR::v_stretching) {
            let size = regs.i16(SPRVSIZL);
            let stretch = regs.i16(STRETCHL);
            regs.set_i16(SPRVSIZL, size + stretch * self.pixel_height as i16);
        } 

        self.sprite_data.reset(regs);
        regs.set_task_step(TaskStep::RenderLinesStart);
    }

    fn render_pixel_height_start(&mut self, regs: &mut SuzyRegisters) {
        trace!("> render_pixel_height_start.");

        if (self.vsign > 0 && self.voff >= LYNX_SCREEN_HEIGHT as i16) || (self.vsign < 0 && self.voff < 0) || self.orig_pixel_height == 0 { 
            regs.set_task_step(TaskStep::RenderLinesEnd);
            return;
        } 

        if self.voff < 0 || self.voff >= LYNX_SCREEN_HEIGHT as i16 {
            regs.set_task_step(TaskStep::RenderPixelheightEnd);
            return;
        }

        if let Result::Err(_e) = self.sprite_data.initialize(regs, self.voff) { 
            regs.scb_peek_sprite_data();
            return;
        }

        self.onscreen = false;

        let mut hposstart = regs.i16(HPOSSTRTL);
        hposstart += regs.i16(TILTACUML) >> 8 ;
        regs.set_u16(HPOSSTRTL, hposstart as u16);

        regs.set_data(TILTACUMH, 0);

        self.hoff = regs.i16(HPOSSTRTL) - self.screen_h_start;

        regs.set_u16(TMPADRL, 0);
        if self.hsign > 0 {
            regs.set_u16(TMPADRL, regs.u16(HSIZOFFL));
        }

        if self.quadrant == self.start_quadrant {
            self.hquadoff = self.hsign;
        }
        if self.hsign != self.hquadoff {
            self.hoff += self.hsign;
        }
        
        regs.inc_task_step();
    }

    fn render_pixel_height_end(&mut self, regs: &mut SuzyRegisters) {
        trace!("< render_pixel_height_end.");
        self.voff += self.vsign;
    
        let sprctl1 = regs.sprctl1();

        if sprctl1 & SPRCTL1_RELOAD_HVS == SPRCTL1_RELOAD_HVS {
            regs.set_u16(SPRHSIZL, regs.u16(SPRHSIZL).overflowing_add(regs.u16(STRETCHL)).0);
        } 

        if sprctl1 & SPRCTL1_RELOAD_HVST == SPRCTL1_RELOAD_HVST {
            regs.set_u16(TILTACUML, regs.u16(TILTACUML).overflowing_add(regs.u16(TILTL)).0);
        }

        self.pixel_height += 1;

        if self.pixel_height == self.orig_pixel_height {
            regs.inc_task_step();
        }
        else {
            self.sprite_data.reset(regs);
            regs.set_task_step(TaskStep::RenderPixelHeightStart); 
        }
    }

    fn render_pixels_in_line(&mut self, regs: &mut SuzyRegisters, dma_ram: &mut Ram) {
        trace!("- render_pixels_in_line.");
        self.pixel = 0;

        match self.sprite_data.line_get_pixel(regs, &self.data) {
            Result::Err(_e) => { 
                regs.scb_peek_sprite_data();
                return;
            }
            Result::Ok(v) => self.pixel = v,
        }

        if self.pixel == LINE_END {
            regs.inc_task_step();
            return;
        }

        regs.set_u16(TMPADRL, regs.u16(TMPADRL).overflowing_add(regs.u16(SPRHSIZL)).0);
        self.pixel_width = regs.data(TMPADRH);
        regs.set_data(TMPADRH, 0);

        if self.pixel_width == 0 {
            return;
        }

        for _ in 0..self.pixel_width {
            if self.hoff >= 0 && self.hoff < LYNX_SCREEN_WIDTH as i16 {
                self.onscreen = true;
                self.ever_on_screen = true;
                trace!("- RenderPixel.");    
                let mem_access_count = self.process_pixel(regs, dma_ram); 
                regs.set_task_ticks_delay(mem_access_count * RAM_DMA_READ_TICKS as u16);
            }
            else if self.onscreen {
                regs.inc_task_step();
                return;
            }
            self.hoff += self.hsign;
        }
    }
   
    fn write_pixel(&mut self, regs: &SuzyRegisters, dma_ram: &mut Ram, pixel: u32) -> u16 {
        let scr_addr : u16 = regs.u16(VIDADRL) + (self.hoff as u16 / 2);

        let mut dest: u8 = dma_ram.get(scr_addr);

        if self.hoff & 0x01 == 0 {
            dest &= 0x0f;
            dest |= (pixel as u8) << 4;
        } else {
            dest &= 0xf0;
            dest |= pixel as u8;
        }
        dma_ram.set(scr_addr, dest);
        trace!("write_pixel({}, {}) 0x{:04x} = 0x{:02x}", self.hoff, pixel, scr_addr, dest);

        2
    }

    fn read_pixel(&mut self, regs: &SuzyRegisters, dma_ram: &mut Ram) -> (u8, u16) {
        let scr_addr : u16 = regs.u16(VIDADRL) + (self.hoff as u16 / 2);

        let mut data: u8 = dma_ram.get(scr_addr);

        if self.hoff & 0x01 == 0 {
            data >>= 4;
        } else {
            data &= 0x0f;
        }

        (data, 1)
    }

    fn write_collision(&mut self, regs: &SuzyRegisters, dma_ram: &mut Ram, pixel: u8) -> u16 {
        let col_addr = regs.u16(COLLADRL) + (self.hoff as u16 / 2);

        let mut dest: u8 = dma_ram.get(col_addr);

        if self.hoff & 0x01 == 0 {
            dest &= 0x0f;
            dest |= pixel << 4;
        }
        else {
            dest &= 0xf0;
            dest |= pixel;
        }
        dma_ram.set(col_addr, dest);
        trace!("Write collision pixel 0x{:04x} = 0x{:02x}", col_addr, dest);
        2
    }

    fn read_collision(&mut self, regs: &SuzyRegisters, dma_ram: &mut Ram) -> (u8, u16) {
        let col_addr : u16 = regs.u16(COLLADRL) + (self.hoff as u16 / 2);

        let mut data: u8 = dma_ram.get(col_addr);

        if self.hoff & 0x01 == 0 {
            data >>= 4;
        } else {
            data &= 0x0f;
        }

        (data, 1)
    }

    pub fn process_pixel(&mut self, regs: &mut SuzyRegisters, dma_ram: &mut Ram) -> u16 {
        let mut mem_accesses : u16 = 0;

        trace!("process_pixel() 0x{:04x} 0x{:02x} type:{}", self.hoff, self.pixel, regs.sprctl0() & SPRCTL0_SPR_TYPE);

        let sprcoll = regs.sprcoll();

        match regs.sprctl0() & SPRCTL0_SPR_TYPE {
            // 0 - BACKGROUND SHADOW
            0 => { 
                mem_accesses += self.write_pixel(regs, dma_ram, self.pixel);
                if sprcoll & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide) && self.pixel != 0x0e {
                    mem_accesses += self.write_collision(regs, dma_ram, sprcoll & SPRCOLL_NUMBER);
                }
            }

            // 1 - BACKGROUND NOCOLLIDE
            1 => mem_accesses += self.write_pixel(regs, dma_ram, self.pixel),

            // 2 - BOUNDARY_SHADOW
            2 => {
                if self. pixel != 0x00 && self.pixel != 0x0e && self.pixel != 0x0f {
                    mem_accesses += self.write_pixel(regs, dma_ram, self.pixel);
                }

                if self.pixel != 0x00 && self.pixel != 0x0e && sprcoll & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide) {
                    let (c, m) = self.read_collision(regs, dma_ram);
                    mem_accesses += m;
                    if c > self.collision {
                        self.collision = c;
                    }
                    mem_accesses += self.write_collision(regs, dma_ram, sprcoll & SPRCOLL_NUMBER);
                }
            }

            // 3 - BOUNDARY
            3 => {
                if self.pixel != 0x00 && self.pixel != 0x0f {
                    mem_accesses += self.write_pixel(regs, dma_ram, self.pixel);
                }
                if self.pixel != 0x00 && sprcoll & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide) {
                    let (c, m) = self.read_collision(regs, dma_ram);
                    mem_accesses += m;
                    if c > self.collision {
                        self.collision = c;
                    }
                    mem_accesses += self.write_collision(regs, dma_ram, sprcoll & SPRCOLL_NUMBER);
                }
            }

            // 4 - NORMAL
            4 => {
                if self.pixel != 0x00 {
                    mem_accesses += self.write_pixel(regs, dma_ram, self.pixel);
                    if sprcoll & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide) {
                        let (c, m) = self.read_collision(regs, dma_ram);
                        mem_accesses += m;
                        if c > self.collision {
                            self.collision = c;
                        }
                        mem_accesses += self.write_collision(regs, dma_ram, sprcoll & SPRCOLL_NUMBER);
                    }
                }
            }

            // 5 - NOCOLLIDE
            5 => if self.pixel != 0x00 { mem_accesses += self.write_pixel(regs, dma_ram, self.pixel); },
            
            // 6 - XOR SHADOW
            6 => {
                if self.pixel != 0x00 {
                    let (p, m) = self.read_pixel(regs, dma_ram);
                    mem_accesses += m;
                    mem_accesses += self.write_pixel(regs, dma_ram, p as u32 ^ self.pixel);
                }
                if self.pixel != 0x00 && self.pixel != 0x0e && sprcoll & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide) {
                    let (c, m) = self.read_collision(regs, dma_ram);
                    mem_accesses += m;
                    if c > self.collision {
                        self.collision = c;
                    }
                    mem_accesses += self.write_collision(regs, dma_ram, sprcoll & SPRCOLL_NUMBER);
                }
            }

            // 7 - SHADOW
            7 => {
                if self.pixel != 0x00 {
                    mem_accesses += self.write_pixel(regs, dma_ram, self.pixel);
                }
                if self.pixel != 0x00 && self.pixel != 0x0e && sprcoll & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide) {
                    let (c, m) = self.read_collision(regs, dma_ram);
                    mem_accesses += m;
                    if c > self.collision {
                        self.collision = c;
                    }
                    mem_accesses += self.write_collision(regs, dma_ram, sprcoll & SPRCOLL_NUMBER);
                }
            }

            _ => (),    
        }
        mem_accesses
    }


    pub fn render_sprites(&mut self, regs: &mut SuzyRegisters, dma_ram: &mut Ram) -> bool {
        match regs.task_step() {
            TaskStep::None => (),
            TaskStep::InitializePainting      => self.initialize_for_painting(regs),
            TaskStep::LoadSCB                 => self.load_scb(regs),
            TaskStep::InitializeQuadrants     => self.initialize_quadrants_rendering(regs),
            TaskStep::InitializeQuadrant      => self.initialize_quadrant_render(regs),
            TaskStep::RenderLinesStart        => self.render_lines_start(regs),
            TaskStep::RenderPixelHeightStart  => self.render_pixel_height_start(regs),
            TaskStep::RenderPixelsInLine      => self.render_pixels_in_line(regs, dma_ram),
            TaskStep::RenderPixelheightEnd    => self.render_pixel_height_end(regs),
            TaskStep::RenderLinesEnd          => self.render_lines_end(regs),
            TaskStep::NextQuadrant            => self.end_quadrant_render(regs),
            TaskStep::SpriteEnd               => regs.set_task(SuzyTask::EndSprite),
            _                                 => self.stop_sprite_engine(regs),
        }
        regs.task() == SuzyTask::None
    }

    pub fn push_sprite_data(&mut self, data: &[u8]) {
        self.sprite_data.push_data(data);
    }

    pub fn scb_curr_adr(&self) -> u16 {
        self.sprite_data.addr()
    }

    pub fn inc_scb_curr_adr(&mut self) {
        self.sprite_data.set_addr(self.sprite_data.addr()+1);
    }
}

impl fmt::Debug for Renderer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, 
            "SPRCTL0:0x{:02x} SPRCTL1:0x{:02x} SPRCOLL:0x{:02x}\nSCBNEXT:0x{:04x} SPRDATA:0x{:04x}\nHPOS:0x{:04x} VPOS:0x{:04x}\nHSIZE:0x{:04x} VSIZE:0x{:04x}\nSTRETCH:0x{:04x} TILT:0x{:04x}",
            self.scb_sprctl0(), self.scb_sprctl1(), self.scb_sprcoll(),
            self.scb_next(), self.scb_sprdata(),
            self.scb_hpos(), self.scb_vpos(),
            self.scb_hsize(), self.scb_vsize(),
            self.scb_stretch(), self.scb_tilt(),
        )
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new()
    }
}