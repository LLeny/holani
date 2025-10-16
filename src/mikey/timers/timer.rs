use alloc::fmt;

use crate::mikey::timers::CTRLB_BORROW_IN_BIT;

use super::{
    alloc, trace, Deserialize, NonZeroU8, Serialize, CTRLA_ENABLE_COUNT_BIT,
    CTRLA_ENABLE_RELOAD_BIT, CTRLA_INTERRUPT_BIT, CTRLA_PERIOD_BIT, CTRLA_RESET_DONE_BIT,
    CTRLB_BORROW_OUT_BIT, CTRLB_TIMER_DONE_BIT, TIMER_TICKS_COUNT,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Timer {
    id: u8,
    int: u8,
    backup: u8,
    control_a: u8,
    count: u8,
    control_b: u8,
    clock_ticks: Option<u16>,
    tick_countdown: u16,
    linked: Option<NonZeroU8>,
    is_linked: bool,
    count_enabled: bool,
    reload_enabled: bool,
}

impl Timer {
    #[must_use]
    pub fn new(id: u8, linked_timer: Option<NonZeroU8>, int: u8) -> Self {
        Self {
            id,
            int,
            backup: 0,
            control_a: 0,
            count: 0,
            control_b: 0,
            clock_ticks: None,
            tick_countdown: 0,
            linked: linked_timer,
            is_linked: false,
            count_enabled: false,
            reload_enabled: false,
        }
    }

    #[inline]
    #[must_use]
    pub fn linked_timer(&self) -> Option<NonZeroU8> {
        self.linked
    }

    #[inline]
    #[must_use]
    pub fn int(&self) -> u8 {
        self.int
    }

    #[allow(dead_code)]
    #[inline]
    #[must_use]
    pub fn id(&self) -> u8 {
        self.id
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        trace!("Timer #{} reset.", self.id);
        self.backup = 0;
        self.count = 0;
        self.control_a = 0;
        self.control_b = 0;
    }

    #[inline]
    #[must_use]
    pub fn backup(&self) -> u8 {
        self.backup
    }

    #[inline]
    #[must_use]
    pub fn control_a(&self) -> u8 {
        self.control_a
    }

    #[inline]
    #[must_use]
    pub fn count(&self) -> u8 {
        self.count
    }

    #[inline]
    #[must_use]
    pub fn control_b(&self) -> u8 {
        self.control_b
    }

    #[inline]
    pub fn reset_timer_done(&mut self) {
        self.control_b &= !CTRLB_TIMER_DONE_BIT;
    }

    pub fn set_control_a(&mut self, value: u8) {
        const TICKS_COUNT: [Option<u16>; 8] = [
            Some(TIMER_TICKS_COUNT * u16::pow(2, 0)), 
            Some(TIMER_TICKS_COUNT * u16::pow(2, 1)),
            Some(TIMER_TICKS_COUNT * u16::pow(2, 2)),
            Some(TIMER_TICKS_COUNT * u16::pow(2, 3)),
            Some(TIMER_TICKS_COUNT * u16::pow(2, 4)),  
            Some(TIMER_TICKS_COUNT * u16::pow(2, 5)),
            Some(TIMER_TICKS_COUNT * u16::pow(2, 6)),
            None, // 7 (linked timer)
        ];

        self.control_a = value;
        self.clock_ticks = TICKS_COUNT[self.period() as usize];
        if value & CTRLA_RESET_DONE_BIT != 0 {
            self.reset_timer_done();
        }

        self.is_linked = self.period() == 7;
        self.count_enabled = self.control_a & CTRLA_ENABLE_COUNT_BIT != 0;
        self.reload_enabled = value & CTRLA_ENABLE_RELOAD_BIT != 0;

        if !self.is_linked && self.count_enabled {
            self.tick_countdown = 1 + self.clock_ticks.unwrap();
            trace!(
                "Timer #{} next trigger @ {}",
                self.id,
                self.tick_countdown
            );
        } else {
            self.tick_countdown = 0;
        }

        trace!("Timer {self:?}");
    }

    pub fn set_control_a_transaprent(&mut self, value: u8) {
        self.control_a = value;
    }

    #[inline]
    pub fn set_control_b(&mut self, value: u8) {
        trace!("Timer #{} ctrl_b = {}.", self.id, value);
        if self.control_b & CTRLB_BORROW_IN_BIT == 0 && value & CTRLB_BORROW_IN_BIT != 0 && self.count > 0 {
            self.count -= 1;
        }
        self.control_b = value & 0xF8;
    }

    pub fn clear_borrows(&mut self) {
        self.control_b &= 0xFC;
    }

    pub fn set_control_b_flags(&mut self, flags: u8) {
        self.control_b |= flags;
    }

    #[inline]
    pub fn set_backup(&mut self, value: u8) {
        trace!("Timer #{} backup = {}.", self.id, value);
        self.backup = value;
    }

    #[inline]
    pub fn set_count(&mut self, value: u8) {
        trace!("Timer #{} count = {}.", self.id, value);
        self.count = value;
        if !self.is_linked && self.count_enabled && value != 0 {
            self.tick_countdown = 1 + self.clock_ticks.unwrap();
            trace!(
                "Timer #{} next trigger @ {}",
                self.id,
                self.tick_countdown
            );
        }
    }

    #[inline]
    pub fn set_count_transparent(&mut self, value: u8) {
        trace!("Timer #{} count = {}.", self.id, value);
        self.count = value;
    }

    #[inline]
    fn period(&self) -> u8 {
        self.control_a & CTRLA_PERIOD_BIT
    }

    #[inline]
    #[must_use]
    pub fn interrupt_enabled(&self) -> bool {
        self.control_a & CTRLA_INTERRUPT_BIT != 0
    }

    #[inline]
    #[must_use]
    pub fn is_linked(&self) -> bool {
        self.is_linked
    }

    #[inline]
    pub fn done(&mut self) -> u8 {
        self.set_control_b_flags(CTRLB_TIMER_DONE_BIT | CTRLB_BORROW_OUT_BIT);
        if self.interrupt_enabled() {
            return self.int();
        }
        0
    }

    #[inline]
    #[must_use]
    pub fn tick_countdown(&self) -> u16 {
        self.tick_countdown
    }

    #[inline]
    pub fn reset_tick_countdown(&mut self) {
        self.tick_countdown = 1 + self.clock_ticks.unwrap();
    }

    #[inline]
    pub fn disable_tick_countdown(&mut self) {
        self.tick_countdown = 0;
    }

    #[inline]
    #[must_use]
    pub fn reload_enabled(&self) -> bool {
        self.reload_enabled
    }

    #[inline]
    #[must_use]
    pub fn count_enabled(&self) -> bool {
        self.count_enabled
    }

    #[inline]
    #[must_use]
    pub fn clock_ticks(&self) -> Option<u16> {
        self.clock_ticks
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Timer #:{}, backup:{}, period:{}, int:{} reload:{}, count:{}, islinked:{}",
            self.id,
            self.backup,
            self.period(),
            self.interrupt_enabled(),
            self.reload_enabled,
            self.count_enabled,
            self.is_linked()
        )
    }
}
