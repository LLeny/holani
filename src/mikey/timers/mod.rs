pub mod audio_timer_registers;
pub mod timer;

use crate::mikey::{
    alloc, Deserialize, Serialize, AUD0VOL, CRYSTAL_TICK_LENGTH, MIK_ADDR, TIM4CTLA,
};
use audio_timer_registers::AudioTimerRegisters;
use core::num::{NonZero, NonZeroU8};
use log::trace;
use timer::Timer;

const TIMER_TICKS_COUNT: u16 = (0.000_001 / CRYSTAL_TICK_LENGTH) as u16; // 1us/62.5ns

const TIMER_LINKS: [Option<NonZeroU8>; 12] = [
    Some(NonZero::new(2).unwrap()),
    Some(NonZero::new(3).unwrap()),
    Some(NonZero::new(4).unwrap()),
    Some(NonZero::new(5).unwrap()),
    None,
    Some(NonZero::new(7).unwrap()),
    None,
    Some(NonZero::new(8).unwrap()),
    Some(NonZero::new(9).unwrap()),
    Some(NonZero::new(10).unwrap()),
    Some(NonZero::new(11).unwrap()),
    Some(NonZero::new(1).unwrap()),
];

const TIMER_COUNT: usize = 8;
const AUDIO_TIMER_COUNT: usize = 4;

const CTRLA_INTERRUPT_BIT: u8 = 0b1000_0000;
const CTRLA_RESET_DONE_BIT: u8 = 0b0100_0000;
#[allow(dead_code)]
const CTRLA_MAGMODE_BIT: u8 = 0b0010_0000;
const CTRLA_ENABLE_RELOAD_BIT: u8 = 0b0001_0000;
const CTRLA_ENABLE_COUNT_BIT: u8 = 0b0000_1000;
const CTRLA_PERIOD_BIT: u8 = 0b0000_0111;
const CTRLB_TIMER_DONE_BIT: u8 = 0b0000_1000;
#[allow(dead_code)]
const CTRLB_LAST_CLOCK_BIT: u8 = 0b0000_0100;
const CTRLB_BORROW_IN_BIT: u8 = 0b0000_0010;
const CTRLB_BORROW_OUT_BIT: u8 = 0b0000_0001;

macro_rules! is_audio {
    ($index: expr) => {
        ($index as usize) >= TIMER_COUNT
    };
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TimerReg {
    Backup = 0,
    ControlA,
    Count,
    ControlB,
    Volume,
    Feedback,
    Output,
    ShiftRegister,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timers {
    timer: [Timer; TIMER_COUNT + AUDIO_TIMER_COUNT],
    audio_reg: [AudioTimerRegisters; AUDIO_TIMER_COUNT],
    countdown: [u16; 16], //Round up to 256 bits for SIMD
    triggered: [bool; TIMER_COUNT + AUDIO_TIMER_COUNT],
}

impl Timers {
    #[must_use]
    pub fn new() -> Self {
        Self {
            timer: [
                Timer::new(0, TIMER_LINKS[0], 1),
                Timer::new(1, TIMER_LINKS[1], 1 << 1),
                Timer::new(2, TIMER_LINKS[2], 1 << 2),
                Timer::new(3, TIMER_LINKS[3], 1 << 3),
                Timer::new(4, TIMER_LINKS[4], 1 << 4),
                Timer::new(5, TIMER_LINKS[5], 1 << 5),
                Timer::new(6, TIMER_LINKS[6], 1 << 6),
                Timer::new(7, TIMER_LINKS[7], 1 << 7),
                Timer::new(8, TIMER_LINKS[8], 0),
                Timer::new(9, TIMER_LINKS[9], 0),
                Timer::new(10, TIMER_LINKS[10], 0),
                Timer::new(11, TIMER_LINKS[11], 0),
            ],
            countdown: [0; 16],
            audio_reg: [AudioTimerRegisters::new(); AUDIO_TIMER_COUNT],
            triggered: [false; TIMER_COUNT + AUDIO_TIMER_COUNT],
        }
    }

    #[inline]
    pub fn vsync(&mut self) -> bool {
        if self.triggered[2] {
            self.triggered[2] = false;
            return true;
        }
        false
    }

    #[inline]
    pub fn hsync(&mut self) -> Option<u8> {
        if self.triggered[0] {
            self.triggered[0] = false;
            return Some(self.timer[2].count());
        }
        None
    }

    #[inline]
    #[allow(unreachable_code)]
    pub fn check_if_triggered(&mut self, countdown_triggered: &mut [u16; 16]) {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        unsafe {
            use core::arch::x86_64::{
                _mm256_loadu_si256, _mm256_set1_epi16, _mm256_storeu_si256, _mm256_cmpeq_epi16,_mm256_subs_epi16
            };

            let countdowns = _mm256_loadu_si256(self.countdown.as_ptr() as *const _);
            let ones = _mm256_set1_epi16(1);
            let eq1 = _mm256_cmpeq_epi16(countdowns, ones);
            let dec = _mm256_subs_epi16(countdowns, ones);

            _mm256_storeu_si256(countdown_triggered.as_mut_ptr() as *mut _, eq1);
            _mm256_storeu_si256(self.countdown.as_mut_ptr() as *mut _, dec);

            return;
        }
        for (countdown, triggered) in self.countdown[0..TIMER_COUNT + AUDIO_TIMER_COUNT]
            .iter_mut()
            .zip(countdown_triggered[0..TIMER_COUNT + AUDIO_TIMER_COUNT].iter_mut())
        {
            *triggered = u16::from(*countdown == 1);
            *countdown = (*countdown).saturating_sub(1);
        }
    }

    pub fn tick_all(&mut self) -> (u8, bool) {
        // bool: Timer 4 has a special treatment, triggered information without interrupt
        let mut int: u8 = 0;
        let mut countdown_triggered: [u16; 16] = [0; 16];

        self.check_if_triggered(&mut countdown_triggered);

        countdown_triggered[0..TIMER_COUNT + AUDIO_TIMER_COUNT]
            .iter()
            .enumerate()
            .filter(|(_, &x)| x != 0)
            .for_each(|(id, _)| {
                int |= Self::tick_timer(
                    &mut self.timer,
                    &mut self.audio_reg,
                    &mut self.triggered,
                    id,
                );
                self.update_timer_countdown(id);
            });

        (int, countdown_triggered[4] != 0)
    }

    pub fn tick_timer(
        timers: &mut [Timer],
        audio_regs: &mut [AudioTimerRegisters],
        triggereds: &mut [bool],
        id: usize,
    ) -> u8 {
        let timer = &mut timers[id];

        if !timer.is_linked() {
            timer.set_control_b(timer.control_b() & !CTRLB_BORROW_IN_BIT);
            if !timer.count_enabled() || (is_audio!(id) && audio_regs[id - TIMER_COUNT].disabled())
            {
                timer.disable_tick_countdown();
                triggereds[id] = false;
                return 0;
            }
            timer.reset_tick_countdown();
        }

        let mut int: u8;
        let audio = if is_audio!(id) {
            Some(&mut audio_regs[id - TIMER_COUNT])
        } else {
            None
        };
        (triggereds[id], int) = Self::timer_count_down(timer, audio);

        if !triggereds[id] {
            return 0;
        }

        if let Some(lid) = timer.linked_timer() {
            let linked_id = lid.get() as usize;
            if timers[linked_id].is_linked() {
                int |= Self::tick_timer(timers, audio_regs, triggereds, linked_id);
            }
        }

        int
    }

    pub fn timer_count_down(
        timer: &mut Timer,
        audio: Option<&mut AudioTimerRegisters>,
    ) -> (bool, u8) {
        timer.set_control_b((timer.control_b() & !CTRLB_BORROW_OUT_BIT) | CTRLB_BORROW_IN_BIT);
        let count = timer.count();

        if count == 0 {
            if timer.reload_enabled() {
                trace!(
                    "Timer #{} reload 0x{:02x} next trigger @ {}.",
                    timer.id(),
                    timer.backup(),
                    timer.tick_countdown()
                );
                timer.set_count_transparent(timer.backup());
            } else {
                timer.disable_tick_countdown();
            }
            return (
                true,
                if let Some(aud) = audio {
                    aud.done(timer)
                } else {
                    timer.done()
                },
            );
        }
        timer.set_count_transparent(count - 1);
        (false, 0)
    }

    #[inline]
    #[must_use]
    pub fn timer4_interrupt_enabled(&self) -> bool {
        self.peek(TIM4CTLA) & CTRLA_INTERRUPT_BIT != 0
    }

    #[must_use]
    pub fn peek(&self, addr: u16) -> u8 {
        let (index, cmd) = get_timer(addr);
        match cmd {
            TimerReg::Backup => self.timer[index].backup(),
            TimerReg::ControlA => self.timer[index].control_a(),
            TimerReg::Count => self.timer[index].count(),
            TimerReg::ControlB => self.timer[index].control_b(),
            TimerReg::Volume => self.audio_reg[index - TIMER_COUNT].volume(),
            TimerReg::Feedback => self.audio_reg[index - TIMER_COUNT].feedback(),
            TimerReg::Output => self.audio_reg[index - TIMER_COUNT].output() as u8,
            TimerReg::ShiftRegister => self.audio_reg[index - TIMER_COUNT].shift_register(),
        }
    }

    pub fn poke(&mut self, addr: u16, v: u8) {
        trace!("poke 0x{addr:04x} -> 0x{v:02x}");
        let (index, cmd) = get_timer(addr);
        match cmd {
            TimerReg::Backup => {
                self.timer[index].set_backup(v);
                if is_audio!(index) {
                    self.audio_reg[index - TIMER_COUNT].update_disabled(v);
                }
            }
            TimerReg::ControlA => {
                self.timer[index].set_control_a(v);
                self.update_timer_countdown(index);
            }
            TimerReg::Count => {
                self.timer[index].set_count(v);
                self.update_timer_countdown(index);
            }
            TimerReg::ControlB => self.timer[index].set_control_b(v),
            TimerReg::Volume => self.audio_reg[index - TIMER_COUNT].set_volume(v),
            TimerReg::Feedback => {
                self.audio_reg[index - TIMER_COUNT].set_feedback(self.timer[index].backup(), v);
            }
            TimerReg::Output => self.audio_reg[index - TIMER_COUNT].set_output(v as i8),
            TimerReg::ShiftRegister => self.audio_reg[index - TIMER_COUNT].set_shift_register(v),
        }
    }

    #[inline]
    #[must_use]
    pub fn timer_countdown(&self, id: usize) -> u16 {
        self.timer[id].tick_countdown()
    }

    #[inline]
    fn update_timer_countdown(&mut self, id: usize) {
        self.countdown[id] = self.timer_countdown(id);
    }

    #[inline]
    #[must_use]
    pub fn audio_out(&self, n: usize) -> i16 {
        i16::from(self.audio_reg[n].output())
    }

    #[inline]
    #[must_use]
    pub fn timer(&self, id: usize) -> &Timer {
        &self.timer[id]
    }

    #[inline]
    #[must_use]
    pub fn audio_timer(&self, id: usize) -> &AudioTimerRegisters {
        &self.audio_reg[id]
    }
}

fn get_timer(addr: u16) -> (usize, TimerReg) {
    if addr < AUD0VOL {
        (
            ((addr - MIK_ADDR) / 4) as usize,
            match addr % 4 {
                0 => TimerReg::Backup,
                1 => TimerReg::ControlA,
                2 => TimerReg::Count,
                3 => TimerReg::ControlB,
                _ => unreachable!(),
            },
        )
    } else {
        (
            (((addr - AUD0VOL) / 8) + 8) as usize,
            match addr % 8 {
                0 => TimerReg::Volume,
                1 => TimerReg::Feedback,
                2 => TimerReg::Output,
                3 => TimerReg::ShiftRegister,
                4 => TimerReg::Backup,
                5 => TimerReg::ControlA,
                6 => TimerReg::Count,
                7 => TimerReg::ControlB,
                _ => unreachable!(),
            },
        )
    }
}

impl Default for Timers {
    fn default() -> Self {
        Timers::new()
    }
}
