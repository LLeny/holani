use crate::{
    consts::{LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH},
    suzy,
};
use log::trace;
use row_pipeline::{
    RowPipeline, SUZY_ACTIVE_SCB_STARTUP_TICKS, SUZY_COLLISION_CLEAR_BURST_TICKS,
    SUZY_COLLISION_DETECT_BURST_TICKS, SUZY_COLLISION_MERGE_BURST_TICKS,
    SUZY_COLLISION_PIPELINE_GROUP_TICKS, SUZY_CONTROL_LINE_TICKS,
    SUZY_LINKED_SCB_CONTROL_OVERLAP_TICKS, SUZY_LITERAL_1BPP_HALF_PAIR_TICKS,
    SUZY_LITERAL_1BPP_RECORD_TICKS, SUZY_PIXEL_BUILDER_COMPLETE_END_TICKS,
    SUZY_PIXEL_BUILDER_EVEN_END_TICKS, SUZY_RAM_READ_TICKS, SUZY_REGULAR_CLIP_FIFO_TICKS,
    SUZY_VERTICAL_SKIP_TICKS, SUZY_VIDEO_MERGE_BURST_TICKS, SUZY_XOR_BYTE_TICKS,
};
use sprite_data::SpriteData;
use suzy::step_queue::StepKind;
use suzy::{
    row_pipeline, sprite_data, Deserialize, Ram, Serialize, SprSysR, SprSysW, SuzyRegisters,
    SuzyTask, TaskStep, COLLADRL, COLLOFFL, HOFFL, HPOSSTRTH, HPOSSTRTL, HSIZOFFL, LINE_END,
    SCBNEXTH, SCBNEXTL, SPRCOLL, SPRCOLL_DONT_COLLIDE, SPRCOLL_NUMBER, SPRCTL0, SPRCTL0_BPP,
    SPRCTL0_HFLIP, SPRCTL0_SPR_TYPE, SPRCTL0_VFLIP, SPRCTL1, SPRCTL1_LITERAL,
    SPRCTL1_RELOAD_HVS, SPRCTL1_RELOAD_HVST, SPRCTL1_REUSE_PALETTE, SPRCTL1_SKIP_SPRITE,
    SPRDLINEH, SPRDLINEL, SPRDOFFL, SPRGO, SPRGO_EVERON, SPRHSIZH, SPRHSIZL, SPRVSIZH, SPRVSIZL,
    STRETCHH, STRETCHL, TILTACUMH, TILTACUML, TILTH, TILTL, VIDADRL, VOFFL, VPOSSTRTH, VPOSSTRTL,
    VSIZACUMH, VSIZACUML, VSIZOFFL,
};

macro_rules! peek_dma {
    ($regs: ident, $ram: ident, $addr: expr) => {{
        $regs.set_task_steps(StepKind::ScbFetch, SUZY_RAM_READ_TICKS - 1);
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

macro_rules! peek_scb_palette {
    ($slf: ident, $regs: ident, $ram: ident) => {{
        let data = $ram.get($regs.tmp_addr());
        $regs.set_tmp_addr($regs.tmp_addr().wrapping_add(1));
        trace!(
            "  SCB Palette Step {}: Read 0x{:02X} from 0x{:04X}",
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
        let data = $ram.get($regs.tmp_addr());
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

fn pen_transparent(sprite_type: u8, pen: u8) -> bool {
    match sprite_type & 0x07 {
        0 | 1 => false,
        2 | 3 => pen == 0x00 || pen == 0x0f,
        4..=7 => pen == 0x00,
        _ => true,
    }
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
    pipeline: RowPipeline,
    row_render: bool,
    skip_row: bool,
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
            pipeline: RowPipeline::new(),
            row_render: false,
            skip_row: false,
        }
    }

    pub fn sprite_go_start(&mut self) {
        self.pipeline.sprite_go_start();
    }

    pub fn notify_bus_grant(&mut self) {
        self.pipeline.set_lcd_dma_granted();
    }

    fn drain_pipeline(&mut self, regs: &mut SuzyRegisters, absorb_step: bool) {
        let mut pending = self.pipeline.take_pending_steps();
        pending.drop_front_n(usize::from(absorb_step));
        regs.task_steps().append(&mut pending);
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
            if !self.pipeline.first_scb() {
                regs.push_task_steps(StepKind::ScbStartup, SUZY_ACTIVE_SCB_STARTUP_TICKS - 1);
            }
            self.pipeline.set_first_scb(false);
            regs.inc_task_step();
        }
    }

    fn stop_sprite_engine(&mut self, regs: &mut SuzyRegisters) {
        self.scb_step = 0;
        self.pipeline.blitter_done();
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
                    if self.pipeline.scb_control_line_pending() {
                        regs.push_task_steps(StepKind::ControlLine, SUZY_CONTROL_LINE_TICKS);
                        self.pipeline.set_scb_control_line_pending(false);
                    }
                    self.pipeline.set_row_pipeline_warm(false);
                    self.scb_step = 0;
                    regs.set_task_step(TaskStep::InitializePainting); // next scb if any
                } else if self.pipeline.scb_control_line_pending() {
                    regs.push_task_steps(
                        StepKind::ControlLine,
                        SUZY_CONTROL_LINE_TICKS - SUZY_LINKED_SCB_CONTROL_OVERLAP_TICKS,
                    );
                    self.pipeline.set_scb_control_line_pending(false);
                } else {
                    self.pipeline.set_row_pipeline_warm(false);
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
                let data = peek_scb_palette!(self, regs, ram);
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
        self.pipeline.set_sprite_row_started(false);

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

        let unflipped_down = self.quadrant == 0 || self.quadrant == 3;
        if unflipped_down {
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
            mem_count += 1;
            if self.ever_on_screen {
                coldat &= 0x7f;
            } else {
                coldat |= 0x80;
            }
            ram.set(coladr, coldat);
            mem_count += 1;
        }

        if regs.sprsys_w_is_flag_set(SprSysW::sprite_to_stop) {
            if self.pipeline.scb_control_line_pending() {
                regs.push_task_steps(StepKind::ControlLine, SUZY_CONTROL_LINE_TICKS);
                self.pipeline.set_scb_control_line_pending(false);
            }
            regs.inc_task_step();
        } else {
            self.pipeline.sprite_end_next_scb();
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

        let offset = regs.u16(SPRDOFFL);
        let literal_1bpp_record = offset > 1
            && self.pipeline.sprite_row_started()
            && regs.sprctl1() & SPRCTL1_LITERAL != 0
            && regs.sprctl0() & SPRCTL0_BPP == 0;
        regs.push_task_steps(
            StepKind::ScbFetch,
            if literal_1bpp_record {
                SUZY_LITERAL_1BPP_RECORD_TICKS - 1
            } else {
                SUZY_RAM_READ_TICKS - 1
            },
        );

        regs.set_u16(
            VSIZACUML,
            regs.u16(VSIZACUML).wrapping_add(regs.u16(SPRVSIZL)),
        );

        self.orig_pixel_height = regs.data(VSIZACUMH);
        trace!("- render_lines_start. height:{}", self.orig_pixel_height);
        self.pixel_height = 0;

        regs.set_data(VSIZACUMH, 0);

        if 1 == offset {
            regs.push_task_steps(StepKind::ControlLine, SUZY_CONTROL_LINE_TICKS);
            regs.set_u16(SPRDLINEL, regs.sprdline().wrapping_add(1));
            regs.set_task_step(TaskStep::NextQuadrant);
            return;
        }

        if 0 == offset {
            if regs.sbc_next() & 0xFF00 != 0 {
                self.pipeline.set_scb_control_line_pending(true);
                self.pipeline
                    .set_row_pipeline_warm(self.pipeline.sprite_row_started());
            } else {
                regs.push_task_steps(StepKind::ControlLine, SUZY_CONTROL_LINE_TICKS);
            }
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

        if self.orig_pixel_height == 0 {
            regs.push_task_steps(StepKind::VerticalSkip, SUZY_VERTICAL_SKIP_TICKS - 1);
            regs.set_task_step(TaskStep::RenderLinesEnd);
            return;
        }

        if (self.vsign > 0 && self.voff >= LYNX_SCREEN_HEIGHT as i16)
            || (self.vsign < 0 && self.voff < 0)
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

        let unflipped_right = self.quadrant == 0 || self.quadrant == 1;
        self.hsize_accumulator = if unflipped_right {
            regs.u16(HSIZOFFL)
        } else {
            0
        };

        if self.quadrant == self.start_quadrant {
            self.hquadoff = self.hsign;
        }
        if self.hsign != self.hquadoff {
            self.hoff += self.hsign;
        }

        self.row_render = self.voff >= 0 && self.voff < LYNX_SCREEN_HEIGHT as i16;
        let away_x = (self.hoff < 0 && self.hsign < 0)
            || (self.hoff >= LYNX_SCREEN_WIDTH as i16 && self.hsign > 0);
        self.skip_row = !self.row_render || away_x;

        let mut literal_2bpp_natural_eof = false;
        if !self.skip_row
            && regs.sprctl1() & SPRCTL1_LITERAL != 0
            && regs.sprctl0() & SPRCTL0_BPP == 0x40
            && self.pixel_height > 0
        {
            let source_bytes = u32::from(regs.data(SPRDOFFL).saturating_sub(1));
            let source_pixels = (source_bytes << 2).saturating_sub(1);
            let output_pixels = (u32::from(self.hsize_accumulator)
                + source_pixels * u32::from(regs.u16(SPRHSIZL)))
                >> 8;
            let outputs_before_clip = if self.hsign > 0 {
                (LYNX_SCREEN_WIDTH as i16 - self.hoff) as u32
            } else {
                (self.hoff + 1) as u32
            };
            literal_2bpp_natural_eof = output_pixels <= outputs_before_clip;
        }

        self.pipeline.reset(
            self.row_render,
            !self.skip_row,
            literal_2bpp_natural_eof,
            regs,
            self.pixel_height,
        );
        self.drain_pipeline(regs, true);

        regs.inc_task_step();
    }

    fn render_pixel_height_end(
        &mut self,
        ram: &mut Ram,
        regs: &mut SuzyRegisters,
        absorb_step: bool,
    ) {
        trace!(
            "< render_pixel_height_end. pixel_height:{}",
            self.pixel_height
        );

        let quad_row = self.pixel_height;
        let dx = self.hsign;
        let warm_row = self.pipeline.is_warm_row(quad_row);
        let literal = regs.sprctl1() & SPRCTL1_LITERAL != 0;
        let bpp_mask = regs.sprctl0() & SPRCTL0_BPP;
        let hsiz = regs.u16(SPRHSIZL);
        let sprite_type = regs.sprctl0() & SPRCTL0_SPR_TYPE;
        let pipeline_literal_4bpp = warm_row && literal && bpp_mask == SPRCTL0_BPP && hsiz > 0x0100;
        let pipeline_lower_depth_expansion = literal && bpp_mask != SPRCTL0_BPP && hsiz > 0x0100;

        if !literal && warm_row && hsiz == 0x0100 {
            self.pipeline.finalize_packed(regs, quad_row);
        }

        if pipeline_literal_4bpp {
            self.pipeline.refresh_4bpp(regs, quad_row);
        }

        if !pipeline_lower_depth_expansion {
            self.pipeline
                .finalize_lower_depth_collision(dx, self.hoff, self.row_render, regs, quad_row);
        }

        if warm_row && self.pipeline.output_pixels() > 0 {
            let last_visible_x = if self.row_render {
                self.hoff - dx
            } else if dx > 0 {
                (LYNX_SCREEN_WIDTH - 1) as i16
            } else {
                0
            };
            let partial_word = if dx > 0 {
                (last_visible_x & 1) == 0
            } else {
                (last_visible_x & 1) != 0
            };
            let literal_1bpp = literal && bpp_mask == 0;

            if literal_1bpp {
                let mut finalization_ticks = 0;
                let pipeline_pixels = self
                    .pipeline
                    .source_pixels()
                    .max(self.pipeline.output_pixels());

                if (pipeline_pixels & 1) != 0
                    && (self.row_render || self.pipeline.source_bytes() >= 3)
                {
                    finalization_ticks += SUZY_LITERAL_1BPP_HALF_PAIR_TICKS;
                }

                if partial_word {
                    finalization_ticks += SUZY_PIXEL_BUILDER_EVEN_END_TICKS;
                } else if self.row_render && (pipeline_pixels & 1) == 0 {
                    finalization_ticks += SUZY_PIXEL_BUILDER_COMPLETE_END_TICKS;
                }

                if !self.row_render {
                    let start_x = i32::from(self.hoff)
                        - i32::from(dx) * self.pipeline.output_pixels() as i32;
                    let visible_pixels = if dx > 0 {
                        (LYNX_SCREEN_WIDTH as i32 - start_x.max(0)) as u32
                    } else {
                        (start_x.min((LYNX_SCREEN_WIDTH - 1) as i32) + 1) as u32
                    };

                    if (visible_pixels & 1) != 0 && ((((visible_pixels - 1) >> 4) & 1) != 0) {
                        finalization_ticks += SUZY_REGULAR_CLIP_FIFO_TICKS;
                    }
                }

                if finalization_ticks > 0 {
                    self.pipeline
                        .add_internal_ticks(finalization_ticks, regs, quad_row);
                }
            } else if !literal {
                if partial_word {
                    let video_access = (self.pipeline.video_burst_mask()
                        | self.pipeline.video_read_burst_mask())
                        != 0;

                    if video_access && self.pipeline.video_pixels() > 0 {
                        let final_group = (self.pipeline.video_pixels() - 1) >> 3;

                        if final_group < 32 {
                            let final_bit = 1 << final_group;
                            if (self.pipeline.video_burst_mask() & final_bit) != 0
                                && (self.pipeline.video_read_burst_mask() & final_bit) == 0
                            {
                                self.pipeline
                                    .add_direct(StepKind::VideoMerge, SUZY_VIDEO_MERGE_BURST_TICKS >> 1);
                            }
                        }
                    }
                }
            } else {
                let video_access = (self.pipeline.video_burst_mask()
                    | self.pipeline.video_read_burst_mask())
                    != 0;

                if partial_word && video_access {
                    self.pipeline.add_video_word(regs, quad_row);

                    let final_group = (self.pipeline.video_words() - 1) >> 2;
                    if final_group < 32 {
                        self.pipeline.set_video_read_burst_group(final_group);
                    }
                }

                for group in 0..20 {
                    let group_bit = 1 << group;
                    if (self.pipeline.video_burst_mask() & group_bit) != 0
                        && (self.pipeline.video_read_burst_mask() & group_bit) != 0
                    {
                        let first_word = group << 2;
                        if first_word >= self.pipeline.video_words() {
                            break;
                        }

                        let words = (self.pipeline.video_words() - first_word).min(4);
                        self.pipeline
                            .add_bus_ticks((words << 1) as u16, regs, quad_row);
                    }
                }
            }

            if !literal {
                let screen_collision_mask = self.pipeline.collision_burst_mask()
                    | self.pipeline.collision_read_burst_mask();
                let pipeline_collision_mask = self.pipeline.collision_group_mask()
                    | self.pipeline.collision_read_group_mask();

                if screen_collision_mask.count_ones() > pipeline_collision_mask.count_ones() {
                    let read_only =
                        self.pixel == 0x0E && matches!(sprite_type, 0 | 2 | 6 | 7);
                    let ticks = if sprite_type == 0 || read_only {
                        SUZY_COLLISION_CLEAR_BURST_TICKS
                    } else {
                        SUZY_COLLISION_DETECT_BURST_TICKS
                    };
                    self.pipeline.add_direct(StepKind::CollisionBus, ticks >> 1);
                }

                if !self.row_render {
                    self.pipeline
                        .add_direct(StepKind::ClipFifo, SUZY_REGULAR_CLIP_FIFO_TICKS);

                    if sprite_type != 0
                        && (self.pipeline.video_pixels() & 7) != 0
                        && self.pipeline.collision_group_mask() != 0
                    {
                        self.pipeline
                            .add_direct(StepKind::CollisionGroup, SUZY_COLLISION_PIPELINE_GROUP_TICKS);
                    }
                }
            }

            let pipeline_xor = bpp_mask == SPRCTL0_BPP
                && sprite_type == 6
                && self.row_render
                && !partial_word
                && self.pipeline.output_pixels() > 16
                && self.pixel != 0
                && self.pixel != 0x0E;

            if pipeline_xor {
                self.pipeline.add_direct(StepKind::XorByte, 4);
            }
        }

        if pipeline_lower_depth_expansion {
            self.pipeline
                .finalize_lower_depth_collision(dx, self.hoff, self.row_render, regs, quad_row);
        }

        if self.pipeline.source_pixels() > 0 {
            self.pipeline.set_sprite_row_started(true);
        }

        self.pipeline.advance_sprite_row_ticks(regs, true);
        self.drain_pipeline(regs, absorb_step);

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
            regs.set_task_step(TaskStep::RenderPixelHeightStart);
        }
    }

    fn render_pixels_in_line(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram) {
        trace!("- render_pixels_in_line.");

        if self.skip_row {
            regs.inc_task_step();
            return;
        }

        let literal_bpp = if regs.sprctl1() & SPRCTL1_LITERAL != 0 {
            regs.bpp() + 1
        } else {
            0
        };
        let sprite_type = regs.sprctl0() & SPRCTL0_SPR_TYPE;

        'row: loop {
            let pixel = loop {
                match self.sprite_data.line_get_pixel(regs, &self.pens) {
                    Result::Ok(v) => break v,
                    Result::Err(_e) => {
                        peek_and_store_scb_data!(self, regs, ram);
                        self.pipeline
                            .add_source_byte(regs, self.pixel_height);
                    }
                }
            };

            if pixel == LINE_END {
                break;
            }

            if let Some(packet) = self.sprite_data.take_pending_packet() {
                self.pipeline.add_packed_packet(
                    packet.is_literal(),
                    u32::from(packet.count()),
                    regs,
                    self.pixel_height,
                );
            }

            self.pixel = pixel;
            self.pipeline
                .add_source_pixel(literal_bpp, regs, self.pixel_height);

            self.hsize_accumulator = self.hsize_accumulator.wrapping_add(regs.u16(SPRHSIZL));
            self.pixel_width = (self.hsize_accumulator >> 8) as u8;
            self.hsize_accumulator &= 0xff;

            let mut remaining = self.pixel_width;
            while remaining > 0 {
                self.pipeline
                    .add_output_pixel(literal_bpp, regs, self.pixel_height);

                if (self.hsign > 0 && self.hoff >= LYNX_SCREEN_WIDTH as i16)
                    || (self.hsign < 0 && self.hoff < 0)
                {
                    if literal_bpp == 3 {
                        self.pipeline
                            .discard_3bpp_clipped_output(regs, self.pixel_height);
                    }
                    self.row_render = false;
                    self.hoff = self.hoff.wrapping_add(self.hsign);
                    break 'row;
                }

                if self.hoff >= 0 && self.hoff < LYNX_SCREEN_WIDTH as i16 {
                    self.ever_on_screen = true;
                    self.pipeline.draw_pixel_video_hook(
                        literal_bpp,
                        pen_transparent(sprite_type, self.pixel),
                        regs,
                        self.pixel_height,
                    );
                    self.process_pixel(regs, ram, literal_bpp);
                    self.pipeline.collision_group_complete_hook(
                        literal_bpp,
                        regs,
                        self.pixel_height,
                    );
                    if literal_bpp > 1 {
                        let word_complete = if self.hsign > 0 {
                            self.hoff & 0x01 != 0
                        } else {
                            self.hoff & 0x01 == 0
                        };
                        if word_complete {
                            self.pipeline.add_video_word(regs, self.pixel_height);
                        }
                    }
                    trace!("- RenderPixel. width:{}", self.pixel_width);
                }

                self.hoff = self.hoff.wrapping_add(self.hsign);
                remaining -= 1;
            }
        }

        self.drain_pipeline(regs, true);

        regs.set_task_step(TaskStep::RenderPixelheightEnd);
        self.render_pixel_height_end(ram, regs, false);
    }

    fn pipeline_collision(&self, regs: &SuzyRegisters, literal_bpp: u8) -> bool {
        let bpp_mask = regs.sprctl0() & SPRCTL0_BPP;
        (bpp_mask != SPRCTL0_BPP && literal_bpp > 0)
            || (bpp_mask == SPRCTL0_BPP
                && (literal_bpp == 0 || (literal_bpp == 4 && regs.u16(SPRHSIZL) > 0x0100))
                && self.pixel_height > 0)
    }

    fn collision_write_hook(&mut self, regs: &SuzyRegisters, literal_bpp: u8, clear: bool) {
        let pipeline_collision = self.pipeline_collision(regs, literal_bpp);
        if pipeline_collision {
            self.pipeline.add_collision_pixel(regs, self.pixel_height);
        }

        let burst_bit = 1 << (self.hoff >> 3);
        if (self.pipeline.collision_burst_mask() & burst_bit) == 0 {
            self.pipeline.set_collision_burst_mask(burst_bit);
            let ticks = if (self.pipeline.collision_read_burst_mask() & burst_bit) != 0 {
                SUZY_COLLISION_MERGE_BURST_TICKS
            } else if clear {
                SUZY_COLLISION_CLEAR_BURST_TICKS
            } else {
                SUZY_COLLISION_DETECT_BURST_TICKS
            };
            if pipeline_collision {
                self.pipeline
                    .add_collision_bus_ticks(ticks, literal_bpp, regs, self.pixel_height);
            } else {
                self.pipeline.add_direct(StepKind::CollisionBus, ticks);
            }
        }
    }

    fn collision_read_hook_background(&mut self, regs: &SuzyRegisters, literal_bpp: u8) {
        let pipeline_collision = self.pipeline_collision(regs, literal_bpp);
        if pipeline_collision {
            self.pipeline
                .add_collision_read_pixel(regs, self.pixel_height);
        }

        let burst_bit = 1 << (self.hoff >> 3);
        if (self.pipeline.collision_read_burst_mask() & burst_bit) == 0 {
            self.pipeline.set_collision_read_burst_mask(burst_bit);
            let ticks = if (self.pipeline.collision_burst_mask() & burst_bit) != 0 {
                SUZY_COLLISION_MERGE_BURST_TICKS
            } else {
                SUZY_COLLISION_CLEAR_BURST_TICKS
            };
            if pipeline_collision {
                self.pipeline
                    .add_collision_bus_ticks(ticks, literal_bpp, regs, self.pixel_height);
            } else {
                self.pipeline.add_direct(StepKind::CollisionBus, ticks);
            }
        }
    }

    fn collision_read_hook_shadow(&mut self, regs: &SuzyRegisters, literal_bpp: u8) {
        let pipeline_collision = self.pipeline_collision(regs, literal_bpp);
        if pipeline_collision {
            self.pipeline
                .add_collision_read_pixel(regs, self.pixel_height);
        }

        let burst_bit = 1 << (self.hoff >> 3);
        if ((self.pipeline.collision_burst_mask() | self.pipeline.collision_read_burst_mask())
            & burst_bit)
            == 0
        {
            self.pipeline.set_collision_read_burst_mask(burst_bit);
            if pipeline_collision {
                self.pipeline.add_collision_bus_ticks(
                    SUZY_COLLISION_CLEAR_BURST_TICKS,
                    literal_bpp,
                    regs,
                    self.pixel_height,
                );
            } else {
                self.pipeline
                    .add_direct(StepKind::CollisionBus, SUZY_COLLISION_CLEAR_BURST_TICKS);
            }
        }
    }

    fn write_pixel(&mut self, regs: &SuzyRegisters, ram: &mut Ram, pixel: u8) {
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
    }

    fn read_pixel(&mut self, regs: &SuzyRegisters, ram: &mut Ram) -> u8 {
        let scr_addr: u16 = regs.u16(VIDADRL) + (self.hoff as u16 / 2);

        let mut data: u8 = ram.get(scr_addr);

        if self.hoff & 0x01 == 0 {
            data >>= 4;
        } else {
            data &= 0x0f;
        }

        data
    }

    fn write_collision(&mut self, regs: &SuzyRegisters, ram: &mut Ram, pixel: u8) {
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
    }

    fn read_collision(&mut self, regs: &SuzyRegisters, ram: &mut Ram) -> u8 {
        let col_addr = regs.u16(COLLADRL) + (self.hoff as u16 / 2);

        let mut data: u8 = ram.get(col_addr);

        if self.hoff & 0x01 == 0 {
            data >>= 4;
        } else {
            data &= 0x0f;
        }

        data
    }

    fn process_sprite_type(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprite_type: u8,
        sprcoll: u8,
        literal_bpp: u8,
    ) {
        match sprite_type {
            // 0 - BACKGROUND SHADOW
            0 => self.process_background_shadow(regs, ram, sprcoll, literal_bpp),
            // 1 - BACKGROUND NOCOLLIDE
            1 => self.write_pixel(regs, ram, self.pixel),
            // 2 - BOUNDARY_SHADOW
            2 => self.process_boundary_shadow(regs, ram, sprcoll, literal_bpp),
            // 3 - BOUNDARY
            3 => self.process_boundary_sprite(regs, ram, sprcoll, literal_bpp),
            // 4 - NORMAL
            4 => self.process_normal_sprite(regs, ram, sprcoll, literal_bpp),
            // 5 - NOCOLLIDE
            5 => self.process_nocollide(regs, ram),
            // 6 - XOR SHADOW
            6 => self.process_xor_shadow_sprite(regs, ram, sprcoll, literal_bpp),
            // 7 - SHADOW
            7 => self.process_shadow_sprite(regs, ram, sprcoll, literal_bpp),
            _ => (),
        }
    }

    fn process_nocollide(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram) {
        if self.pixel != 0x00 {
            self.write_pixel(regs, ram, self.pixel);
        }
    }

    fn collide_enabled(regs: &SuzyRegisters, sprcoll: u8) -> bool {
        sprcoll & SPRCOLL_DONT_COLLIDE == 0 && !regs.sprsys_w_is_flag_set(SprSysW::no_collide)
    }

    fn read_modify_collision(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram, sprcoll: u8) {
        let c = self.read_collision(regs, ram);
        if c > self.collision {
            self.collision = c;
        }
        self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
    }

    fn process_background_shadow(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        literal_bpp: u8,
    ) {
        self.write_pixel(regs, ram, self.pixel);
        if !Self::collide_enabled(regs, sprcoll) {
            return;
        }
        if self.pixel == 0x0e {
            self.collision_read_hook_background(regs, literal_bpp);
        } else {
            self.collision_write_hook(regs, literal_bpp, true);
            self.write_collision(regs, ram, sprcoll & SPRCOLL_NUMBER);
        }
    }

    fn process_boundary_shadow(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        literal_bpp: u8,
    ) {
        if self.pixel != 0x00 && self.pixel != 0x0f {
            self.write_pixel(regs, ram, self.pixel);
        }
        if !Self::collide_enabled(regs, sprcoll) {
            return;
        }
        if self.pixel != 0x00 && self.pixel != 0x0e {
            self.collision_write_hook(regs, literal_bpp, false);
            self.read_modify_collision(regs, ram, sprcoll);
        } else if self.pixel == 0x0e {
            self.collision_read_hook_shadow(regs, literal_bpp);
        }
    }

    fn process_boundary_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        literal_bpp: u8,
    ) {
        if self.pixel != 0x00 && self.pixel != 0x0f {
            self.write_pixel(regs, ram, self.pixel);
        }
        if self.pixel != 0x00 && Self::collide_enabled(regs, sprcoll) {
            self.collision_write_hook(regs, literal_bpp, false);
            self.read_modify_collision(regs, ram, sprcoll);
        }
    }

    fn process_normal_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        literal_bpp: u8,
    ) {
        if self.pixel != 0x00 {
            self.write_pixel(regs, ram, self.pixel);
            if Self::collide_enabled(regs, sprcoll) {
                self.collision_write_hook(regs, literal_bpp, false);
                self.read_modify_collision(regs, ram, sprcoll);
            }
        }
    }

    fn process_xor_shadow_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        literal_bpp: u8,
    ) {
        if self.pixel != 0x00 {
            let p = self.read_pixel(regs, ram);
            let pipeline_xor =
                literal_bpp == 4 && regs.u16(SPRHSIZL) > 0x0100 && self.pixel_height > 0;
            if !pipeline_xor && (self.pipeline.video_pixels() & 1) != 0 {
                self.pipeline
                    .add_direct(StepKind::XorByte, SUZY_XOR_BYTE_TICKS);
            }
            self.write_pixel(regs, ram, p ^ self.pixel);
        }
        if !Self::collide_enabled(regs, sprcoll) {
            return;
        }
        if self.pixel != 0x00 && self.pixel != 0x0e {
            self.collision_write_hook(regs, literal_bpp, false);
            self.read_modify_collision(regs, ram, sprcoll);
        } else if self.pixel == 0x0e {
            self.collision_read_hook_shadow(regs, literal_bpp);
        }
    }

    fn process_shadow_sprite(
        &mut self,
        regs: &mut SuzyRegisters,
        ram: &mut Ram,
        sprcoll: u8,
        literal_bpp: u8,
    ) {
        if self.pixel != 0x00 {
            self.write_pixel(regs, ram, self.pixel);
        }
        if !Self::collide_enabled(regs, sprcoll) {
            return;
        }
        if self.pixel != 0x00 && self.pixel != 0x0e {
            self.collision_write_hook(regs, literal_bpp, false);
            self.read_modify_collision(regs, ram, sprcoll);
        } else if self.pixel == 0x0e {
            self.collision_read_hook_shadow(regs, literal_bpp);
        }
    }

    pub fn process_pixel(&mut self, regs: &mut SuzyRegisters, ram: &mut Ram, literal_bpp: u8) {
        trace!(
            "process_pixel() 0x{:04x} 0x{:02x} type:{}",
            self.hoff,
            self.pixel,
            regs.sprctl0() & SPRCTL0_SPR_TYPE
        );

        let sprcoll = regs.sprcoll();
        let sprite_type = regs.sprctl0() & SPRCTL0_SPR_TYPE;
        self.process_sprite_type(regs, ram, sprite_type, sprcoll, literal_bpp);
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
            TaskStep::RenderPixelheightEnd => self.render_pixel_height_end(ram, regs, true),
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
