pub mod audio_channel_timer;
pub mod base_timer;

use audio_channel_timer::AudioChannelTimer;
use base_timer::BaseTimer;
use log::trace;
use crate::CRYSTAL_TICK_LENGTH;
use crate::mikey::*;

const TIMER_TICKS_COUNT: u16 = (0.000001 / CRYSTAL_TICK_LENGTH) as u16; // 1us/62.5ns

const TIMER_LINKS: [Option<usize>; 12] = [Some(2), Some(3), Some(4), Some(5), None, Some(7), None, Some(8), Some(9), Some(10), Some(11), Some(1)];
const TIMER_COUNT: u8 = 12;

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

#[derive(Clone, Serialize, Deserialize)]
enum TimerType {
    Base(BaseTimer),
    Audio(AudioChannelTimer),
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
    timers: [TimerType; TIMER_COUNT as usize],
    timer_triggers: [(u64, usize); TIMER_COUNT as usize],
    ticks: u64,
}

macro_rules! tick_linked_timer {
    ($self: ident, $t: ident) => {{
        let (triggered, i) = $t.tick_linked();
        if !triggered {
            0
        } else {
            let mut int = i;
            match $t.linked_timer() {
                Some(id) => int |= $self.tick_linked_timer(id),
                _ => (),
            }
            int
        }
    }};
}

macro_rules! tick_timer {
    ($self: ident, $t: ident) => {{
        let (triggered, i) = $t.tick($self.ticks);
        if !triggered {
            0
        } else {
            let mut int = i;
            match $t.linked_timer() {
                Some(id) => int |= $self.tick_linked_timer(id),
                _ => (),
            }
            int
        }
    }};
}

impl Timers {
    pub fn new() -> Self {
        Self {
            timers: [
                TimerType::Base(BaseTimer::new(0, TIMER_LINKS[0])), 
                TimerType::Base(BaseTimer::new(1, TIMER_LINKS[1])), 
                TimerType::Base(BaseTimer::new(2, TIMER_LINKS[2])), 
                TimerType::Base(BaseTimer::new(3, TIMER_LINKS[3])), 
                TimerType::Base(BaseTimer::new(4, TIMER_LINKS[4])), 
                TimerType::Base(BaseTimer::new(5, TIMER_LINKS[5])), 
                TimerType::Base(BaseTimer::new(6, TIMER_LINKS[6])), 
                TimerType::Base(BaseTimer::new(7, TIMER_LINKS[7])),
                TimerType::Audio(AudioChannelTimer::new(8, TIMER_LINKS[8])), 
                TimerType::Audio(AudioChannelTimer::new(9, TIMER_LINKS[9])), 
                TimerType::Audio(AudioChannelTimer::new(10, TIMER_LINKS[10])), 
                TimerType::Audio(AudioChannelTimer::new(11, TIMER_LINKS[11])),
            ],
            timer_triggers: [
                (u64::MAX, 0),
                (u64::MAX, 1),
                (u64::MAX, 2),
                (u64::MAX, 3),
                (u64::MAX, 4),
                (u64::MAX, 5),
                (u64::MAX, 6),
                (u64::MAX, 7),
                (u64::MAX, 8),
                (u64::MAX, 9),
                (u64::MAX, 10),
                (u64::MAX, 11),
            ],
            ticks: 0,
        }
    }

    fn tick_linked_timer(&mut self, timer_id: usize) -> u8 {
        match &mut self.timers[timer_id] {
            TimerType::Base(t) => tick_linked_timer!(self, t),
            TimerType::Audio(t) => tick_linked_timer!(self, t),
        }
    }

    pub fn vsync(&mut self) -> bool {
        match &mut self.timers[2] {
            TimerType::Base(t) => {
                if t.triggered() {
                    t.reset_triggered();
                    return true;
                }
                false
            }
            _ => panic!()
        }
    }

    pub fn hsync(&mut self) -> bool {
        match &mut self.timers[0] {
            TimerType::Base(t) => {
                if t.triggered() {
                    t.reset_triggered();
                    return true;
                }
                false
            }
            _ => panic!()
        }
    }

    pub fn tick_all(&mut self, current_tick: u64) -> (u8, bool) { // bool: Timer 4 has a special treatment, triggered information without interrupt
        let mut int = 0;
        let mut triggered: bool = false;
        let mut int4_triggered: bool = false;

        self.ticks = current_tick;

        for tt in self.timer_triggers { 
            if tt.0 <= self.ticks {
                int |= match &mut self.timers[tt.1] {
                    TimerType::Base(t) => tick_timer!(self, t),
                    TimerType::Audio(t) => tick_timer!(self, t),
                };  
                self.update_timer_trigger_tick(tt.1);
                if tt.1 == 4 {
                    int4_triggered = true;
                }
                triggered = true;
            } else {
                break;
            }
        };

        if triggered {
            self.sort_timers();
        }

        (int, int4_triggered)
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
                    _ => panic!()
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
                    _ => panic!()
                }
            )
        }
    }

    pub fn timer4_interrupt_enabled(&self) -> bool {
        self.peek(TIM4CTLA) & CTRLA_INTERRUPT_BIT != 0
    }

    pub fn peek(&self, addr: u16) -> u8 {
        let (index, cmd) = self.get_timer(addr);
        match &self.timers[index] {
            TimerType::Base(t) => match cmd {
                TimerReg::Backup => t.backup(),
                TimerReg::ControlA => t.control_a(),
                TimerReg::Count => t.count(),
                TimerReg::ControlB => t.control_b(),
                _ => panic!(),
            },
            TimerType::Audio(t) => match cmd {
                TimerReg::Backup => t.backup(),
                TimerReg::ControlA => t.control_a(),
                TimerReg::Count => t.count(),
                TimerReg::ControlB => t.control_b(),
                TimerReg::Volume => t.volume(),
                TimerReg::Feedback => t.feedback(),
                TimerReg::Output => t.output() as u8,
                TimerReg::ShiftRegister => t.shift_register(),
            },
        }
    }

    pub fn poke(&mut self, addr: u16, v: u8) {
        trace!("poke 0x{:04x} -> 0x{:02x}", addr, v);
        let (index, cmd) = self.get_timer(addr);
        match &mut self.timers[index] {
            TimerType::Base(t) => match cmd {
                TimerReg::Backup => t.set_backup(v),
                TimerReg::ControlA => {
                    t.set_control_a(v, self.ticks);
                    self.update_timer_trigger_tick(index);
                    self.sort_timers();
                },
                TimerReg::Count => {
                    t.set_count(v, self.ticks);
                    self.update_timer_trigger_tick(index);
                    self.sort_timers();
                },
                TimerReg::ControlB => t.set_control_b(v),
                _ => panic!(), 
            },
            TimerType::Audio(t) => match cmd {
                TimerReg::Backup => t.set_backup(v),
                TimerReg::ControlA => {
                    t.set_control_a(v, self.ticks);
                    self.update_timer_trigger_tick(index);
                    self.sort_timers();
                },
                TimerReg::Count => {
                    t.set_count(v, self.ticks);
                    self.update_timer_trigger_tick(index);
                    self.sort_timers();
                },
                TimerReg::ControlB => t.set_control_b(v),
                TimerReg::Volume => t.set_volume(v),
                TimerReg::Feedback => t.set_feedback(v),
                TimerReg::Output => t.set_output(v as i8),
                TimerReg::ShiftRegister => t.set_shift_register(v), 
            },
        }
    }

    pub fn timer_trigger(&self, id: usize) -> u64 {
        match &self.timers[id] {
            TimerType::Base(t) => t.next_trigger_tick(),
            TimerType::Audio(t) => t.next_trigger_tick(),
        }
    }

    fn update_timer_trigger_tick(&mut self, id: usize) {
        let loc = self.timer_triggers.iter().position(|kv| kv.1 == id).unwrap();
        let tick = match &self.timers[id] {
            TimerType::Base(t) => t.next_trigger_tick(),
            TimerType::Audio(t) => t.next_trigger_tick(),
        };
        self.timer_triggers[loc].0 = tick;
    }

    fn sort_timers(&mut self) {
        self.timer_triggers.sort_by_key(|ti| ti.0);
    }

    pub fn audio_out(&self, n: usize) -> i16 {
        match &self.timers[n] {
            TimerType::Audio(t) => t.output() as i16,
            _ => 0
        }
    }
}

impl Default for Timers {
    fn default() -> Self {
        Timers::new()
    }
}
