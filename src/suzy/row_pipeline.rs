use super::step_queue::{StepKind, StepQueue};
use super::{Deserialize, Serialize, SuzyRegisters};
use crate::consts::{SPRCTL0_SPR_TYPE, SPRCTL1_LITERAL, SPRHSIZL};

pub const SUZY_RAM_READ_TICKS: u16 = 2;
pub const SUZY_VISIBLE_ROW_TICKS: u16 = 60;
pub const SUZY_CONTROL_LINE_TICKS: u16 = 54;
pub const SUZY_LINKED_SCB_CONTROL_OVERLAP_TICKS: u16 = 48;
pub const SUZY_ACTIVE_SCB_STARTUP_TICKS: u16 = 35;
pub const SUZY_COLLISION_CLEAR_BURST_TICKS: u16 = 10;
pub const SUZY_COLLISION_DETECT_BURST_TICKS: u16 = 18;
pub const SUZY_COLLISION_MERGE_BURST_TICKS: u16 = 8;
pub const SUZY_SOURCE_FIFO_BURST_TICKS: u16 = 10;
pub const SUZY_SOURCE_FIFO_BYTES: u16 = 8;
pub const SUZY_SOURCE_FIFO_TURNOVER_TICKS: u16 = 1;
pub const SUZY_PIXEL_FIFO_OUTPUTS: u16 = 16;
pub const SUZY_VIDEO_WRITE_BURST_TICKS: u16 = 10;
pub const SUZY_VIDEO_READ_BURST_TICKS: u16 = 10;
pub const SUZY_VIDEO_MERGE_BURST_TICKS: u16 = 8;
pub const SUZY_XOR_BYTE_TICKS: u16 = 2;
pub const SUZY_PIPELINE_PIXEL_PAIR_TICKS: u16 = 5;
pub const SUZY_CLIPPED_ROW_TICKS: u16 = 46;
pub const SUZY_LITERAL_1BPP_PIXEL_PAIR_TICKS: u16 = 4;
pub const SUZY_LITERAL_ROW_INTERNAL_TICKS: u16 = 70;
pub const SUZY_LITERAL_4BPP_UPSCALE_TICKS: u16 = 72;
pub const SUZY_LITERAL_4BPP_COLLISION_TICKS: u16 = 78;
pub const SUZY_LITERAL_4BPP_SOURCE_OVERLAP_TICKS: u16 = 8;
pub const SUZY_COLLISION_PIPELINE_GROUP_TICKS: u16 = 5;
pub const SUZY_UNPACKER_SHIFT_BITS: u16 = 12;
pub const SUZY_LITERAL_1BPP_RECORD_TICKS: u16 = 5;
pub const SUZY_LITERAL_1BPP_HALF_PAIR_TICKS: u16 = 2;
pub const SUZY_PIXEL_BUILDER_COMPLETE_END_TICKS: u16 = 4;
pub const SUZY_PIXEL_BUILDER_EVEN_END_TICKS: u16 = 6;
pub const SUZY_REGULAR_CLIP_FIFO_TICKS: u16 = 6;
pub const SUZY_VERTICAL_SKIP_TICKS: u16 = 18;
pub const SUZY_STRETCH_ROW_TICKS: u16 = 8;
pub const SUZY_TILT_ROW_TICKS: u16 = 18;
pub const SUZY_PACKED_PACKET_TICKS: u16 = 4;
pub const SUZY_PACKED_READINESS_TICKS: u16 = 72;
pub const SUZY_PACKED_SCALED_PACKET_TICKS: u16 = 1;
pub const SUZY_PACKED_SCALED_PACKET_PAIR_TICKS: u16 = 3;
pub const SUZY_PACKED_ROW_BUS_TICKS: u16 = 12;
pub const SUZY_PACKED_ROW_INTERNAL_TICKS: u16 = 24;
pub const SUZY_PACKED_COLLISION_HANDOFF_TICKS: u16 = 3;
pub const SUZY_LCD_DMA_BURST_TICKS: u16 = 28;
pub const SUZY_BUS_GRANT_OVERHEAD_TICKS: u16 = 10;

const SPRCTL0_BPP_MASK: u8 = 0xC0;

#[derive(Serialize, Deserialize)]
pub struct RowPipeline {
    bus_ticks: u32,
    internal_ticks: u32,
    internal_base_ticks: u32,
    charged_ticks: u32,
    pending: StepQueue,
    source_bytes: u32,
    source_pixels: u32,
    output_pixels: u32,
    video_pixels: u32,
    video_words: u32,
    video_burst_mask: u32,
    video_read_burst_mask: u32,
    collision_group_mask: u32,
    collision_read_group_mask: u32,
    collision_burst_mask: u32,
    collision_read_burst_mask: u32,
    packed_packet_ticks: u32,
    packed_rle_seen: bool,
    packed_literal_excess: i32,
    packed_builder_stall_ticks: u32,
    packed_literal_start_pixels: u32,
    packed_literal_run: bool,
    lcd_dma_granted: bool,
    sprite_row_started: bool,
    row_pipeline_warm: bool,
    first_scb: bool,
    scb_control_line_pending: bool,
    expansion_fifo_primed: bool,
}

impl RowPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bus_ticks: 0,
            internal_ticks: 0,
            internal_base_ticks: 0,
            charged_ticks: 0,
            pending: StepQueue::default(),
            source_bytes: 0,
            source_pixels: 0,
            output_pixels: 0,
            video_pixels: 0,
            video_words: 0,
            video_burst_mask: 0,
            video_read_burst_mask: 0,
            collision_group_mask: 0,
            collision_read_group_mask: 0,
            collision_burst_mask: 0,
            collision_read_burst_mask: 0,
            packed_packet_ticks: 0,
            packed_rle_seen: false,
            packed_literal_excess: 0,
            packed_builder_stall_ticks: 0,
            packed_literal_start_pixels: 0,
            packed_literal_run: false,
            lcd_dma_granted: false,
            sprite_row_started: false,
            row_pipeline_warm: false,
            first_scb: true,
            scb_control_line_pending: false,
            expansion_fifo_primed: false,
        }
    }

    #[must_use]
    pub fn sprite_row_started(&self) -> bool {
        self.sprite_row_started
    }

    pub fn set_sprite_row_started(&mut self, v: bool) {
        self.sprite_row_started = v;
    }

    #[must_use]
    pub fn row_pipeline_warm(&self) -> bool {
        self.row_pipeline_warm
    }

    pub fn set_row_pipeline_warm(&mut self, v: bool) {
        self.row_pipeline_warm = v;
    }

    #[must_use]
    pub fn first_scb(&self) -> bool {
        self.first_scb
    }

    pub fn set_first_scb(&mut self, v: bool) {
        self.first_scb = v;
    }

    #[must_use]
    pub fn scb_control_line_pending(&self) -> bool {
        self.scb_control_line_pending
    }

    pub fn set_scb_control_line_pending(&mut self, v: bool) {
        self.scb_control_line_pending = v;
    }

    #[must_use]
    pub fn source_pixels(&self) -> u32 {
        self.source_pixels
    }

    #[must_use]
    pub fn source_bytes(&self) -> u32 {
        self.source_bytes
    }

    #[must_use]
    pub fn is_warm_row(&self, quad_row: u8) -> bool {
        self.is_warm(quad_row)
    }

    #[must_use]
    pub fn video_burst_mask(&self) -> u32 {
        self.video_burst_mask
    }

    #[must_use]
    pub fn video_read_burst_mask(&self) -> u32 {
        self.video_read_burst_mask
    }

    pub fn set_video_read_burst_group(&mut self, group: u32) {
        self.video_read_burst_mask |= 1 << group;
    }

    #[must_use]
    pub fn collision_group_mask(&self) -> u32 {
        self.collision_group_mask
    }

    #[must_use]
    pub fn collision_read_group_mask(&self) -> u32 {
        self.collision_read_group_mask
    }

    pub fn add_internal_ticks(&mut self, ticks: u16, regs: &SuzyRegisters, quad_row: u8) {
        self.internal_ticks += u32::from(ticks);
        self.update_timing(regs, quad_row);
    }

    pub fn refresh_4bpp(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        self.update_4bpp(regs, quad_row);
    }

    pub fn collision_group_complete_hook(
        &mut self,
        literal_bpp: u8,
        regs: &SuzyRegisters,
        quad_row: u8,
    ) {
        if (regs.sprctl0() & SPRCTL0_BPP_MASK) == SPRCTL0_BPP_MASK
            && (literal_bpp == 0 || (literal_bpp == 4 && regs.u16(SPRHSIZL) > 0x0100))
            && quad_row > 0
            && self.video_pixels.trailing_zeros() >= 3
            && (self.collision_group_mask | self.collision_read_group_mask) != 0
        {
            self.update_4bpp(regs, quad_row);
        }
    }

    pub fn sprite_end_next_scb(&mut self) {
        self.sprite_row_started = false;
        self.expansion_fifo_primed = false;
    }

    #[must_use]
    pub fn output_pixels(&self) -> u32 {
        self.output_pixels
    }

    #[must_use]
    pub fn video_pixels(&self) -> u32 {
        self.video_pixels
    }

    #[must_use]
    pub fn video_words(&self) -> u32 {
        self.video_words
    }

    #[must_use]
    pub fn collision_burst_mask(&self) -> u32 {
        self.collision_burst_mask
    }

    #[must_use]
    pub fn collision_read_burst_mask(&self) -> u32 {
        self.collision_read_burst_mask
    }

    pub fn set_collision_burst_mask(&mut self, bit: u32) {
        self.collision_burst_mask |= bit;
    }

    pub fn set_collision_read_burst_mask(&mut self, bit: u32) {
        self.collision_read_burst_mask |= bit;
    }

    pub fn set_lcd_dma_granted(&mut self) {
        self.lcd_dma_granted = true;
    }

    pub fn sprite_go_start(&mut self) {
        self.sprite_row_started = false;
        self.row_pipeline_warm = false;
        self.first_scb = true;
        self.scb_control_line_pending = false;
        self.expansion_fifo_primed = false;
    }

    pub fn blitter_done(&mut self) {
        self.row_pipeline_warm = false;
        self.scb_control_line_pending = false;
    }

    pub fn take_pending_steps(&mut self) -> StepQueue {
        core::mem::take(&mut self.pending)
    }

    pub fn add_direct(&mut self, kind: StepKind, ticks: u16) {
        self.pending.push(kind, ticks);
    }

    #[must_use]
    fn is_warm(&self, quad_row: u8) -> bool {
        quad_row > 0 || self.sprite_row_started || self.row_pipeline_warm
    }

    fn update_timing(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let mut row_ticks = self.bus_ticks.max(self.internal_ticks);
        let pipeline_xor = quad_row > 0
            && regs.sprctl1() & SPRCTL1_LITERAL != 0
            && regs.sprctl0() & SPRCTL0_BPP_MASK == SPRCTL0_BPP_MASK
            && regs.sprctl0() & SPRCTL0_SPR_TYPE == 6
            && regs.u16(SPRHSIZL) > 0x0100;

        if pipeline_xor {
            row_ticks += ((self.output_pixels + 1) >> 1) * u32::from(SUZY_XOR_BYTE_TICKS);
        }

        if row_ticks > self.charged_ticks {
            let kind = if self.bus_ticks == row_ticks {
                StepKind::BusFrontier
            } else {
                StepKind::InternalFrontier
            };
            self.pending.push(kind, (row_ticks - self.charged_ticks) as u16);
            self.charged_ticks = row_ticks;
        }
    }

    pub fn add_bus_ticks(&mut self, ticks: u16, regs: &SuzyRegisters, quad_row: u8) {
        self.bus_ticks += u32::from(ticks);
        self.update_timing(regs, quad_row);
    }

    fn update_internal(
        &mut self,
        pipeline_pixels: u32,
        literal_bpp: u8,
        regs: &SuzyRegisters,
        quad_row: u8,
    ) {
        self.internal_ticks = self.internal_base_ticks
            + self.pixel_ticks(pipeline_pixels, literal_bpp, quad_row)
            + self.packed_packet_ticks;
        self.update_timing(regs, quad_row);
    }

    fn update_2bpp(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let pipeline_pixels = self.source_pixels.max(self.output_pixels);
        self.internal_ticks = self.internal_base_ticks
            + (pipeline_pixels << 1)
            + ((self.source_pixels << 1) / u32::from(SUZY_UNPACKER_SHIFT_BITS))
            + self.packed_packet_ticks;
        self.update_timing(regs, quad_row);
    }

    fn update_3bpp(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let pipeline_pixels = self.source_pixels.max(self.output_pixels);
        self.internal_ticks = self.internal_base_ticks
            + (pipeline_pixels << 1)
            + (self.source_pixels >> 2)
            + self.packed_packet_ticks;
        self.update_timing(regs, quad_row);
    }

    pub fn discard_3bpp_clipped_output(&mut self, _regs: &SuzyRegisters, quad_row: u8) {
        if !self.is_warm(quad_row) || self.internal_ticks < 2 {
            return;
        }

        let previous_ticks = self.bus_ticks.max(self.internal_ticks);
        self.internal_ticks -= 2;
        let clipped_ticks = self.bus_ticks.max(self.internal_ticks);
        let discarded_ticks = previous_ticks - clipped_ticks;

        self.charged_ticks -= discarded_ticks;
        self.pending.pop_back_n(discarded_ticks);
    }

    fn update_4bpp(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let packed = regs.sprctl1() & SPRCTL1_LITERAL == 0;
        let warm_row = self.is_warm(quad_row);
        let mut source_ticks =
            (self.source_pixels * u32::from(SUZY_PIPELINE_PIXEL_PAIR_TICKS) + 1) >> 1;
        if warm_row
            && self.video_pixels > 0
            && source_ticks >= u32::from(SUZY_LITERAL_4BPP_SOURCE_OVERLAP_TICKS)
        {
            source_ticks -= u32::from(SUZY_LITERAL_4BPP_SOURCE_OVERLAP_TICKS);
        }

        let source_fifo_windows =
            (self.source_bytes + u32::from(SUZY_SOURCE_FIFO_BYTES)) >> 3;
        let output_ticks = if regs.u16(SPRHSIZL) > 0x0100
            && self.output_pixels > self.source_pixels
        {
            u32::from(SUZY_LITERAL_4BPP_UPSCALE_TICKS) + (self.output_pixels << 1)
        } else {
            u32::from(SUZY_VISIBLE_ROW_TICKS)
                + source_fifo_windows
                + ((self.output_pixels * u32::from(SUZY_PIPELINE_PIXEL_PAIR_TICKS) + 1) >> 1)
        };
        let collision_mask = self.collision_group_mask | self.collision_read_group_mask;

        let mut internal_ticks = if packed {
            self.internal_ticks
        } else {
            source_ticks.max(output_ticks) + self.packed_packet_ticks
        };

        if collision_mask != 0 {
            let collision_groups = collision_mask.count_ones();
            let complete_groups = self.video_pixels >> 3;
            let complete_mask = if complete_groups >= 32 {
                0xFFFF_FFFF
            } else {
                (1 << complete_groups) - 1
            };
            let detect_mask = self.collision_group_mask & complete_mask;
            let detect_groups = detect_mask.count_ones();

            let mut collision_ticks = u32::from(SUZY_LITERAL_4BPP_COLLISION_TICKS)
                + (self.output_pixels << 1)
                + collision_groups * u32::from(SUZY_COLLISION_PIPELINE_GROUP_TICKS);

            if packed {
                collision_ticks += u32::from(SUZY_PACKED_COLLISION_HANDOFF_TICKS);

                if (collision_mask & 1) == 0 {
                    collision_ticks += u32::from(SUZY_COLLISION_CLEAR_BURST_TICKS)
                        - u32::from(SUZY_PACKED_COLLISION_HANDOFF_TICKS);
                }
            }

            if (regs.sprctl0() & SPRCTL0_SPR_TYPE) != 0 {
                collision_ticks += detect_groups * u32::from(SUZY_COLLISION_MERGE_BURST_TICKS);
            }

            internal_ticks = internal_ticks.max(collision_ticks);
        }

        self.internal_ticks = internal_ticks;
        self.update_timing(regs, quad_row);
    }

    pub fn finalize_lower_depth_collision(
        &mut self,
        dx: i16,
        row_x: i16,
        row_render: bool,
        regs: &SuzyRegisters,
        quad_row: u8,
    ) {
        if (regs.sprctl0() & SPRCTL0_BPP_MASK) == SPRCTL0_BPP_MASK {
            return;
        }

        let collision_mask = self.collision_group_mask | self.collision_read_group_mask;
        if collision_mask == 0 {
            return;
        }

        let collision_groups = collision_mask.count_ones();
        let complete_groups = self.video_pixels >> 3;
        let complete_mask = if complete_groups >= 32 {
            0xFFFF_FFFF
        } else {
            (1 << complete_groups) - 1
        };
        let detect_groups = (self.collision_group_mask & complete_mask).count_ones();
        let mut collision_ticks = u32::from(SUZY_LITERAL_4BPP_COLLISION_TICKS)
            + (self.output_pixels << 1)
            + collision_groups * u32::from(SUZY_COLLISION_PIPELINE_GROUP_TICKS);

        let collision_detect = (regs.sprctl0() & SPRCTL0_SPR_TYPE) != 0;
        if collision_detect {
            collision_ticks += detect_groups * u32::from(SUZY_COLLISION_MERGE_BURST_TICKS);
        }

        self.internal_ticks = self.internal_ticks.max(collision_ticks);
        self.update_timing(regs, quad_row);

        let lower_depth_expansion =
            regs.sprctl1() & SPRCTL1_LITERAL != 0 && regs.u16(SPRHSIZL) > 0x0100;

        if lower_depth_expansion && collision_detect && row_render {
            let start_x = i32::from(row_x) - i32::from(dx) * self.output_pixels as i32;
            let first_visible_x = if dx > 0 {
                start_x.max(0)
            } else {
                start_x.min((crate::consts::LYNX_SCREEN_WIDTH - 1) as i32)
            };

            if (first_visible_x & 1) != 0 {
                self.add_direct(
                    StepKind::CollisionHalfPair,
                    (SUZY_COLLISION_DETECT_BURST_TICKS >> 1) + SUZY_LITERAL_1BPP_HALF_PAIR_TICKS,
                );
            }
        }
    }

    #[must_use]
    fn packed_literal_ticks(&self, finalizing: bool, quad_row: u8) -> u32 {
        if self.packed_rle_seen {
            return 0;
        }

        let mut builder_ticks = u32::from(SUZY_PACKED_READINESS_TICKS)
            + self.pixel_ticks(self.source_pixels, 0, quad_row)
            + (self.packed_packet_ticks >> 1);
        let packet_ticks = self.packed_packet_ticks >> 1;
        let fifo_headroom = if packet_ticks < u32::from(SUZY_PIXEL_FIFO_OUTPUTS) {
            u32::from(SUZY_PIXEL_FIFO_OUTPUTS) - packet_ticks
        } else {
            0
        };

        if !finalizing || self.output_pixels.trailing_zeros() >= 3 {
            builder_ticks -= fifo_headroom;
        }

        builder_ticks
    }

    fn update_packed(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let output_ticks = u32::from(SUZY_PACKED_READINESS_TICKS)
            + (self.output_pixels << 1)
            + self.packed_packet_ticks
            + self.packed_builder_stall_ticks;
        let builder_ticks = self.packed_literal_ticks(false, quad_row);

        self.internal_ticks = output_ticks.max(builder_ticks);
        self.update_timing(regs, quad_row);
    }

    fn finalize_packed_literal_run(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        if !self.packed_literal_run {
            return;
        }

        if self.packed_rle_seen && self.packed_literal_excess > 0 {
            let phase = self.output_pixels & 15;
            let used_words = (phase + 1) >> 1;
            let free_words = if phase == 0 { 8 } else { 8 - used_words };

            let visible_fifo_drain = self.video_pixels > 0;
            let visible_headroom = if self.packed_literal_start_pixels == 0 {
                9
            } else {
                8
            };
            let fifo_overflow = self.packed_literal_excess as u32 > visible_headroom;

            if (!visible_fifo_drain || fifo_overflow)
                && self.packed_literal_excess as u32 > free_words
            {
                let stall_ticks = self.packed_literal_excess as u32 - free_words;
                self.packed_builder_stall_ticks += stall_ticks;
            }
        }

        self.packed_literal_excess = 0;
        self.packed_literal_start_pixels = 0;
        self.packed_literal_run = false;
        self.update_packed(regs, quad_row);
    }

    pub fn finalize_packed(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        self.finalize_packed_literal_run(regs, quad_row);

        if !self.packed_rle_seen && self.lcd_dma_granted {
            let packet_ticks = self.packed_packet_ticks >> 1;
            let fifo_headroom = if packet_ticks < u32::from(SUZY_PIXEL_FIFO_OUTPUTS) {
                u32::from(SUZY_PIXEL_FIFO_OUTPUTS) - packet_ticks
            } else {
                0
            };
            let mut recovery_ticks = u32::from(SUZY_LCD_DMA_BURST_TICKS)
                + u32::from(SUZY_BUS_GRANT_OVERHEAD_TICKS)
                + packet_ticks;
            if self.output_pixels.trailing_zeros() >= 3 {
                recovery_ticks -= fifo_headroom >> 1;
            }
            self.add_bus_ticks(recovery_ticks as u16, regs, quad_row);
        }

        let phase = self.output_pixels & 7;
        let finalization_ticks = if (phase & 1) != 0 {
            phase + 1
        } else if phase > 0 {
            phase - 2
        } else {
            0
        };
        let output_ticks = u32::from(SUZY_PACKED_READINESS_TICKS)
            + (self.output_pixels << 1)
            + self.packed_packet_ticks
            + self.packed_builder_stall_ticks
            + finalization_ticks;
        let builder_ticks = self.packed_literal_ticks(true, quad_row);

        self.internal_ticks = output_ticks.max(builder_ticks);
        self.update_timing(regs, quad_row);
    }

    fn clear(&mut self) {
        self.collision_group_mask = 0;
        self.collision_read_group_mask = 0;
        self.video_burst_mask = 0;
        self.video_read_burst_mask = 0;
        self.bus_ticks = 0;
        self.internal_ticks = 0;
        self.internal_base_ticks = 0;
        self.charged_ticks = 0;
        self.source_bytes = 0;
        self.source_pixels = 0;
        self.output_pixels = 0;
        self.packed_packet_ticks = 0;
        self.packed_rle_seen = false;
        self.packed_literal_excess = 0;
        self.packed_builder_stall_ticks = 0;
        self.packed_literal_start_pixels = 0;
        self.packed_literal_run = false;
        self.video_pixels = 0;
        self.video_words = 0;
        self.lcd_dma_granted = false;
    }

    pub fn clear_row(&mut self) {
        self.clear();
        self.collision_burst_mask = 0;
        self.collision_read_burst_mask = 0;
    }

    pub fn reset(
        &mut self,
        visible: bool,
        process_pixels: bool,
        literal_2bpp_natural_eof: bool,
        regs: &SuzyRegisters,
        quad_row: u8,
    ) {
        self.clear();
        self.collision_burst_mask = 0;
        self.collision_read_burst_mask = 0;

        if !process_pixels {
            self.internal_ticks = u32::from(SUZY_CLIPPED_ROW_TICKS);
            self.update_timing(regs, quad_row);
            return;
        }

        let literal = regs.sprctl1() & SPRCTL1_LITERAL != 0;
        let literal_1bpp = literal && (regs.sprctl0() & SPRCTL0_BPP_MASK) == 0;
        let repeated_or_linked =
            quad_row > 0 || self.row_pipeline_warm || (literal_1bpp && self.sprite_row_started);
        let packed_warm = !literal
            && (quad_row > 0
                || ((self.sprite_row_started || self.row_pipeline_warm)
                    && regs.u16(SPRHSIZL) == 0x0100));

        if packed_warm {
            self.bus_ticks = u32::from(SUZY_PACKED_ROW_BUS_TICKS);
            self.internal_base_ticks = u32::from(SUZY_PACKED_ROW_INTERNAL_TICKS);
            self.internal_ticks = self.internal_base_ticks;
        } else if literal
            && (((regs.sprctl0() & SPRCTL0_BPP_MASK) == 0
                || (regs.sprctl0() & SPRCTL0_BPP_MASK) == 0x80)
                || literal_2bpp_natural_eof)
            && repeated_or_linked
        {
            self.internal_base_ticks = u32::from(SUZY_LITERAL_ROW_INTERNAL_TICKS);
            self.internal_ticks = self.internal_base_ticks;
        }

        if visible && !packed_warm {
            self.bus_ticks += u32::from(SUZY_VISIBLE_ROW_TICKS);
        }

        self.update_timing(regs, quad_row);
    }

    pub fn add_source_byte(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        if self.source_bytes.trailing_zeros() >= 2 {
            self.add_bus_ticks(SUZY_SOURCE_FIFO_BURST_TICKS, regs, quad_row);
        }

        let literal_4bpp = regs.sprctl1() & SPRCTL1_LITERAL != 0
            && (regs.sprctl0() & SPRCTL0_BPP_MASK) == SPRCTL0_BPP_MASK;
        let warm_row = self.is_warm(quad_row);
        let source_stream_bytes = self.source_bytes + 1;
        if literal_4bpp
            && warm_row
            && source_stream_bytes > u32::from(SUZY_SOURCE_FIFO_BYTES)
            && (source_stream_bytes & u32::from(SUZY_SOURCE_FIFO_BYTES - 1)) == 0
        {
            self.add_bus_ticks(SUZY_SOURCE_FIFO_TURNOVER_TICKS, regs, quad_row);
        }

        self.source_bytes += 1;
    }

    #[must_use]
    fn pixel_ticks(&self, pixels: u32, literal_bpp: u8, quad_row: u8) -> u32 {
        let mut pair_ticks = u32::from(SUZY_PIPELINE_PIXEL_PAIR_TICKS);
        let mut pixels = pixels;

        if literal_bpp == 1 && self.is_warm(quad_row) {
            if pixels <= 3 {
                return 0;
            }

            pair_ticks = u32::from(SUZY_LITERAL_1BPP_PIXEL_PAIR_TICKS);
            pixels &= !1;
        }

        (pixels * pair_ticks + 1) >> 1
    }

    pub fn add_source_pixel(&mut self, literal_bpp: u8, regs: &SuzyRegisters, quad_row: u8) {
        self.source_pixels += 1;

        if literal_bpp == 0 && self.is_warm(quad_row) && regs.u16(SPRHSIZL) == 0x0100 {
            self.update_packed(regs, quad_row);
        } else if literal_bpp == 3 && (quad_row > 0 || self.row_pipeline_warm) {
            self.update_3bpp(regs, quad_row);
        } else if literal_bpp == 4 && self.is_warm(quad_row) {
            self.update_4bpp(regs, quad_row);
        } else if literal_bpp == 2
            && self.internal_base_ticks == u32::from(SUZY_LITERAL_ROW_INTERNAL_TICKS)
            && (quad_row > 0 || self.row_pipeline_warm)
        {
            self.update_2bpp(regs, quad_row);
        } else if self.source_pixels > self.output_pixels {
            self.update_internal(self.source_pixels, literal_bpp, regs, quad_row);
        }
    }

    pub fn add_packed_packet(&mut self, literal: bool, count: u32, regs: &SuzyRegisters, quad_row: u8) {
        let mut packet_ticks = u32::from(SUZY_PACKED_PACKET_TICKS);

        if literal {
            if !self.packed_literal_run {
                self.packed_literal_start_pixels = self.output_pixels;
            }

            let literal_ticks =
                2 + ((count * u32::from(SUZY_PIPELINE_PIXEL_PAIR_TICKS) + 1) >> 1);
            let common_ticks = u32::from(SUZY_PACKED_PACKET_TICKS) + (count << 1);
            self.packed_literal_excess += literal_ticks as i32 - common_ticks as i32;
            self.packed_literal_run = true;
        } else {
            if !self.packed_rle_seen && self.is_warm(quad_row) && regs.u16(SPRHSIZL) == 0x0100 {
                self.add_bus_ticks(SUZY_VISIBLE_ROW_TICKS, regs, quad_row);
            }

            self.packed_rle_seen = true;
            self.finalize_packed_literal_run(regs, quad_row);
        }

        if quad_row > 0 && regs.u16(SPRHSIZL) > 0x0100 {
            packet_ticks = if self.packed_packet_ticks % u32::from(SUZY_PACKED_SCALED_PACKET_PAIR_TICKS)
                == u32::from(SUZY_PACKED_SCALED_PACKET_TICKS)
            {
                u32::from(SUZY_PACKED_SCALED_PACKET_PAIR_TICKS - SUZY_PACKED_SCALED_PACKET_TICKS)
            } else {
                u32::from(SUZY_PACKED_SCALED_PACKET_TICKS)
            };
        }

        self.packed_packet_ticks += packet_ticks;
        if self.is_warm(quad_row) && regs.u16(SPRHSIZL) == 0x0100 {
            self.update_packed(regs, quad_row);
        } else {
            self.update_internal(
                self.source_pixels.max(self.output_pixels),
                0,
                regs,
                quad_row,
            );
        }
    }

    pub fn add_output_pixel(&mut self, literal_bpp: u8, regs: &SuzyRegisters, quad_row: u8) {
        self.output_pixels += 1;

        if literal_bpp == 0 && self.is_warm(quad_row) && regs.u16(SPRHSIZL) == 0x0100 {
            self.update_packed(regs, quad_row);
        } else if literal_bpp == 4 && self.is_warm(quad_row) {
            self.update_4bpp(regs, quad_row);
        } else if literal_bpp == 2
            && self.internal_base_ticks == u32::from(SUZY_LITERAL_ROW_INTERNAL_TICKS)
            && (quad_row > 0 || self.row_pipeline_warm)
        {
            self.update_2bpp(regs, quad_row);
        } else if self.output_pixels > self.source_pixels {
            if literal_bpp == 3 && (quad_row > 0 || self.row_pipeline_warm) {
                self.update_3bpp(regs, quad_row);
            } else {
                self.update_internal(self.output_pixels, literal_bpp, regs, quad_row);
            }
        }
    }

    pub fn add_video_pixel(&mut self, literal_bpp: u8, regs: &SuzyRegisters, quad_row: u8) {
        let literal_1bpp = literal_bpp == 1;
        let literal_word_timing = literal_bpp > 1;
        let group = if literal_word_timing {
            self.video_words >> 2
        } else if literal_1bpp {
            (self.output_pixels - 1) >> 3
        } else {
            (self.video_pixels - 1) >> 3
        };

        if group >= 32 {
            return;
        }

        let burst_bit = 1 << group;

        if (self.video_burst_mask & burst_bit) == 0 {
            self.video_burst_mask |= burst_bit;
            let first_1bpp_transaction =
                literal_1bpp && self.video_burst_mask == 1 && self.video_read_burst_mask == 0;

            if !first_1bpp_transaction && !literal_word_timing {
                let ticks = if (self.video_read_burst_mask & burst_bit) != 0 {
                    SUZY_VIDEO_MERGE_BURST_TICKS
                } else {
                    SUZY_VIDEO_WRITE_BURST_TICKS
                };
                self.add_bus_ticks(ticks, regs, quad_row);
            }
        }
    }

    pub fn add_video_read_pixel(&mut self, literal_bpp: u8, regs: &SuzyRegisters, quad_row: u8) {
        let literal_1bpp = literal_bpp == 1;
        let literal_word_timing = literal_bpp > 1;
        let group = if literal_word_timing {
            self.video_words >> 2
        } else if literal_1bpp {
            (self.output_pixels - 1) >> 3
        } else {
            (self.video_pixels - 1) >> 3
        };

        if group >= 32 {
            return;
        }

        let burst_bit = 1 << group;

        if (self.video_read_burst_mask & burst_bit) == 0 {
            self.video_read_burst_mask |= burst_bit;
            let first_1bpp_transaction =
                literal_1bpp && self.video_read_burst_mask == 1 && self.video_burst_mask == 0;

            if !first_1bpp_transaction && !literal_word_timing {
                let ticks = if (self.video_burst_mask & burst_bit) != 0 {
                    SUZY_VIDEO_MERGE_BURST_TICKS
                } else {
                    SUZY_VIDEO_READ_BURST_TICKS
                };
                self.add_bus_ticks(ticks, regs, quad_row);
            }
        }
    }

    pub fn add_collision_pixel(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let group = (self.video_pixels - 1) >> 3;

        if group < 32 {
            let group_bit = 1 << group;
            if (self.collision_group_mask & group_bit) == 0 {
                self.collision_group_mask |= group_bit;
                if (regs.sprctl0() & SPRCTL0_BPP_MASK) == SPRCTL0_BPP_MASK {
                    self.update_4bpp(regs, quad_row);
                }
            }
        }
    }

    pub fn add_collision_read_pixel(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let group = (self.video_pixels - 1) >> 3;

        if group < 32 {
            let group_bit = 1 << group;
            if (self.collision_read_group_mask & group_bit) == 0 {
                self.collision_read_group_mask |= group_bit;
                if (regs.sprctl0() & SPRCTL0_BPP_MASK) == SPRCTL0_BPP_MASK {
                    self.update_4bpp(regs, quad_row);
                }
            }
        }
    }

    pub fn add_collision_bus_ticks(
        &mut self,
        ticks: u16,
        literal_bpp: u8,
        regs: &SuzyRegisters,
        quad_row: u8,
    ) {
        if literal_bpp == 0 {
            self.add_bus_ticks(ticks >> 1, regs, quad_row);
            self.update_4bpp(regs, quad_row);
        } else if literal_bpp == 4 {
            self.add_bus_ticks(ticks, regs, quad_row);
            self.update_4bpp(regs, quad_row);
        } else if regs.u16(SPRHSIZL) > 0x0100 {
            let screen_mask = self.collision_burst_mask | self.collision_read_burst_mask;
            let pipeline_mask = self.collision_group_mask | self.collision_read_group_mask;

            if screen_mask.count_ones() <= pipeline_mask.count_ones() {
                self.add_bus_ticks(ticks, regs, quad_row);
                self.update_4bpp(regs, quad_row);
            }
        } else {
            self.add_bus_ticks(ticks, regs, quad_row);
            self.update_4bpp(regs, quad_row);
        }
    }

    pub fn add_video_word(&mut self, regs: &SuzyRegisters, quad_row: u8) {
        let ticks = if self.video_words.trailing_zeros() >= 2 { 4 } else { 2 };
        self.add_bus_ticks(ticks, regs, quad_row);
        self.video_words += 1;
    }

    pub fn advance_sprite_row_ticks(&mut self, regs: &SuzyRegisters, charge_transform: bool) {
        let reload_depth = (regs.sprctl1() >> 4) & 0x03;

        if charge_transform && reload_depth >= 2 {
            self.add_direct(StepKind::StretchRow, SUZY_STRETCH_ROW_TICKS);
        }
        if charge_transform && reload_depth >= 3 && !self.lcd_dma_granted {
            self.add_direct(StepKind::TiltRow, SUZY_TILT_ROW_TICKS);
        }
    }

    pub fn draw_pixel_video_hook(
        &mut self,
        literal_bpp: u8,
        transparent: bool,
        regs: &SuzyRegisters,
        quad_row: u8,
    ) {
        self.video_pixels += 1;
        if literal_bpp == 4 && regs.u16(SPRHSIZL) > 0x0100 && self.video_pixels >= 8 {
            self.expansion_fifo_primed = true;
        }

        if transparent {
            self.add_video_read_pixel(literal_bpp, regs, quad_row);
        } else {
            self.add_video_pixel(literal_bpp, regs, quad_row);
        }
    }
}

impl Default for RowPipeline {
    fn default() -> Self {
        RowPipeline::new()
    }
}
