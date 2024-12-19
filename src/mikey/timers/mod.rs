pub mod audio_timer_registers;
pub mod timer;

use core::num::{NonZero, NonZeroU8};
use audio_timer_registers::AudioTimerRegisters;
use timer::Timer;
use log::trace;
use crate::mikey::*;

const TIMER_TICKS_COUNT: u16 = (0.000001 / CRYSTAL_TICK_LENGTH) as u16; // 1us/62.5ns

const TIMER_LINKS: [Option<NonZeroU8>; 12] = [Some(NonZero::new(2).unwrap()), Some(NonZero::new(3).unwrap()), Some(NonZero::new(4).unwrap()), Some(NonZero::new(5).unwrap()), None, Some(NonZero::new(7).unwrap()), None, Some(NonZero::new(8).unwrap()), Some(NonZero::new(9).unwrap()), Some(NonZero::new(10).unwrap()), Some(NonZero::new(11).unwrap()), Some(NonZero::new(1).unwrap())];
const TIMER_COUNT: usize = 8;
const AUDIO_TIMER_COUNT: usize = 4;

const CTRLA_INTERRUPT_BIT: u8 = 0b10000000;
const CTRLA_RESET_DONE_BIT: u8 = 0b01000000;
#[allow(dead_code)]
const CTRLA_MAGMODE_BIT: u8 = 0b00100000;
const CTRLA_ENABLE_RELOAD_BIT: u8 = 0b00010000;
const CTRLA_ENABLE_COUNT_BIT: u8 = 0b00001000;
const CTRLA_PERIOD_BIT: u8 = 0b00000111;
const CTRLB_TIMER_DONE_BIT: u8 = 0b00001000;
#[allow(dead_code)]
const CTRLB_LAST_CLOCK_BIT: u8 = 0b00000100;
const CTRLB_BORROW_IN_BIT: u8 = 0b00000010;
const CTRLB_BORROW_OUT_BIT: u8 = 0b00000001;

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

#[derive(Serialize, Deserialize)]
pub struct Timers {
    timers: [Timer; TIMER_COUNT + AUDIO_TIMER_COUNT],
    audio_timer_regs: [AudioTimerRegisters; AUDIO_TIMER_COUNT],
    timer_triggers: [u64; TIMER_COUNT + AUDIO_TIMER_COUNT],
    timers_triggered: [bool; TIMER_COUNT + AUDIO_TIMER_COUNT],
    ticks: u64,
}

impl Timers {
    pub fn new() -> Self {
       
        Self {
            timers: [
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
            timer_triggers: [u64::MAX; TIMER_COUNT + AUDIO_TIMER_COUNT],
            ticks: 0,
            audio_timer_regs: [AudioTimerRegisters::new(); AUDIO_TIMER_COUNT],
            timers_triggered: [false; TIMER_COUNT + AUDIO_TIMER_COUNT],
        }
    }

    #[inline]
    pub fn vsync(&mut self) -> bool {
        if self.timers_triggered[2] {
            self.timers_triggered[2] = false;
            return true;
        }
        false
    }

    #[inline]
    pub fn hsync(&mut self) -> Option<u8> {
        if self.timers_triggered[0] {
            self.timers_triggered[0] = false;
            return Some(self.timers[2].count());
        }
        None
    }

    pub fn tick_all(&mut self, current_tick: u64) -> (u8, bool) { // bool: Timer 4 has a special treatment, triggered information without interrupt
        let mut int = 0;
        let mut int4_triggered: bool = false;

        self.ticks = current_tick;

        for id in 0..TIMER_COUNT+AUDIO_TIMER_COUNT {
            if self.timer_triggers[id] > self.ticks {
                continue;
            }
            int |= Self::tick_timer(&mut self.timers, &mut self.audio_timer_regs, &mut self.timers_triggered, id, current_tick);
            self.update_timer_trigger_tick(id);
            if id == 4 {
                int4_triggered = true;
            }
        }

        (int, int4_triggered)
    }


    pub fn tick_timer(timers: &mut [Timer], audio_regs: &mut [AudioTimerRegisters], triggereds: &mut [bool], id: usize, current_tick: u64) -> u8 {
        let timer = &mut timers[id];

        if !timer.is_linked() {
            timer.set_control_b(timer.control_b() & !CTRLB_BORROW_IN_BIT);        
            if !timer.count_enabled() || (is_audio!(id) && audio_regs[id - TIMER_COUNT].disabled()) { 
                timer.disable_trigger_tick();
                triggereds[id] = false;
                return 0;
            }    
            timer.set_next_trigger_tick(current_tick);
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
                int |= Self::tick_timer(timers, audio_regs, triggereds, linked_id, current_tick);
            }
        }

        int
    }

    pub fn timer_count_down(timer: &mut Timer, audio: Option<&mut AudioTimerRegisters>) -> (bool, u8) {
        timer.set_control_b((timer.control_b() & !CTRLB_BORROW_OUT_BIT) | CTRLB_BORROW_IN_BIT);

        if timer.count() == 0 {
            if timer.reload_enabled() {
                trace!("Timer #{} reload 0x{:02x} next trigger @ {}.", timer.id(), timer.backup(), timer.next_trigger_tick());
                timer.set_count_transparent(timer.backup());
            } else {
                timer.disable_trigger_tick();
            }
            return (
                true,
                if let Some(aud) = audio {
                    aud.done(timer)                        
                } else {
                    timer.done()
                });
        } else {
            timer.set_count_transparent(timer.count() - 1)
        }
        (false, 0)
    }

    fn get_timer(&self, addr: u16) -> (usize, TimerReg) {
        if addr < AUD0VOL { 
            (
                ((addr - MIK_ADDR) / 4) as usize, 
                match addr % 4 { 
                    0 => TimerReg::Backup,
                    1 => TimerReg::ControlA,
                    2 => TimerReg::Count,
                    3 => TimerReg::ControlB,
                    _ => unreachable!()
                }
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
                    _ => unreachable!()
                }
            )
        }
    }

    #[inline(always)]
    pub fn timer4_interrupt_enabled(&self) -> bool {
        self.peek(TIM4CTLA) & CTRLA_INTERRUPT_BIT != 0
    }

    pub fn peek(&self, addr: u16) -> u8 {
        let (index, cmd) = self.get_timer(addr);
        match cmd {
            TimerReg::Backup => self.timers[index].backup(),
            TimerReg::ControlA => self.timers[index].control_a(),
            TimerReg::Count => self.timers[index].count(),
            TimerReg::ControlB => self.timers[index].control_b(),
            TimerReg::Volume => self.audio_timer_regs[index - TIMER_COUNT].volume(),
            TimerReg::Feedback => self.audio_timer_regs[index - TIMER_COUNT].feedback(),
            TimerReg::Output => self.audio_timer_regs[index - TIMER_COUNT].output() as u8,
            TimerReg::ShiftRegister => self.audio_timer_regs[index - TIMER_COUNT].shift_register(),
        }
    }

    pub fn poke(&mut self, addr: u16, v: u8) {
        trace!("poke 0x{:04x} -> 0x{:02x}", addr, v);
        let (index, cmd) = self.get_timer(addr);
        match cmd {
            TimerReg::Backup => {
                self.timers[index].set_backup(v);
                if is_audio!(index) {
                    self.audio_timer_regs[index - TIMER_COUNT].update_disabled(v)
                };
            },
            TimerReg::ControlA => {
                self.timers[index].set_control_a(v, self.ticks);
                self.update_timer_trigger_tick(index);
            },
            TimerReg::Count => {
                self.timers[index].set_count(v, self.ticks);
                self.update_timer_trigger_tick(index);
            },
            TimerReg::ControlB => self.timers[index].set_control_b(v),
            TimerReg::Volume => self.audio_timer_regs[index - TIMER_COUNT].set_volume(v),
            TimerReg::Feedback => self.audio_timer_regs[index - TIMER_COUNT].set_feedback(self.timers[index].backup(), v),
            TimerReg::Output => self.audio_timer_regs[index - TIMER_COUNT].set_output(v as i8),
            TimerReg::ShiftRegister => self.audio_timer_regs[index - TIMER_COUNT].set_shift_register(v), 
        }
    }

    #[inline(always)]
    pub fn timer_trigger(&self, id: usize) -> u64 {
        self.timers[id].next_trigger_tick()
    }

    #[inline(always)]
    fn update_timer_trigger_tick(&mut self, id: usize) {        
        self.timer_triggers[id] = self.timer_trigger(id);
    }

    #[inline(always)]
    pub fn audio_out(&self, n: usize) -> i16 {
        self.audio_timer_regs[n].output() as i16
    }    
}

impl Default for Timers {
    fn default() -> Self {
        Timers::new()
    }
}
